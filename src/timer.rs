use super::board::*;
use super::moov::*;
use super::piece::*;
use regex::{Match, Regex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

// Some ideas taken from asymptote, which has a very elegant timer implementation.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TimeControl {
    Infinite,
    FixedDuration(Duration),
    FixedDepth(i8),
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
                .parse::<i64>()
                .map_err(|_| "Unable to parse time.")
                .map(|x| Duration::from_millis(x.max(0) as u64))
        })
        .transpose()
    }
}

impl TryFrom<&str> for TimeControl {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if line == "go" || line == "go ponder" {
            return Ok(TimeControl::Infinite);
        }

        let re_captures = GO_RE.captures(line).ok_or("Invalid go format.")?;

        if re_captures.name("searchmoves").is_some() || re_captures.name("mate").is_some() {
            return Err("Feature is not implemented.");
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
                .parse::<i8>()
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
            .name("movestogo")
            .map(|m| {
                m.as_str()
                    .parse::<u32>()
                    .map_err(|_| "Unable to parse movestogo.")
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
        r"(?x)^
                go
                (?:
                    \s+depth\s+(?P<depth>\d+) |
                    \s+nodes\s+(?P<nodes>\d+) |
                    \s+movetime\s+(?P<movetime>\d+) |
                    \s+wtime\s+(?P<wtime>-?\d+) |
                    \s+btime\s+(?P<btime>-?\d+) |
                    \s+winc\s+(?P<winc>\d+) |
                    \s+binc\s+(?P<binc>\d+) |
                    \s+mate\s+(?P<mate>\d+) |
                    \s+movestogo\s+(?P<movestogo>\d+) |
                    \s+ponder
                )*
            $",
    )
    .expect("Go regex should be valid.")
});

#[derive(Clone)]
pub struct Timer {
    control: TimeControl,
    start_time: Instant,
    pondering: Arc<AtomicBool>,
    global_stop: Arc<AtomicBool>,
    local_stop: bool,
    nodes: Arc<AtomicU64>,
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
        pondering: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
        nodes: Arc<AtomicU64>,
        overhead: Duration,
    ) -> Self {
        let (time_target, time_maximum) = if let TimeControl::Variable { .. } = control {
            Self::calculate_time(board, control)
        } else {
            (Duration::ZERO, Duration::ZERO)
        };

        Self {
            start_time: Instant::now(),
            pondering,
            local_stop: false,
            global_stop: stop,
            nodes,
            control,
            overhead,
            time_target,
            time_maximum,
            last_best_move: None,
            times_checked: 0,
        }
    }

    fn calculate_time(board: &Board, control: TimeControl) -> (Duration, Duration) {
        let TimeControl::Variable {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
        } = control
        else {
            unreachable!()
        };

        let (time, inc) = match board.ctm() {
            Color::White => (wtime, winc),
            Color::Black => (btime, binc),
        };

        let mtg = moves_to_go.unwrap_or(40);

        let time_target = time.min(time / mtg + inc.unwrap_or(Duration::ZERO));
        let time_maximum = time_target + (time - time_target) / 4;

        (time_target, time_maximum)
    }

    pub fn start_check(&mut self, depth: i8) -> bool {
        if self.local_stop {
            return false;
        }

        if self.global_stop.load(Ordering::Acquire) {
            self.nodes.fetch_add(self.times_checked, Ordering::Relaxed);
            return false;
        }

        // Always search to a depth of at least 1
        if depth <= 1 {
            return true;
        }

        if self.pondering.load(Ordering::Acquire) {
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
            self.stop();
        }
        start
    }

    pub fn stop_check(&mut self) -> bool {
        if self.local_stop {
            return true;
        }

        self.times_checked += 1;

        if self.times_checked < Self::CHECK_FREQ {
            return false;
        }

        let nodes = self.nodes.fetch_add(self.times_checked, Ordering::Relaxed);
        self.times_checked = 0;

        self.local_stop = self.global_stop.load(Ordering::Acquire);
        if self.local_stop {
            return true;
        }

        if self.pondering.load(Ordering::Acquire) {
            return false;
        }

        let stop = match self.control {
            TimeControl::Infinite => false,
            TimeControl::FixedDuration(duration) => self.elapsed() + self.overhead >= duration,
            TimeControl::Variable { .. } => self.elapsed() + self.overhead >= self.time_maximum,
            TimeControl::FixedDepth(_) => false,
            TimeControl::FixedNodes(stop_nodes) => nodes >= stop_nodes,
        };

        if stop {
            self.nodes.fetch_add(self.times_checked, Ordering::Relaxed);
            self.stop();
        }

        stop
    }

    pub fn stop(&mut self) {
        self.local_stop = true;
        self.global_stop.store(true, Ordering::Release);
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn nodes(&self) -> u64 {
        self.nodes.load(Ordering::Relaxed) + self.times_checked
    }

    pub fn local_stop(&self) -> bool {
        self.local_stop
    }

    pub fn pondering(&self) -> bool {
        self.pondering.load(Ordering::Acquire)
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
    const CHECK_FREQ: u64 = 4096;
}
