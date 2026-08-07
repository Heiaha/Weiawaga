use super::board::*;
use super::moov::*;
use super::search::*;
use super::types::*;
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
    const fn new(
        hash: u64,
        value: i32,
        best_move: Option<Move>,
        depth: i8,
        bound: Bound,
        age: u8,
    ) -> Self {
        let key16 = (hash >> Self::KEY_SHIFT) as u16 as u64;
        let m16 = match best_move {
            Some(m) => m.move_int() as u64,
            None => 0,
        };
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

    pub const fn key(self) -> u64 {
        self.0 >> Self::KEY_SHIFT
    }

    pub const fn age(self) -> u8 {
        ((self.0 >> Self::AGE_SHIFT) & Self::AGE_MASK) as u8
    }

    pub const fn depth(self) -> i8 {
        ((self.0 >> Self::DEPTH_SHIFT) & Self::DEPTH_MASK) as u8 as i8
    }

    pub const fn bound(self) -> Bound {
        unsafe { core::mem::transmute(((self.0 >> Self::BOUND_SHIFT) & Self::BOUND_MASK) as u8) }
    }

    pub const fn value(self) -> i32 {
        ((self.0 >> Self::VALUE_SHIFT) & Self::VALUE_MASK) as u16 as i16 as i32
    }

    pub const fn best_move(self) -> Option<Move> {
        match (self.0 & Self::MOVE_MASK) as u16 {
            0 => None,
            m => Some(Move::from_int(m)),
        }
    }

    pub const fn with_value(self, value: i32) -> Self {
        let value16 = (value as i16 as u16 as u64) << Self::VALUE_SHIFT;
        let cleared = self.0 & !(Self::VALUE_MASK << Self::VALUE_SHIFT);
        Self(cleared | value16)
    }

    const fn with_age(self, age: u8) -> Self {
        let age6 = ((age & Self::AGE_MASK as u8) as u64) << Self::AGE_SHIFT;
        let cleared = self.0 & !(Self::AGE_MASK << Self::AGE_SHIFT);
        Self(cleared | age6)
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

const BUCKET_SIZE: usize = 4;

// Half a cache line, so a bucket never straddles lines and one prefetch
// covers all of its entries.
#[repr(align(32))]
struct Bucket([AtomicU64; BUCKET_SIZE]);

impl Bucket {
    fn probe(&self, key: u64, age: u8) -> Option<TTEntry> {
        self.0.iter().find_map(|aentry| {
            let data = aentry.load(Ordering::Relaxed);
            let entry = TTEntry(data);
            (data != 0 && entry.key() == key).then(|| {
                // Refresh the raw word: the caller rebases mate values to
                // its ply, and those must not be written back.
                if entry.age() != age {
                    aentry.store(entry.with_age(age).0, Ordering::Relaxed);
                }
                entry
            })
        })
    }

    fn find(&self, key: u64) -> Option<&AtomicU64> {
        self.0.iter().find(|aentry| {
            let data = aentry.load(Ordering::Relaxed);
            data != 0 && TTEntry(data).key() == key
        })
    }
}

pub struct TT {
    table: Vec<Bucket>,
    age: u8,
}

impl TT {
    pub fn new(megabytes: usize) -> Self {
        let upper_limit = megabytes * 1024 * 1024 / size_of::<Bucket>() + 1;
        let count = upper_limit.next_power_of_two() / 2;
        let table = (0..count)
            .map(|_| Bucket([const { AtomicU64::new(0) }; BUCKET_SIZE]))
            .collect();

        TT { table, age: 0 }
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
        let hash = board.hash();
        let bucket = self.bucket(hash);

        let slot = match bucket.find(hash >> TTEntry::KEY_SHIFT) {
            Some(aentry) => {
                let entry = TTEntry(aentry.load(Ordering::Relaxed));
                if !self.should_replace(entry, depth, bound) {
                    return;
                }
                aentry
            }
            // None sorts below Some, so empty slots are the preferred victims.
            None => bucket
                .0
                .iter()
                .min_by_key(|aentry| {
                    let data = aentry.load(Ordering::Relaxed);
                    (data != 0).then(|| self.quality(TTEntry(data)))
                })
                .unwrap(),
        };

        if value.is_checkmate() {
            value += value.signum() * ply as i32;
        }

        slot.store(
            TTEntry::new(hash, value, best_move, depth, bound, self.age).0,
            Ordering::Relaxed,
        );
    }

    pub fn get(&self, board: &Board, ply: usize) -> Option<TTEntry> {
        let hash = board.hash();

        self.bucket(hash)
            .probe(hash >> TTEntry::KEY_SHIFT, self.age)
            .map(|entry| {
                let value = entry.value();
                if value.is_checkmate() {
                    entry.with_value(value - value.signum() * ply as i32)
                } else {
                    entry
                }
            })
    }

    fn bucket(&self, hash: u64) -> &Bucket {
        &self.table[(hash as usize) & (self.table.len() - 1)]
    }

    fn should_replace(&self, entry: TTEntry, depth: i8, bound: Bound) -> bool {
        bound == Bound::Exact
            || self.age != entry.age()
            || depth >= entry.depth() - Self::DEPTH_MARGIN
    }

    fn quality(&self, entry: TTEntry) -> i32 {
        let relative_age = (self.age.wrapping_sub(entry.age()) & TTEntry::AGE_MASK as u8) as i32;
        entry.depth() as i32 - Self::AGE_PENALTY * relative_age
    }

    pub fn clear(&self) {
        self.table
            .iter()
            .flat_map(|bucket| bucket.0.iter())
            .for_each(|entry| entry.store(0, Ordering::Relaxed));
    }

    pub fn age_up(&mut self) {
        self.age = (self.age + 1) & TTEntry::AGE_MASK as u8;
    }

    pub fn hashfull(&self) -> usize {
        // Sample the first 1000 entries to estimate how full the table is.
        self.table
            .iter()
            .take(1000 / BUCKET_SIZE)
            .flat_map(|bucket| bucket.0.iter())
            .filter(|&aentry| {
                let data = aentry.load(Ordering::Relaxed);
                data != 0 && TTEntry(data).age() == self.age
            })
            .count()
    }

    #[allow(unused_variables)]
    pub fn prefetch(&self, board: &Board) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let ptr = self.bucket(board.hash()) as *const Bucket as *const i8;
            x86_64::_mm_prefetch(ptr, x86_64::_MM_HINT_T0);
        }
    }
}

impl TT {
    const AGE_PENALTY: i32 = 8;
    const DEPTH_MARGIN: i8 = 2;
}
