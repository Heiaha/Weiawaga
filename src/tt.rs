use super::board::*;
use super::moov::*;
use super::search::*;
use crate::types::Score;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;
use std::sync::atomic::{AtomicU64, Ordering};

///////////////////////////////////////////////////////////////////
// Transposition Table Entry
///////////////////////////////////////////////////////////////////

#[derive(Eq, PartialEq, Copy, Clone, Default)]
#[repr(transparent)]
pub struct TTEntry(u64);

impl TTEntry {
    fn new(
        hash: u64,
        value: i32,
        best_move: Option<Move>,
        depth: i8,
        bound: Bound,
        age: u8,
    ) -> Self {
        let key16 = (hash >> Self::KEY_SHIFT) as u16 as u64;
        let m16 = best_move.map_or(0, |m| m.move_int()) as u64;
        let value16 = value as i16 as u16 as u64;
        let depth8 = depth as u8 as u64;
        let bound2 = bound as u8 as u64;
        let age6 = (age & Self::AGE_MASK as u8) as u64; // mask to 6 bits

        Self(
            m16 | (value16 << Self::VALUE_SHIFT)
                | (depth8 << Self::DEPTH_SHIFT)
                | (bound2 << Self::BOUND_SHIFT)
                | (age6 << Self::AGE_SHIFT)
                | (key16 << Self::KEY_SHIFT),
        )
    }

    pub fn key(self) -> u64 {
        self.0 >> Self::KEY_SHIFT
    }

    pub fn age(self) -> u8 {
        ((self.0 >> Self::AGE_SHIFT) & Self::AGE_MASK) as u8
    }

    pub fn depth(self) -> i8 {
        ((self.0 >> Self::DEPTH_SHIFT) & Self::DEPTH_MASK) as u8 as i8
    }

    pub fn bound(self) -> Bound {
        unsafe { core::mem::transmute(((self.0 >> Self::BOUND_SHIFT) & Self::BOUND_MASK) as u8) }
    }

    pub fn value(self) -> i32 {
        ((self.0 >> Self::VALUE_SHIFT) & Self::VALUE_MASK) as u16 as i16 as i32
    }

    pub fn best_move(self) -> Option<Move> {
        let m = (self.0 & Self::MOVE_MASK) as u16;
        (m != 0).then(|| Move::from(m))
    }

    pub fn with_value(self, value: i32) -> Self {
        let value16 = (value as i16 as u16 as u64) << Self::VALUE_SHIFT;
        let cleared = self.0 & !(Self::VALUE_MASK << Self::VALUE_SHIFT);
        Self(cleared | value16)
    }
}

impl TTEntry {
    const AGE_MASK: u64 = 0x3F;
    const BOUND_MASK: u64 = 0x3;
    const DEPTH_MASK: u64 = 0xFF;
    const MOVE_MASK: u64 = 0xFFFF;
    const VALUE_MASK: u64 = 0xFFFF;

    const AGE_SHIFT: usize = 42;
    const BOUND_SHIFT: usize = 40;
    const DEPTH_SHIFT: usize = 32;
    const KEY_SHIFT: usize = 48;
    const VALUE_SHIFT: usize = 16;
}

///////////////////////////////////////////////////////////////////
// Transposition Table
///////////////////////////////////////////////////////////////////

pub struct TT {
    table: Vec<AtomicU64>,
    bitmask: u64,
    age: u8,
}

impl TT {
    pub fn new(megabytes: usize) -> Self {
        let upper_limit = megabytes * 1024 * 1024 / size_of::<AtomicU64>() + 1;
        let count = upper_limit.next_power_of_two() / 2;
        let mut table = Vec::with_capacity(count);

        for _ in 0..count {
            table.push(AtomicU64::new(0));
        }

        TT {
            table,
            bitmask: count as u64 - 1,
            age: 0,
        }
    }

    pub fn insert(
        &self,
        board: &Board,
        depth: i8,
        mut value: i32,
        best_move: Option<Move>,
        bound: Bound,
        ply: usize,
    ) {
        let idx = self.index(board);
        debug_assert!(idx < self.table.len());
        let aentry = &self.table[idx];
        let data = aentry.load(Ordering::Relaxed);
        let entry = (data != 0).then_some(TTEntry(data));

        if entry.is_none_or(|entry| {
            bound == Bound::Exact
                || self.age != entry.age()
                || depth >= entry.depth() - Self::DEPTH_MARGIN
        }) {
            if value.is_checkmate() {
                value += value.signum() * ply as i32;
            }

            aentry.store(
                TTEntry::new(board.hash(), value, best_move, depth, bound, self.age).0,
                Ordering::Relaxed,
            );
        }
    }

    pub fn get(&self, board: &Board, ply: usize) -> Option<TTEntry> {
        let idx = self.index(board);
        debug_assert!(idx < self.table.len());

        let data = self.table[idx].load(Ordering::Relaxed);
        if data == 0 {
            return None;
        }

        let mut entry = TTEntry(data);
        if entry.key() != (board.hash() >> TTEntry::KEY_SHIFT) {
            return None;
        }

        let value = entry.value();
        if value.is_checkmate() {
            entry = entry.with_value(value - value.signum() * ply as i32);
        }

        Some(entry)
    }

    pub fn clear(&self) {
        self.table
            .iter()
            .for_each(|entry| entry.store(0, Ordering::Relaxed));
    }

    pub fn age_up(&mut self) {
        self.age = (self.age + 1) & TTEntry::AGE_MASK as u8;
    }

    fn index(&self, board: &Board) -> usize {
        (board.hash() & self.bitmask) as usize
    }

    pub fn mb_size(&self) -> usize {
        self.table.len() * size_of::<AtomicU64>() / 1024 / 1024
    }

    pub fn hashfull(&self) -> usize {
        // Sample the first 1000 entries to estimate how full the table is.
        self.table
            .iter()
            .take(1000)
            .filter(|&aentry| {
                let data = aentry.load(Ordering::Relaxed);
                let entry = (data != 0).then_some(TTEntry(data));
                entry.is_some_and(|entry| entry.age() == self.age)
            })
            .count()
    }

    #[allow(unused_variables)]
    pub fn prefetch(&self, board: &Board) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let ptr = &self.table[self.index(board)] as *const AtomicU64 as *const i8;
            x86_64::_mm_prefetch(ptr, x86_64::_MM_HINT_T0);
        }
    }
}

impl TT {
    const DEPTH_MARGIN: i8 = 2;
}
