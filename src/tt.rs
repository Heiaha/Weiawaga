#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;
use std::sync::atomic::{AtomicU64, Ordering};

use super::board::*;
use super::moov::*;
use super::search::*;

///////////////////////////////////////////////////////////////////
// Transposition Table Entry
///////////////////////////////////////////////////////////////////

#[derive(Eq, PartialEq, Copy, Clone, Default)]
pub struct TTEntry(u64);

impl TTEntry {
    fn new(value: i32, best_move: Option<Move>, depth: i8, bound: Bound, age: u8) -> Self {
        let m16 = best_move.map_or(0, |m| m.move_int()) as u64;
        let value16 = value as i16 as u16 as u64;
        let depth8 = depth as u8 as u64;
        let bound2 = bound as u8 as u64;
        let age6 = (age & Self::AGE_MASK as u8) as u64; // mask to 6 bits

        Self(
            m16 | (value16 << Self::VALUE_SHIFT)
                | (depth8 << Self::DEPTH_SHIFT)
                | (bound2 << Self::BOUND_SHIFT)
                | (age6 << Self::AGE_SHIFT),
        )
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
        if m == 0 { None } else { Some(Move::from(m)) }
    }
}

impl From<u64> for TTEntry {
    fn from(value: u64) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<TTEntry> for u64 {
    fn from(value: TTEntry) -> Self {
        unsafe { std::mem::transmute(value) }
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
    const VALUE_SHIFT: usize = 16;
}

///////////////////////////////////////////////////////////////////
// Transposition Table
///////////////////////////////////////////////////////////////////

pub struct TT {
    table: Vec<AtomicEntry>,
    bitmask: u64,
    age: u8,
}

impl TT {
    pub fn new(mb_size: usize) -> Self {
        let upper_limit = mb_size * 1024 * 1024 / size_of::<AtomicEntry>() + 1;
        let count = upper_limit.next_power_of_two() / 2;
        let mut table = Vec::with_capacity(count);

        for _ in 0..count {
            table.push(AtomicEntry::default());
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
        value: i32,
        best_move: Option<Move>,
        bound: Bound,
    ) {
        unsafe {
            let idx = self.index(board);
            let entry = self.table.get_unchecked(idx).read(board.hash());

            if entry.is_none_or(|entry| {
                bound == Bound::Exact
                    || self.age != entry.age()
                    || depth >= entry.depth() - Self::DEPTH_MARGIN
            }) {
                self.table.get_unchecked(idx).write(
                    board.hash(),
                    TTEntry::new(value, best_move, depth, bound, self.age),
                )
            }
        }
    }

    pub fn get(&self, board: &Board) -> Option<TTEntry> {
        unsafe {
            self.table
                .get_unchecked(self.index(board))
                .read(board.hash())
        }
    }

    pub fn clear(&mut self) {
        self.table
            .iter_mut()
            .for_each(|entry| *entry = AtomicEntry::default());
    }

    pub fn age_up(&mut self) {
        self.age = self.age.wrapping_add(1) & TTEntry::AGE_MASK as u8;
    }

    fn index(&self, board: &Board) -> usize {
        (board.hash() & self.bitmask) as usize
    }

    pub fn mb_size(&self) -> usize {
        self.table.len() * size_of::<AtomicEntry>() / 1024 / 1024
    }

    pub fn hashfull(&self) -> usize {
        // Sample the first 1000 entries to estimate how full the table is.
        self.table
            .iter()
            .take(1000)
            .filter(|&atomic_entry| {
                let entry = atomic_entry.entry();
                entry.age() == self.age
            })
            .count()
    }

    #[allow(unused_variables)]
    pub fn prefetch(&self, board: &Board) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let ptr =
                self.table.get_unchecked(self.index(board)) as *const AtomicEntry as *const i8;
            x86_64::_mm_prefetch(ptr, x86_64::_MM_HINT_T0);
        }
    }
}

impl TT {
    const DEPTH_MARGIN: i8 = 2;
}

///////////////////////////////////////////////////////////////////
// Atomic value for storage.
///////////////////////////////////////////////////////////////////

#[derive(Default)]
struct AtomicEntry {
    checksum: AtomicU64,
    data: AtomicU64,
}

impl AtomicEntry {
    fn read(&self, hash: u64) -> Option<TTEntry> {
        let (checksum, data) = (
            self.checksum.load(Ordering::Relaxed),
            self.data.load(Ordering::Relaxed),
        );
        if checksum ^ data == hash {
            Some(TTEntry::from(data))
        } else {
            None
        }
    }

    fn entry(&self) -> TTEntry {
        TTEntry::from(self.data.load(Ordering::Relaxed))
    }

    fn write(&self, hash: u64, entry: TTEntry) {
        let data = u64::from(entry);
        self.checksum.store(hash ^ data, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    fn is_used(&self) -> bool {
        self.checksum.load(Ordering::Relaxed) != u64::default()
    }
}
