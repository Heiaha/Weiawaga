use super::board::*;
use super::color::*;
use super::moov::*;
use super::types::*;
use regex::{Match, Regex};
use std::sync;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

// Some ideas taken from asymptote, which has a very elegant timer implementation.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TimeControl {
    Infinite,
    FixedDuration(Duration),
    FixedDepth(Depth),
    FixedNodes(u64),
    Variable {
        wtime: Duration,
        btime: Duration,
        winc: Option<Duration>,
        binc: Option<Duration>,
        moves_to_go: Option<u32>,
    },
}

impl TimeControl {
    fn parse_duration(m: Option<Match>) -> Result<Option<Duration>, &'static str> {
        m.map(|m| {
            m.as_str()
                .parse::<u64>()
                .map_err(|_| "Unable to parse wtime.")
                .map(Duration::from_millis)
        })
        .transpose()
    }
}

impl TryFrom<&str> for TimeControl {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if line == "go" {
            return Ok(TimeControl::Infinite);
        }

        let re_captures = GO_RE.captures(line).ok_or("Invalid go format.")?;

        if re_captures.name("ponder").is_some() {
            return Err("Ponder is not implemented.");
        }

        if re_captures.name("searchmoves").is_some() {
            return Err("Searchmoves is not implemented.");
        }

        if re_captures.name("mate").is_some() {
            return Err("Mate is not implemented.");
        }

        let mut count = 0;
        let mut result = Err("Unable to parse go parameters.");

        if let Some(m) = re_captures.name("nodes") {
            count += 1;
            result = m
                .as_str()
                .parse::<u64>()
                .map_err(|_| "Unable to parse nodes.")
                .map(Self::FixedNodes);
        }

        if let Some(m) = re_captures.name("depth") {
            count += 1;
            result = m
                .as_str()
                .parse::<Depth>()
                .map_err(|_| "Unable to parse depth.")
                .map(Self::FixedDepth);
        }

        if let Some(movetime) = Self::parse_duration(re_captures.name("movetime"))? {
            count += 1;
            result = Ok(Self::FixedDuration(movetime));
        }

        let wtime = Self::parse_duration(re_captures.name("wtime"))?;
        let btime = Self::parse_duration(re_captures.name("btime"))?;
        let winc = Self::parse_duration(re_captures.name("winc"))?;
        let binc = Self::parse_duration(re_captures.name("binc"))?;

        if wtime.is_some() ^ btime.is_some() {
            return Err("Must provide both wtime and btime.");
        }

        let moves_to_go = re_captures
            .name("moves_to_go")
            .map(|m| {
                m.as_str()
                    .parse::<u32>()
                    .map_err(|_| "Unable to parse moves_to_go.")
            })
            .transpose()?;

        if let (Some(wtime), Some(btime)) = (wtime, btime) {
            count += 1;
            result = Ok(Self::Variable {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
            });
        }

        if count > 1 {
            return Err(
                "Only one of depth, nodes, movetime, or time control parameters is allowed.",
            );
        }

        result
    }
}

static GO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
                ^go
                (?:
                    \s+depth\s+(?P<depth>\d+) |
                    \s+nodes\s+(?P<nodes>\d+) |
                    \s+movetime\s+(?P<movetime>\d+) |
                    \s+wtime\s+(?P<wtime>\d+) |
                    \s+btime\s+(?P<btime>\d+) |
                    \s+winc\s+(?P<winc>\d+) |
                    \s+binc\s+(?P<binc>\d+) |
                    \s+mate\s+(?P<mate>\d+) |
                    \s+movestogo\s+(?P<movestogo>\d+)
                )*
                $",
    )
    .expect("Go regex should be valid.")
});

#[derive(Clone)]
pub struct Timer {
    control: TimeControl,
    start_time: Instant,
    stop: Arc<AtomicBool>,
    times_checked: u64,
    time_target: Duration,
    time_maximum: Duration,
    overhead: Duration,
    last_best_move: Option<Move>,
}

impl Timer {
    pub fn new(
        board: &Board,
        control: TimeControl,
        stop: Arc<AtomicBool>,
        overhead: Duration,
    ) -> Self {
        let mut time_target = Duration::ZERO;
        let mut time_maximum = Duration::ZERO;

        if let TimeControl::Variable {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
        } = control
        {
            let (time, inc) = match board.ctm() {
                Color::White => (wtime, winc),
                Color::Black => (btime, binc),
            };

            let mtg = moves_to_go.unwrap_or_else(|| {
                (Self::MTG_INTERCEPT
                    + Self::MTG_EVAL_WEIGHT * (board.simple_eval().abs() as f32)
                    + Self::MTG_MOVE_WEIGHT * (board.fullmove_number() as f32))
                    .ceil()
                    .max(1.0) as u32
            });

            time_target = time.min(time / mtg + inc.unwrap_or(Duration::ZERO));
            time_maximum = time_target + (time - time_target) / 4;
        }

        Self {
            start_time: Instant::now(),
            stop,
            control,
            overhead,
            time_target,
            time_maximum,
            last_best_move: None,
            times_checked: 0,
        }
    }

    pub fn start_check(&self, depth: Depth) -> bool {
        if self.stop.load(sync::atomic::Ordering::Relaxed) {
            return false;
        }

        // Always search to a depth of at least 1
        if depth <= 1 {
            return true;
        }

        let start = match self.control {
            TimeControl::Infinite => true,
            TimeControl::FixedDuration(duration) => self.elapsed() + self.overhead <= duration,
            TimeControl::FixedDepth(stop_depth) => depth <= stop_depth,
            TimeControl::FixedNodes(_) => true,
            TimeControl::Variable { .. } => self.elapsed() + self.overhead <= self.time_target / 2,
        };

        if !start {
            self.stop.store(true, sync::atomic::Ordering::Relaxed);
        }
        start
    }

    pub fn stop_check(&mut self) -> bool {
        self.times_checked += 1;

        let should_check = self.times_checked & Self::CHECK_FLAG == 0;

        if should_check && self.stop.load(sync::atomic::Ordering::Relaxed) {
            return true;
        }

        let stop = match self.control {
            TimeControl::Infinite => false,
            TimeControl::FixedDuration(duration) => {
                if should_check {
                    self.elapsed() + self.overhead >= duration
                } else {
                    false
                }
            }
            TimeControl::Variable { .. } => {
                if should_check {
                    self.elapsed() + self.overhead >= self.time_maximum
                } else {
                    false
                }
            }
            TimeControl::FixedDepth(_) => false,
            TimeControl::FixedNodes(nodes) => self.times_checked >= nodes,
        };

        if stop {
            self.stop.store(true, sync::atomic::Ordering::Relaxed);
        }
        stop
    }

    pub fn stop(&mut self) {
        self.stop.store(true, sync::atomic::Ordering::SeqCst);
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn update(&mut self, best_move: Option<Move>) {
        if self
            .last_best_move
            .is_some_and(|last_move| Some(last_move) != best_move)
        {
            self.time_target = self.time_maximum.min(self.time_target * 3 / 2);
        }

        self.last_best_move = best_move;
    }
}

impl Timer {
    const CHECK_FLAG: u64 = 0x1000 - 1;
    const MTG_INTERCEPT: f32 = 52.52;
    const MTG_EVAL_WEIGHT: f32 = -0.01833;
    const MTG_MOVE_WEIGHT: f32 = -0.4657;
}
