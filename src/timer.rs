use std::cmp::min;
use std::sync;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use super::board::*;
use super::color::*;
use super::types::*;

// Some ideas taken from asymptote, which has a very elegant timer implementation.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TimeControl {
    Infinite,
    FixedMillis(Time),
    FixedDepth(Depth),
    FixedNodes(u64),
    Variable {
        wtime: Time,
        btime: Time,
        winc: Option<Time>,
        binc: Option<Time>,
        moves_to_go: Option<u64>,
    },
}

impl TryFrom<&str> for TimeControl {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut result = Ok(Self::Infinite);

        let mut wtime: Option<Time> = None;
        let mut btime: Option<Time> = None;
        let mut winc: Option<Time> = None;
        let mut binc: Option<Time> = None;
        let mut moves_to_go: Option<u64> = None;

        let mut split = s.split_whitespace();
        while let Some(s) = split.next() {
            if s == "movetime" {
                result = Ok(Self::FixedMillis(
                    split
                        .next()
                        .ok_or("Must provide a movetime value")?
                        .parse()
                        .or(Err("Unable to parse movetime value."))?,
                ));
            } else if s == "infinite" {
                result = Ok(Self::Infinite);
            } else if s == "nodes" {
                result = Ok(Self::FixedNodes(
                    split
                        .next()
                        .ok_or("Must provides a nodes value.")?
                        .parse()
                        .or(Err("Unable to parse nodes value."))?,
                ));
            } else if s == "depth" {
                result = Ok(Self::FixedDepth(
                    split
                        .next()
                        .ok_or("Must provides a depth value.")?
                        .parse()
                        .or(Err("Unable to parse depth value."))?,
                ));
            } else if s == "wtime" {
                wtime = Some(
                    split
                        .next()
                        .ok_or("Must provide a wtime value")?
                        .parse()
                        .or(Err("Unable to parse wtime value."))?,
                );
            } else if s == "btime" {
                btime = Some(
                    split
                        .next()
                        .ok_or("Must provide a btime value")?
                        .parse()
                        .or(Err("Unable to parse btime value."))?,
                );
            } else if s == "winc" {
                winc = Some(
                    split
                        .next()
                        .ok_or("Must provide a winc value")?
                        .parse()
                        .or(Err("Unable to parse winc value."))?,
                );
            } else if s == "binc" {
                binc = Some(
                    split
                        .next()
                        .ok_or("Must provide a binc value")?
                        .parse()
                        .or(Err("Unable to parse binc value."))?,
                );
            } else if s == "movestogo" {
                moves_to_go = Some(
                    split
                        .next()
                        .ok_or("Must provide a movestogo value")?
                        .parse()
                        .or(Err("Unable to parse movestogo value."))?,
                );
            }
        }

        if let (Some(wtime), Some(btime)) = (wtime, btime) {
            result = Ok(Self::Variable {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
            });
        }
        result
    }
}

#[derive(Clone)]
pub struct Timer {
    control: TimeControl,
    start_time: Instant,
    stop: Arc<AtomicBool>,
    times_checked: u64,
    time_target: Time,
    time_maximum: Time,
    overhead: Time,
    last_score: Value,
}

impl Timer {
    pub fn new(board: &Board, control: TimeControl, stop: Arc<AtomicBool>, overhead: Time) -> Self {
        let mut tm = Self {
            start_time: Instant::now(),
            stop,
            control,
            overhead,
            last_score: 0,
            times_checked: 0,
            time_target: 0,
            time_maximum: 0,
        };
        tm.calc(board);
        tm
    }

    fn calc(&mut self, board: &Board) {
        if let TimeControl::Variable {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
        } = self.control
        {
            let time = if board.color_to_play() == Color::White {
                wtime
            } else {
                btime
            };
            let inc = if board.color_to_play() == Color::White {
                winc
            } else {
                binc
            }
            .unwrap_or(0);

            let target = time.min(time / moves_to_go.unwrap_or(40) + inc);
            self.time_target = target as Time;
            self.time_maximum = (target + (time - target) / 4) as Time;
        }
    }

    pub fn start_check(&self, depth: Depth) -> bool {
        if self.stop.load(sync::atomic::Ordering::Relaxed) {
            return false;
        }

        let start = match self.control {
            TimeControl::Infinite => true,
            TimeControl::FixedMillis(millis) => self.elapsed() + self.overhead <= millis,
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
        if self.times_checked & Self::CHECK_FLAG == 0
            && self.stop.load(sync::atomic::Ordering::Relaxed)
        {
            return true;
        }

        let stop = match self.control {
            TimeControl::Infinite => false,
            TimeControl::FixedMillis(millis) => {
                if self.times_checked & Self::CHECK_FLAG == 0 {
                    self.elapsed() + self.overhead >= millis
                } else {
                    false
                }
            }
            TimeControl::Variable { .. } => {
                if self.times_checked & Self::CHECK_FLAG == 0 {
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

    #[inline(always)]
    pub fn elapsed(&self) -> Time {
        self.start_time.elapsed().as_millis() as Time
    }

    pub fn update(&mut self, score: Value) {
        let diff = score - self.last_score;
        self.last_score = score;

        if diff > -25 {
            return;
        }

        if diff > -75 {
            self.time_target = min(self.time_maximum, self.time_target * 5 / 4);
        }
        self.time_target = min(self.time_maximum, self.time_target * 3 / 2);
    }
}

impl Timer {
    const CHECK_FLAG: u64 = 0x1000 - 1;
}
