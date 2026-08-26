use super::board::*;
use super::moov::*;
use super::piece::*;
use super::square::*;
use regex_lite::{Captures, Regex};
use std::str::FromStr;
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
            if winc.is_some() || binc.is_some() || moves_to_go.is_some() {
                return Err("Increment or movestogo given without a clock time.");
            }
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

impl FromStr for TimeControl {
    type Err = &'static str;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        // Unknown tokens match the trailing catch-all and are skipped, as
        // the protocol asks. This includes searchmoves: the restriction is
        // ignored and every root move is searched.
        static GO_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?x)^
                go
                (?:
                    \s+(?P<infinite>infinite) |
                    \s+depth\s+(?P<depth>\d+) |
                    \s+nodes\s+(?P<nodes>\d+) |
                    \s+movetime\s+(?P<movetime>\d+) |
                    \s+wtime\s+(?P<wtime>-?\d+) |
                    \s+btime\s+(?P<btime>-?\d+) |
                    \s+winc\s+(?P<winc>\d+) |
                    \s+binc\s+(?P<binc>\d+) |
                    \s+mate\s+(?P<mate>\d+) |
                    \s+movestogo\s+(?P<movestogo>\d+) |
                    \s+(?P<ponder>ponder) |
                    \s+\S+
                )*
            \s*$",
            )
            .expect("Go regex should be valid.")
        });

        let caps = GO_RE.captures(line).ok_or("Invalid go format.")?;

        if caps.name("mate").is_some() {
            return Err("Feature is not implemented.");
        }

        if caps.name("infinite").is_some() {
            return Ok(TimeControl::Infinite);
        }

        let fixed = Self::parse_fixed(&caps)?;
        let variable = Self::parse_variable(&caps)?;

        if fixed.is_some() && variable.is_some() {
            return Err("Bad combination of go parameters provided.");
        }

        // Nothing recognized searches until told to stop, as for a bare go;
        // an error here would leave the GUI waiting on a bestmove that
        // never comes.
        Ok(fixed.or(variable).unwrap_or(TimeControl::Infinite))
    }
}

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
        let (time_target, time_maximum) = match control {
            TimeControl::Variable {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
            } => Self::calculate_time(board, wtime, btime, winc, binc, moves_to_go),
            _ => (Duration::ZERO, Duration::ZERO),
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
            nodes_table: SQMap::default(),
        }
    }

    fn calculate_time(
        board: &Board,
        wtime: Duration,
        btime: Duration,
        winc: Option<Duration>,
        binc: Option<Duration>,
        moves_to_go: Option<u32>,
    ) -> (Duration, Duration) {
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

        if depth <= 1 {
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

        let total_nodes = self.nodes_table.iter().flatten().sum::<u64>();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_fixed_controls_parse() {
        assert_eq!("go".parse(), Ok(TimeControl::Infinite));
        assert_eq!("go ponder".parse(), Ok(TimeControl::Infinite));
        assert_eq!("go infinite".parse(), Ok(TimeControl::Infinite));
        // Infinite outranks any clocks given alongside it.
        assert_eq!("go infinite wtime 100".parse(), Ok(TimeControl::Infinite));
        assert_eq!("go depth 8".parse(), Ok(TimeControl::FixedDepth(8)));
        assert_eq!("go nodes 50000".parse(), Ok(TimeControl::FixedNodes(50000)));
        assert_eq!(
            "go movetime 200".parse(),
            Ok(TimeControl::FixedDuration(Duration::from_millis(200)))
        );
    }

    #[test]
    fn go_clocks_parse_in_any_order() {
        let control = Ok(TimeControl::Variable {
            wtime: Duration::from_millis(3000),
            btime: Duration::from_millis(2000),
            winc: Some(Duration::from_millis(100)),
            binc: Some(Duration::from_millis(50)),
            moves_to_go: Some(40),
        });
        assert_eq!(
            "go wtime 3000 btime 2000 winc 100 binc 50 movestogo 40".parse(),
            control
        );
        assert_eq!(
            "go movestogo 40 binc 50 wtime 3000 winc 100 btime 2000".parse(),
            control
        );
    }

    #[test]
    fn go_negative_clock_clamps_to_zero() {
        assert_eq!(
            "go wtime -50 btime 1000".parse(),
            Ok(TimeControl::Variable {
                wtime: Duration::ZERO,
                btime: Duration::from_millis(1000),
                winc: None,
                binc: None,
                moves_to_go: None,
            })
        );
    }

    #[test]
    fn go_skips_unknown_tokens() {
        assert_eq!(
            "go wtime 1000 btime 1000 searchmoves e2e4 xyzzy".parse(),
            Ok(TimeControl::Variable {
                wtime: Duration::from_millis(1000),
                btime: Duration::from_millis(1000),
                winc: None,
                binc: None,
                moves_to_go: None,
            })
        );
        assert_eq!("go frobnicate".parse(), Ok(TimeControl::Infinite));
        assert!("go movetime 200  ".parse::<TimeControl>().is_ok());
    }

    #[test]
    fn go_rejects_conflicting_limits() {
        assert!(
            "go depth 5 wtime 1000 btime 1000"
                .parse::<TimeControl>()
                .is_err()
        );
        assert!("go depth 5 nodes 100".parse::<TimeControl>().is_err());
    }

    #[test]
    fn go_rejects_orphan_clock_parameters() {
        assert!("go movestogo 30".parse::<TimeControl>().is_err());
        assert!("go winc 100".parse::<TimeControl>().is_err());
    }

    #[test]
    fn go_rejects_unsupported_or_overflowing_values() {
        assert!("go mate 3".parse::<TimeControl>().is_err());
        assert!("go depth 200".parse::<TimeControl>().is_err());
    }
}
