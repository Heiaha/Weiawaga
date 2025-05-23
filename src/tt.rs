#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;
use std::sync::atomic::{AtomicU64, Ordering};

use super::board::*;
use super::moov::*;
use super::search::*;

///////////////////////////////////////////////////////////////////
// Transposition Table Entry
///////////////////////////////////////////////////////////////////

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct TTEntry {
    value: i32,
    best_move: Option<Move>,
    depth: i8,
    flag: Bound,
}

impl TTEntry {
    pub fn new(value: i32, best_move: Option<Move>, depth: i8, flag: Bound) -> Self {
        TTEntry {
            best_move,
            depth,
            value,
            flag,
        }
    }

    pub fn best_move(&self) -> Option<Move> {
        self.best_move
    }

    pub fn depth(&self) -> i8 {
        self.depth
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn flag(&self) -> Bound {
        self.flag
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            best_move: None,
            depth: 0,
            value: 0,
            flag: Bound::Exact,
        }
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

///////////////////////////////////////////////////////////////////
// Transposition Table
///////////////////////////////////////////////////////////////////

pub struct TT {
    table: Vec<AtomicEntry>,
    bitmask: u64,
}

impl TT {
    pub fn new(mb_size: usize) -> Self {
        assert_eq!(size_of::<TTEntry>(), 8);
        let upper_limit = mb_size * 1024 * 1024 / size_of::<AtomicEntry>() + 1;
        let count = upper_limit.next_power_of_two() / 2;
        let mut table = Vec::with_capacity(count);

        for _ in 0..count {
            table.push(AtomicEntry::default());
        }

        TT {
            table,
            bitmask: count as u64 - 1,
        }
    }

    pub fn insert(
        &self,
        board: &Board,
        depth: i8,
        value: i32,
        best_move: Option<Move>,
        flag: Bound,
    ) {
        unsafe {
            self.table
                .get_unchecked(self.index(board))
                .write(board.hash(), TTEntry::new(value, best_move, depth, flag))
        }
    }

    pub fn probe(&self, board: &Board) -> Option<TTEntry> {
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
            .filter(|&entry| entry.is_used())
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

    fn write(&self, hash: u64, entry: TTEntry) {
        let data = u64::from(entry);
        self.checksum.store(hash ^ data, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    fn is_used(&self) -> bool {
        self.checksum.load(Ordering::Relaxed) != u64::default()
    }
}
