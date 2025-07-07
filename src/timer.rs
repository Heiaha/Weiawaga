use super::board::*;
use super::moov::*;
use super::piece::*;
use super::square::*;
use super::types::*;
use regex::{Captures, Regex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use std::{convert::TryFrom, str::FromStr};

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
    fn opt_number<T: FromStr>(
        caps: &Captures,
        name: &'static str,
        err: &'static str,
    ) -> Result<Option<T>, &'static str> {
        caps.name(name)
            .map(|m| m.as_str().parse::<T>().map_err(|_| err))
            .transpose()
    }

    fn opt_duration(caps: &Captures, name: &'static str) -> Result<Option<Duration>, &'static str> {
        Self::opt_number::<i64>(caps, name, "Unable to parse time.")?
            .map(|ms| Ok(Duration::from_millis(ms.max(0) as u64)))
            .transpose()
    }

    fn parse_fixed(caps: &Captures) -> Result<Option<Self>, &'static str> {
        let mut iter = [
            Self::opt_number::<u64>(caps, "nodes", "Unable to parse nodes.")?.map(Self::FixedNodes),
            Self::opt_number::<i8>(caps, "depth", "Unable to parse depth.")?.map(Self::FixedDepth),
            Self::opt_duration(caps, "movetime")?.map(Self::FixedDuration),
        ]
        .into_iter()
        .flatten();

        let first = iter.next();
        if iter.next().is_some() {
            return Err("Only one of depth, nodes, or movetime may be given.");
        }

        Ok(first)
    }

    fn parse_variable(caps: &Captures) -> Result<Option<Self>, &'static str> {
        let wtime = Self::opt_duration(caps, "wtime")?;
        let btime = Self::opt_duration(caps, "btime")?;

        let winc = Self::opt_duration(caps, "winc")?;
        let binc = Self::opt_duration(caps, "binc")?;
        let moves_to_go = Self::opt_number::<u32>(caps, "movestogo", "Unable to parse movestogo.")?;

        if wtime.is_none() && btime.is_none() {
            return Ok(None);
        }

        Ok(Some(Self::Variable {
            wtime: wtime.unwrap_or(Duration::ZERO),
            btime: btime.unwrap_or(Duration::ZERO),
            winc,
            binc,
            moves_to_go,
        }))
    }
}

impl TryFrom<&str> for TimeControl {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if matches!(line, "go" | "go ponder") {
            return Ok(TimeControl::Infinite);
        }

        let caps = GO_RE.captures(line).ok_or("Invalid go format.")?;

        if caps.name("searchmoves").is_some() || caps.name("mate").is_some() {
            return Err("Feature is not implemented.");
        }

        Self::parse_fixed(&caps)?
            .xor(Self::parse_variable(&caps)?)
            .ok_or("No recognizable or bad combination of go parameters provided.")
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
    stop: Arc<AtomicBool>,
    nodes: Arc<AtomicU64>,
    batch: u64,
    time_target: Duration,
    time_maximum: Duration,
    overhead: Duration,
    current_nodes: u64,
    nodes_table: SQMap<SQMap<u64>>,
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
            stop,
            batch: 0,
            nodes,
            control,
            overhead,
            time_target,
            time_maximum,
            current_nodes: 0,
            nodes_table: SQMap::new([SQMap::new([0; SQ::N_SQUARES]); SQ::N_SQUARES]),
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

    pub fn start_check(&mut self, best_move: Option<Move>, depth: i8) -> bool {
        if self.stop.load(Ordering::Acquire) {
            return false;
        }

        if self.pondering.load(Ordering::Acquire) {
            return true;
        }

        let start = match self.control {
            TimeControl::Infinite => true,
            TimeControl::FixedDuration(duration) => self.elapsed() + self.overhead <= duration,
            TimeControl::FixedDepth(stop_depth) => depth <= stop_depth,
            TimeControl::FixedNodes(_) => true,
            TimeControl::Variable { .. } => {
                self.elapsed() + self.overhead
                    <= self
                        .time_target
                        .mul_f64(self.scale_factor(best_move, depth))
                        / 2
            }
        };

        if !start {
            self.set_stop();
        }
        start
    }

    pub fn stop_check(&mut self) -> bool {
        self.increment();

        if self.stop.load(Ordering::Acquire) {
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
            TimeControl::FixedNodes(stop_nodes) => self.nodes() >= stop_nodes,
        };

        if stop {
            self.set_stop();
        }

        stop
    }

    pub fn set_stop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn increment(&mut self) {
        self.batch += 1;
        self.current_nodes += 1;
        if self.batch >= Self::BATCH_SIZE {
            self.nodes.fetch_add(self.batch, Ordering::Relaxed);
            self.batch = 0;
        }
    }

    pub fn nodes(&self) -> u64 {
        self.nodes.load(Ordering::Relaxed) + self.batch
    }

    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    pub fn update_node_table(&mut self, m: Move) {
        let (from_sq, to_sq) = m.squares();
        self.nodes_table[from_sq][to_sq] += self.current_nodes;
        self.current_nodes = 0;
    }

    pub fn scale_factor(&self, best_move: Option<Move>, depth: i8) -> f64 {
        let Some(m) = best_move else {
            return 1.0;
        };

        if depth <= Self::SEARCHES_WO_TIMER_UPDATE {
            return 1.0;
        }

        let total_nodes = self.nodes_table.into_iter().flatten().sum::<u64>();
        if total_nodes == 0 {
            return 1.0;
        }

        let (from_sq, to_sq) = m.squares();
        let effort_ratio = self.nodes_table[from_sq][to_sq] as f64 / total_nodes as f64;
        let logistic = 1.0 / (1.0 + (-Self::K * (effort_ratio - Self::X0)).exp());
        Self::MIN_TIMER_UPDATE
            + (Self::MAX_TIMER_UPDATE - Self::MIN_TIMER_UPDATE) * (1.0 - logistic)
    }
}

impl Timer {
    const BATCH_SIZE: u64 = 4096;
    const K: f64 = 10.0;
    const X0: f64 = 0.5;
    const MIN_TIMER_UPDATE: f64 = 0.5;
    const MAX_TIMER_UPDATE: f64 = 3.0;
    const SEARCHES_WO_TIMER_UPDATE: i8 = 8;
}
