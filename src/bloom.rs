use bitvec::{bitvec, prelude::*};
use rand::random;
use std::{
    hash::{Hash, Hasher},
    sync::OnceLock,
};
use twox_hash::XxHash64;

use crate::Filter;
use crate::FilterError;
use crate::FilterError::InvalidParameter;

static SEED: OnceLock<u64> = OnceLock::new();
static OPTIMIZATION_STEP: f64 = 1.01;

/// A Bloom filter is a space-efficient probabilistic data structure to test
/// whether an item is a member of a set.
///
/// It never returns false negatives but may return false positives.
/// Items can only be added, not deleted.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct BloomFilter {
    filter: BitVec,
    error_rate: f64,
    hash_fn_count: usize,
    bit_count: usize,
}

impl Filter for BloomFilter {
    /// Inserts the `item` into the `BloomFilter`.
    fn insert(&mut self, item: impl Hash) -> &mut Self {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            self.filter.set(idx as usize, true);
        }

        self
    }

    /// *Indicates* whether `item` is in the `BloomFilter`.
    ///
    /// Never yields false negatives.
    /// Yields false positives roughly at the rate of the `Bloomfilter`'s `error_rate`.
    fn contains(&self, item: impl Hash) -> bool {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            if self.filter.get(idx).expect("No bit at index.") == false {
                return false;
            }
        }

        true
    }

    /// Returns an *approximation* of the number of elements added to the `BloomFilter`.
    fn count_approx(&self) -> usize {
        let num_truthy_bits = self.filter.iter_ones().count();
        approximate_elems(self.bit_count, self.hash_fn_count, num_truthy_bits).round() as usize
    }

    // Resets the `BloomFilter` to its empty state.
    fn reset(&mut self) -> &mut Self {
        self.filter = bitvec![usize, Lsb0; 0; self.bit_count as usize];

        self
    }

    /// Returns the amount of bits that constitute the `BloomFilter`'s actual `filter`.
    fn bit_count(&self) -> usize {
        self.bit_count
    }

    /// Returns the `BloomFilter`'s actual error rate.
    fn error_rate(&self) -> f64 {
        self.error_rate
    }

    // Returns the number of hash functions the `BloomFilter` uses.
    fn hash_fn_count(&self) -> usize {
        self.hash_fn_count
    }
}

impl BloomFilter {
    /// Constructs a new `BloomFilter`.
    ///
    /// * `capacity`: Intended elements the Bloom filter shall be able to hold
    /// * `target_err_rate`: The Bloom filter's acceptable false positive rate
    pub fn new(capacity: usize, target_err_rate: f64) -> Result<BloomFilter, FilterError> {
        if capacity < 1 {
            return Err(InvalidParameter {
                expected: "1 <= capacity",
                found: capacity.to_string(),
            });
        }
        if target_err_rate <= 0.0 || 1.0 <= target_err_rate {
            return Err(InvalidParameter {
                expected: "0.0 < error rate < 1.0",
                found: target_err_rate.to_string(),
            });
        }

        SEED.get_or_init(|| random::<u64>());

        let (bit_count, hash_fn_count, error_rate) = optimize(capacity, target_err_rate);
        let filter = bitvec![usize, Lsb0; 0; bit_count as usize];

        Ok(BloomFilter {
            bit_count,
            hash_fn_count,
            filter,
            error_rate,
        })
    }

    /// Calculates the indexes of a `BloomFilter`'s `filter` field of type `BitVec` for the `item`.
    fn get_bit_indexes<T>(&self, item: T) -> Vec<usize>
    where
        T: Hash,
    {
        // Kirsch-Mitzenmacher double hashing
        let mut hasher_1 = XxHash64::default();
        let mut hasher_2 = XxHash64::with_seed(*SEED.get().expect("couldn't get seed."));

        item.hash(&mut hasher_1);
        item.hash(&mut hasher_2);

        let hash_1 = hasher_1.finish();
        let hash_2 = hasher_2.finish();

        let mut acc = vec![];
        for i in 0..self.hash_fn_count {
            let idx = ((hash_1).wrapping_add((i as u64).wrapping_mul(hash_2))
                % self.bit_count as u64) as usize;
            acc.push(idx);
        }
        acc
    }
}

