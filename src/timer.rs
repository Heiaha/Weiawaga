use std::cmp::min;
use std::str::{FromStr, SplitWhitespace};
use std::sync;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::board::*;
use super::color::*;
use super::moov::*;
use super::types::*;

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
    pub fn parse_next<T: FromStr>(split: &mut SplitWhitespace) -> Result<T, &'static str> {
        split
            .next()
            .ok_or("Must provide a value")?
            .parse()
            .or(Err("Unable to parse value."))
    }
}

impl TryFrom<&str> for TimeControl {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Ok(Self::Infinite);
        }

        let mut wtime = None;
        let mut btime = None;
        let mut winc = None;
        let mut binc = None;
        let mut moves_to_go = None;

        let mut split = s.split_whitespace();
        while let Some(key) = split.next() {
            match key {
                "infinite" => return Ok(Self::Infinite),
                "movetime" => {
                    return Ok(Self::FixedDuration(Duration::from_millis(
                        Self::parse_next(&mut split)?,
                    )))
                }
                "nodes" => return Ok(Self::FixedNodes(Self::parse_next(&mut split)?)),
                "depth" => return Ok(Self::FixedDepth(Self::parse_next(&mut split)?)),
                "wtime" => wtime = Some(Duration::from_millis(Self::parse_next(&mut split)?)),
                "btime" => btime = Some(Duration::from_millis(Self::parse_next(&mut split)?)),
                "winc" => winc = Some(Duration::from_millis(Self::parse_next(&mut split)?)),
                "binc" => binc = Some(Duration::from_millis(Self::parse_next(&mut split)?)),
                "movestogo" => moves_to_go = Some(Self::parse_next(&mut split)?),
                _ => continue,
            }
        }

        if (wtime.is_none() && btime.is_some()) || (wtime.is_some() && btime.is_none()) {
            return Err("Must provide both wtime and btime.");
        }

        if let (Some(wtime), Some(btime)) = (wtime, btime) {
            return Ok(Self::Variable {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
            });
        }
        Err("Unable to parse go parameters.")
    }
}

#[derive(Clone)]
pub struct Timer {
    control: TimeControl,
    start_time: Instant,
    stop: Arc<AtomicBool>,
    times_checked: u64,
    time_target: Duration,
    time_maximum: Duration,
    overhead: Duration,
    best_move: Move,
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

            let mtg = moves_to_go.unwrap_or(
                (Self::MTG_INTERCEPT
                    + Self::MTG_EVAL_WEIGHT * (board.simple_eval().abs() as f32)
                    + Self::MTG_MOVE_WEIGHT * (board.fullmove_number() as f32))
                    .ceil()
                    .max(1.0) as u32,
            );

            time_target = min(time / mtg + inc.unwrap_or(Duration::ZERO), time);
            time_maximum = time_target + (time - time_target) / 4;
        }

        Self {
            start_time: Instant::now(),
            stop,
            control,
            overhead,
            time_target,
            time_maximum,
            best_move: Move::NULL,
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

    pub fn update(&mut self, best_move: Move) {
        if !self.best_move.is_null() && best_move != self.best_move {
            self.time_target = min(self.time_maximum, self.time_target * 3 / 2);
        }

        self.best_move = best_move;
    }
}

impl Timer {
    const CHECK_FLAG: u64 = 0x1000 - 1;
    const MTG_INTERCEPT: f32 = 52.52;
    const MTG_EVAL_WEIGHT: f32 = -0.01833;
    const MTG_MOVE_WEIGHT: f32 = -0.4657;
}
