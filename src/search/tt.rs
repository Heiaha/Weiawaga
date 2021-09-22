use crate::evaluation::score::Value;
use crate::search::search::Depth;
use crate::types::bitboard::BitBoard;
use crate::types::bitboard::Key;
use crate::types::moov::{Move, MoveInt};
use std::mem::size_of;

pub struct TT {
    table: Vec<TTEntry>,
    bitmask: Key,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TTFlag {
    EXACT,
    LOWER,
    UPPER,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct TTEntry {
    key: Key,
    best_move: Option<MoveInt>,
    depth: Depth,
    value: Value,
    flag: TTFlag,
}

impl TT {
    pub fn new(mb_size: u64) -> Self {
        let count = mb_size * 1024 * 1024 / std::mem::size_of::<TTEntry>() as u64;
        let new_ttentry_count = count.next_power_of_two() / 2;
        let bitmask = B!(new_ttentry_count - 1);
        let table = vec![TTEntry::default(); new_ttentry_count as usize];
        TT { table, bitmask }
    }

    pub fn insert(
        &mut self,
        hash: Key,
        depth: Depth,
        value: Value,
        best_move: Option<Move>,
        flag: TTFlag,
    ) {
        self.table[(hash & self.bitmask).0 as usize] = TTEntry {
            key: hash,
            best_move: match best_move {
                Some(best_move) => Some(best_move.moove()),
                None => None,
            },
            depth,
            value,
            flag,
        };
    }

    pub fn probe(&self, hash: Key) -> Option<&TTEntry> {
        unsafe {
            let entry = self.table.get_unchecked((hash & self.bitmask).0 as usize);
            if entry.key == hash {
                return Some(entry);
            }
            None
        }
    }

    pub fn use_pct(&self) -> f64 {
        let mut count: u64 = 0;
        for i in 0..self.table.len() {
            if self.table[i] != TTEntry::default() {
                count += 1;
            }
        }
        count as f64 / self.table.len() as f64
    }

    pub fn clear(&mut self) {
        for i in 0..self.table.len() {
            self.table[i] = TTEntry::default();
        }
    }

    pub fn resize(&mut self, mb_size: u64) {
        let count = mb_size * 1024 * 1024 / std::mem::size_of::<TTEntry>() as u64;
        let new_ttentry_count = count.next_power_of_two() / 2;
        self.bitmask = B!(new_ttentry_count - 1);
        self.table = vec![TTEntry::default(); new_ttentry_count as usize];
    }
}

impl TTEntry {
    #[inline(always)]
    pub fn best_move(&self) -> Option<Move> {
        return match self.best_move {
            Some(best_move) => Some(Move::from(best_move)),
            None => None,
        };
    }

    #[inline(always)]
    pub fn depth(&self) -> Depth {
        self.depth
    }

    #[inline(always)]
    pub fn value(&self) -> Value {
        self.value
    }

    #[inline(always)]
    pub fn flag(&self) -> TTFlag {
        self.flag
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            key: B!(0),
            best_move: None,
            depth: 0,
            value: 0,
            flag: TTFlag::EXACT,
        }
    }
}
