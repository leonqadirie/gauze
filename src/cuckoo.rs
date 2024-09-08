use std::{cmp, hash::Hasher, iter::repeat};
use thiserror::Error;
use twox_hash::XxHash64;

use crate::{DynFilter, Filter};

const BUCKET_SIZE: usize = 4;
const EMPTY_FINGERPRINT: Fingerprint = Fingerprint { value: 0 };
const MAX_REBUCKET: u16 = 500;

//TODO: Check implications of using a byte.
// Paper suggests 6bits as optimal for typical workloads, but SIMD might prefer 8.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Fingerprint {
    value: u8,
}

impl Fingerprint {
    /// Creates a `Fingerprint` based on a `XxHash64`.
    #[allow(dead_code)]
    fn create(hash: XxHash64) -> Self {
        // Uses least significant bits for the `Fingerprint`.
        // Ensures it is never 0.
        let value = (hash.finish() % 255 + 1) as u8;

        Self { value }
    }

    /// Checks whether a `Fingerprint` is set to `0`.
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.value == 0
    }

    /// Returns the `Fingerprint`'s value.
    #[allow(dead_code)]
    fn value(&self) -> u8 {
        self.value
    }
}

/// An error returned by calling a method on a `Bucket`.
#[derive(Error, Debug)]
pub enum BucketError {
    #[error("Bucket full, couldn't insert fingerprint: {fingerprint}")]
    BucketFull { fingerprint: u8 },
    #[error("Bucket doesn't contain fingerprint: {fingerprint}")]
    FingerprintNotFound { fingerprint: u8 },
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Bucket {
    pub slots: [Fingerprint; BUCKET_SIZE],
}

impl Bucket {
    /// Constructs a new `Bucket` with preallocated memory of empty `Fingerprint`s.
    fn new() -> Self {
        Self {
            slots: [EMPTY_FINGERPRINT; BUCKET_SIZE],
        }
    }

    /// Attempts to insert `fingerprint` into the `Bucket`.
    /// Fails if no empty slot is found.
    fn try_insert(&mut self, fingerprint: Fingerprint) -> Result<(), BucketError> {
        for slot in &mut self.slots {
            if slot.is_empty() {
                *slot = fingerprint;
                return Ok(());
            }
        }
        Err(BucketError::BucketFull {
            fingerprint: fingerprint.value,
        })
    }

    /// Attempts to delete `fingerprint` from the `Bucket`.
    /// Fails if it wasn't in the bucket.
    fn try_delete(&mut self, fingerprint: Fingerprint) -> Result<(), BucketError> {
        match self.slots.iter().position(|fp| *fp == fingerprint) {
            Some(idx) => {
                self.slots[idx] = EMPTY_FINGERPRINT;
                Ok(())
            }
            None => Err(BucketError::FingerprintNotFound {
                fingerprint: fingerprint.value,
            }),
        }
    }

    /// Resets the bucket.
    #[inline(always)]
    fn reset(&mut self) {
        *self = Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct CuckooFilter {
    filter: Box<[Bucket]>,
}

impl Filter for CuckooFilter {
    fn insert(&mut self, item: impl std::hash::Hash) {
        todo!()
    }

    fn contains(&self, item: impl std::hash::Hash) -> bool {
        todo!()
    }

    fn reset(&mut self) -> &mut Self {
        todo!()
    }
}

impl DynFilter for CuckooFilter {
    fn insert(&mut self, item: Box<dyn crate::DynHash>) {
        todo!()
    }

    fn contains(&self, item: Box<dyn crate::DynHash>) -> bool {
        todo!()
    }
}

impl CuckooFilter {
    /// Constructs a new `CuckooFilter`.
    ///
    /// * `capacity`: Intended elements the Cuckoo filter shall be able to hold
    /// * `target_err_rate`: The Cuckoo filter's acceptable false positive rate
    ///
    /// Fails for invalid parameters or if filter is too large for your architecture.
    pub fn new(capacity: usize) -> Self {
        let buckets = cmp::max(1, capacity.next_power_of_two() / BUCKET_SIZE);

        Self {
            filter: repeat(Bucket::new()).take(buckets).collect(),
        }
    }

    /// Returns the `CuckoFilter`'s error_rate
    pub fn error_rate(&self) -> f64 {
        // error_rate = (BUCKET_SIZE * hash_fn_count)/2^FINGERPRINT_SIZE = (BUCKET_SIZE*2)/256
        (BUCKET_SIZE as f64 * 2.0) / 256.0
    }
}
