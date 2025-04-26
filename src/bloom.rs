use bitvec::{bitvec, prelude::*};
use rand::random;
use std::{
    hash::{Hash, Hasher},
    sync::LazyLock,
};
use twox_hash::XxHash64;

use crate::{utils::float_to_usize, Filter};
use crate::{
    DynFilter, DynHash,
    FilterError::{self, InvalidParameter},
};

static SEED: LazyLock<u64> = LazyLock::new(|| random::<u64>());
const OPTIMIZATION_STEP: f64 = 1.01;

/// A Bloom filter is a space-efficient probabilistic data structure to test
/// whether an item is a member of a set.
///
/// It never returns false negatives but may return false positives.
/// Items can only be added, not deleted.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct BloomFilter {
    array: BitVec,
    false_positive_rate: f64,
    hash_fn_count: usize,
    bit_size: usize,
}

impl Filter for BloomFilter {
    /// Inserts the `item` into the `BloomFilter`.
    fn insert(&mut self, item: impl Hash) {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            self.array.set(idx as usize, true);
        }
    }

    /// *Indicates* whether `item` is in the `BloomFilter`.
    ///
    /// Never yields false negatives.
    /// Yields false positives roughly up to the rate of the `Bloomfilter`'s `error_rate`
    /// unless the filter's maximum capacity defined at construction is exceeded.
    fn might_contain(&self, item: impl Hash) -> bool {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            if self.array.get(idx).expect("No bit at index.") == false {
                return false;
            }
        }

        true
    }

    // Resets the `BloomFilter` to its empty state.
    fn reset(&mut self) -> &mut Self {
        self.array = bitvec![usize, Lsb0; 0; self.bit_size as usize];

        self
    }
}

impl DynFilter for BloomFilter {
    fn insert(&mut self, item: Box<dyn DynHash>) {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            self.array.set(idx as usize, true);
        }
    }

    fn might_contain(&self, item: Box<dyn DynHash>) -> bool {
        let idxes = self.get_bit_indexes(item);
        for idx in idxes {
            if self.array.get(idx).expect("No bit at index.") == false {
                return false;
            }
        }

        true
    }
}

impl BloomFilter {
    /// Constructs a new `BloomFilter`.
    ///
    /// * `capacity`: Intended elements the Bloom filter shall be able to hold
    /// * `target_err_rate`: The Bloom filter's acceptable false positive rate
    ///
    /// Fails for invalid parameters or if filter is too large for your architecture.
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

        let (num_bits, hash_fn_count, error_rate) = optimize(capacity, target_err_rate);
        let bit_count = num_bits?;
        let hash_fn_count = hash_fn_count?;
        let filter = bitvec![usize, Lsb0; 0; bit_count];

        Ok(BloomFilter {
            bit_size: bit_count,
            hash_fn_count,
            array,
            false_positive_rate,
        })
    }

    /// Returns an *approximation* of the number of elements added to the `BloomFilter`.
    pub fn count_approx(&self) -> usize {
        let num_truthy_bits = self.array.iter_ones().count();
        approximate_elems(self.bit_size, self.hash_fn_count, num_truthy_bits).round() as usize
    }

    /// Returns the number of bits that constitute the `BloomFilter`'s actual `filter` field.
    pub fn bit_size(&self) -> usize {
        self.bit_size
    }

    /// Returns the `BloomFilter`'s actual error rate.
    pub fn error_rate(&self) -> f64 {
        self.false_positive_rate
    }

    /// Returns the number of hash functions the `BloomFilter` uses.
    pub fn hash_fn_count(&self) -> usize {
        self.hash_fn_count
    }

    /// Calculates an `item`'s indexes in the `BloomFilter`'s `filter` field.
    ///
    /// * `item`: The item for which the indexes shall be calculated
    ///
    /// This can be used for insertion or to check if its likely included.
    fn get_bit_indexes<T>(&self, item: T) -> Vec<usize>
    where
        T: Hash,
    {
        // Kirsch-Mitzenmacher double hashing
        let mut hasher_1 = XxHash64::default();
        let mut hasher_2 = XxHash64::with_seed(*SEED);

        item.hash(&mut hasher_1);
        item.hash(&mut hasher_2);

        let hash_1 = hasher_1.finish();
        let hash_2 = hasher_2.finish();

        let mut acc = vec![];
        for i in 0..self.hash_fn_count {
            let idx = ((hash_1).wrapping_add((i as u64).wrapping_mul(hash_2))
                % self.bit_size as u64) as usize;
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
fn optimize(
    capacity: usize,
    target_err_rate: f64,
) -> (Result<usize, FilterError>, Result<usize, FilterError>, f64) {
    let (num_bits, hash_fn_count, error_rate) =
        optimize_values(capacity as f64, capacity as f64 * 4.0, 2.0, target_err_rate);

    let num_bits = float_to_usize(num_bits, stringify!(num_bits));
    let hash_fn_count = float_to_usize(hash_fn_count, stringify!(hash_fn_count));

    (num_bits, hash_fn_count, error_rate)
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
/// Returns an *approximately* optimal `(num_bits, hash_fn_count, error_rate)`.
fn optimize_values(
    capacity: f64,
    num_bits: f64,
    hash_fns_count: f64,
    target_error_rate: f64,
) -> (f64, f64, f64) {
    let error_rate = false_positive_rate(num_bits, capacity, hash_fns_count);

    if num_bits == f64::MAX || num_bits.is_infinite() {
        return (num_bits, hash_fns_count.ceil(), error_rate);
    }

    let is_acceptable_error_rate = error_rate < target_error_rate;
    if !is_acceptable_error_rate {
        optimize_values(
            capacity,
            (num_bits * OPTIMIZATION_STEP).ceil(),
            optimal_hash_fn_count((num_bits * OPTIMIZATION_STEP).ceil(), capacity),
            target_error_rate,
        )
    } else {
        (num_bits, hash_fns_count.ceil(), error_rate)
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
    let k = hash_fns_count as f64;
    let x = num_truthy_bits as f64;

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

        assert_eq!(1449, bloom.bit_size());
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

    #[test]
    fn test_new_bloom_filter_capacity_too_large() {
        let bloom = BloomFilter::new(usize::MAX, 0.999);

        assert!(bloom.is_err());
    }
    #[test]
    fn test_false_positive_rate() {
        let bits = 127.0;
        let capacity = 10.0;
        let hash_fn_count = 12.3;

        let false_positive_rate = false_positive_rate(bits, capacity, hash_fn_count);
        println!("{false_positive_rate}");

        assert_eq!(false_positive_rate, 0.004227169523530584);
    }

    #[test]
    fn test_optimal_hash_fn_count() {
        let bits = 127.0;
        let capacity = 10.0;

        let optimal_hash_fn_count = optimal_hash_fn_count(bits, capacity);
        assert_eq!(optimal_hash_fn_count, 8.802969193111304);
    }

    #[test]
    fn test_approximate_elems() {
        let m = 100;
        let k = 9;
        let x = 50;

        let elems_count = approximate_elems(m, k, x);
        assert_eq!(elems_count, 7.701635339554948);
    }
}
