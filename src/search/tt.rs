use super::search::*;
use crate::evaluation::score::*;
use crate::types::bitboard::*;
use crate::types::moov::*;
use std::sync::atomic::{AtomicU64, Ordering};

type TTValue = i16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TTFlag {
    Exact,
    Lower,
    Upper,
}

///////////////////////////////////////////////////////////////////
// Transposition Table Entry
///////////////////////////////////////////////////////////////////

#[derive(Eq, PartialEq, Copy, Clone)]
#[repr(align(8))]
pub struct TTEntry {
    value: TTValue,
    best_move: Option<MoveInt>,
    depth: Depth,
    flag: TTFlag,
}

impl TTEntry {
    pub fn new(value: Value, best_move: Option<Move>, depth: Depth, flag: TTFlag) -> Self {
        TTEntry {
            best_move: best_move.map(|x| x.moove()),
            depth,
            value: value as TTValue,
            flag,
        }
    }
    #[inline(always)]
    pub fn best_move(&self) -> Option<Move> {
        self.best_move.map(Move::from)
    }

    #[inline(always)]
    pub fn depth(&self) -> Depth {
        self.depth
    }

    #[inline(always)]
    pub fn value(&self) -> Value {
        self.value as Value
    }

    #[inline(always)]
    pub fn flag(&self) -> TTFlag {
        self.flag
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            best_move: None,
            depth: 0,
            value: 0,
            flag: TTFlag::Exact,
        }
    }
}

impl From<Hash> for TTEntry {
    fn from(value: Hash) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<&TTEntry> for Hash {
    fn from(value: &TTEntry) -> Self {
        unsafe { std::mem::transmute(*value) }
    }
}

///////////////////////////////////////////////////////////////////
// Transposition Table
///////////////////////////////////////////////////////////////////

pub struct TT {
    table: Vec<AtomicEntry>,
    bitmask: Hash,
}

impl TT {
    pub fn new(mb_size: usize) -> Self {
        let upper_limit = mb_size * 1024 * 1024 / std::mem::size_of::<AtomicEntry>() + 1;
        let count = upper_limit.next_power_of_two() / 2;
        let mut table = Vec::with_capacity(count);

        for _ in 0..count {
            table.push(AtomicEntry::default());
        }

        TT {
            table,
            bitmask: B!(count as u64 - 1),
        }
    }

    pub fn insert(
        &self,
        hash: Hash,
        depth: Depth,
        value: Value,
        best_move: Option<Move>,
        flag: TTFlag,
    ) {
        let entry = TTEntry::new(value, best_move, depth, flag);
        self.table[(hash & self.bitmask).0 as usize].write(hash, &entry)
    }

    pub fn probe(&self, hash: Hash) -> Option<TTEntry> {
        self.table[(hash & self.bitmask).0 as usize].read(hash)
    }

    pub fn clear(&mut self) {
        for j in 0..self.table.len() {
            self.table[j] = AtomicEntry::default();
        }
    }

    pub fn mb_size(&self) -> usize {
        self.table.len() * std::mem::size_of::<AtomicEntry>() / 1024 / 1024
    }
}

///////////////////////////////////////////////////////////////////
// Atomic value for storage.
///////////////////////////////////////////////////////////////////

#[derive(Default)]
struct AtomicEntry(AtomicU64, AtomicU64);

impl AtomicEntry {
    fn read(&self, lookup_hash: Hash) -> Option<TTEntry> {
        let entry_hash = Hash::from(self.0.load(Ordering::Relaxed));
        let data = Hash::from(self.1.load(Ordering::Relaxed));
        if entry_hash ^ data == lookup_hash {
            return Some(TTEntry::from(data));
        }
        None
    }

    fn write(&self, hash: Hash, entry: &TTEntry) {
        let data = Hash::from(entry);
        self.0.store((hash ^ data).0, Ordering::Relaxed);
        self.1.store(data.0, Ordering::Relaxed);
    }
}