/// Proxy function that relays the input to the recursive function `optimize_values`.
/// Used in Bloom filter construction to optimize filter properties.
///
/// * `capacity`: Intended elements the Bloom filter shall be able to hold
/// * `target_err_rate`: The Bloom filter's acceptable false positive rate
///
/// Returns *approximately* optimal (num_bits, hash_fn_count, error_rate).
fn optimize(capacity: usize, target_err_rate: f64) -> (usize, usize, f64) {
    let (num_bits, hash_fn_count, error_rate) =
        optimize_values(capacity as f64, capacity as f64 * 4.0, 2.0, target_err_rate);

    (num_bits as usize, hash_fn_count as usize, error_rate)
}

/// Recursive function to *approximate* optimal Bloom filter properties.
/// Evaluates filter properties for the input parameters and optimizes them if needed.
/// Used in Bloom filter construction.
///
/// * `capacity`: Intended elements the Bloom filter shall be able to hold
/// * `bits`: The number of bits that constitute the filter
/// * `hash_fns_count`: The number of hash functions the filter uses
/// * `target_err_rate`: The Bloom filter's acceptable false positive rate
///
/// Returns *approximately* optimal (num_bits, hash_fn_count, error_rate).
fn optimize_values(
    capacity: f64,
    bits: f64,
    hash_fns_count: f64,
    target_error_rate: f64,
) -> (usize, usize, f64) {
    let error_rate = false_positive_rate(bits, capacity, hash_fns_count);

    if bits.is_infinite() {
        return (bits as usize, hash_fns_count.ceil() as usize, error_rate);
    }

    let is_acceptable_error_rate = error_rate < target_error_rate;
    if !is_acceptable_error_rate {
        optimize_values(
            capacity,
            (bits * OPTIMIZATION_STEP).ceil(),
            optimal_hash_fn_count((bits * OPTIMIZATION_STEP).ceil(), capacity),
            target_error_rate,
        )
    } else {
        (bits as usize, hash_fns_count.ceil() as usize, error_rate)
    }
}

/// Calculates the false positive rate of a Bloom filter with the properties of the parameters.
/// Used in filter construction.
///
/// * `bits`: The number of bits that constitute the filter
/// * `capacity`: The number of elements that the filter shall be to hold
/// * `hash_fns_count`: The number of hash functions the filter uses
///
/// Returns an `f64` as the expected false positive rate.
fn false_positive_rate(bits: f64, capacity: f64, hash_fns_count: f64) -> f64 {
    (1.0 - (-hash_fns_count * (capacity + 0.5) / (bits - 1.0)).exp()).powf(hash_fns_count)
}

/// Calculates the optimal number of hash functions
fn optimal_hash_fn_count(bits: f64, capacity: f64) -> f64 {
    (bits / capacity) * 2_f64.ln()
}

/// Approximates the number of items in the filter
fn approximate_elems(bits: usize, hash_fns_count: usize, num_truthy_bits: usize) -> f64 {
    let m = bits as f64;
    let x = num_truthy_bits as f64;
    let k = hash_fns_count as f64;

    -1.0 * (m * (1.0 - x / m).log(std::f64::consts::E)) / k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bloom_filter() {
        let capacity = 100;
        let target_err_rate = 0.001;
        let bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter");

        assert_eq!(1449, bloom.bit_count());
        assert_eq!(11, bloom.hash_fn_count());
        assert_eq!(0.0009855809404929945, bloom.error_rate());
    }

    #[test]
    fn test_new_bloom_filter_wrong_parameters() {
        let wrong_capacity = 0;
        let wrong_target_err_rate_1 = 0.0;
        let wrong_target_err_rate_2 = 1.0;
        let wrong_target_err_rate_3 = -1.0;
        let correct_capacity = 1;
        let correct_target_err_rate = 0.5;

        assert!(BloomFilter::new(wrong_capacity, wrong_target_err_rate_1).is_err());
        assert!(BloomFilter::new(wrong_capacity, correct_target_err_rate).is_err());
        assert!(BloomFilter::new(correct_capacity, wrong_target_err_rate_1).is_err());
        assert!(BloomFilter::new(correct_capacity, wrong_target_err_rate_2).is_err());
        assert!(BloomFilter::new(correct_capacity, wrong_target_err_rate_3).is_err());
        assert!(BloomFilter::new(correct_capacity, correct_target_err_rate).is_ok());
    }
}
