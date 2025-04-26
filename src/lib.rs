//! # Gauze
//!
//! A collection of probabilistic set membership filters with a simple interface.
//! These filters can claim that a given entry is
//!
//! * definitely not represented in a set of entries, or
//! * might be represented in the set.
//!
//! This crate is a work in progress and currently only implements Bloom filters.
//!
//! If needed, the extension trait `DynFilter` enables object-safe,
//! dynamically dispatched variants of `insert` and `contains`.
//!
//! ## Gauze in Action
//! A simple Bloom filter implementation looks like this:
//!
//! ```rust
//! use gauze::{BloomFilter, Filter};
//!
//! fn main() {
//!     // The number of items we want the `BloomFilter` to store
//!     // while not returning too many false positives
//!     let capacity = 100_000;
//!     // The rate of false positives the `BloomFilter` is allowed
//!     // to return if it stores no more than `capacity` items
//!     let target_err_rate = 0.001;
//!     // These parameters allow us to construct a `BloomFilter` with
//!     // *approximately* optimal properties
//!     let mut bloom =
//!         BloomFilter::new(capacity, target_err_rate)
//!         .expect("couldn't construct Bloom filter.");
//!
//!     // `BloomFilter`s can add any type that is `impl Hash`
//!     bloom.insert(1);
//!     bloom.insert("a");
//!     bloom.insert(Vec::<bool>::new());
//!     bloom.insert([0; 2]);
//!
//!     // Querying whether a `BloomFilter` contains an element
//!     // never yields a false negative
//!     let inserts = capacity - 4;
//!     for i in 0..inserts {
//!         bloom.insert(i);
//!     }
//!
//!     let mut false_negatives = 0;
//!     for i in 0..inserts {
//!         if !bloom.might_contain(i) {
//!             false_negatives += 1;
//!         }
//!     }
//!     println!("False negatives: {false_negatives}");
//!
//!     // But it can yield some false positives
//!     let mut false_positives = 0;
//!     for i in 0..inserts {
//!         if bloom.might_contain(inserts + i) {
//!             false_positives += 1;
//!         }
//!     }
//!     println!("False positives: {false_positives}");
//!
//!     // It is possible to get an *approximation* of the number of
//!     // `item`s stored in the `BloomFilter`
//!     let stored_items_approx = bloom.count_approx();
//!     println!("Approximately count of items stored: {stored_items_approx}");
//!
//!     // Items can't be removed. But the `BloomFilter` can be reset.
//!     bloom.reset();
//!
//!     // We can also get some properties of the `BloomFilter` itself
//!     println!("Number of bits for the actual filter: {}", bloom.bit_count());
//!     println!("Number of bits for the actual filter: {}", bloom.bit_size());
//!     println!("Number of hash functions used: {}", bloom.hash_fn_count());
//!     println!("The filter's actual error rate: {}", bloom.error_rate());
//! }
//! ```

#![deny(warnings)]
#![warn(unused_imports)]
#![warn(missing_docs)]
#![warn(unused_crate_dependencies)]

use core::hash::{Hash, Hasher};
use thiserror::Error;

/// An error returned by a method provided by the `Filter` trait.
#[derive(Error, Debug)]
pub enum FilterError {
    /// A method is called with invalid parameters.
    #[error("invalid parameters (expected {expected:?}, found: {found:?})")]
    InvalidParameter {
        /// Expected parameter
        expected: &'static str,
        /// Provided parameter
        found: String,
    },
    /// Misrepresentation of a filter characteristic through a lossy cast of numerical values.
    #[error("mismatched numerical sizes (casted {argument:?} of value {value:?} into usize)")]
    ConversionError {
        /// Name of casted argument
        argument: &'static str,
        /// Value of casted argument
        value: f64,
    },
}

/// A wrapper to create an object-safe Hash trait.
pub trait DynHash {
    /// Wraps the `.hash()` method for dynamic dispatch.
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

/// Implement the wrapper for all suitable types.
impl<T: Hash + ?Sized> DynHash for T {
    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}

impl Hash for dyn DynHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

/// The common interface of methods shared between all `gauze` filters.
///
/// Every filter adds their own constructor and possibly other methods based
/// on their unique characteristics.
pub trait Filter {
    /// Inserts the `item` into the filter.
    fn insert(&mut self, item: impl Hash);

    /// *Indicates* whether `item` is in the filter.
    ///
    /// Never yields false negatives.
    /// Yields false positives roughly at the rate of the filter's `error_rate`.
    fn might_contain(&self, item: impl Hash) -> bool;

    /// Resets the filter to its empty state.
    fn reset(&mut self) -> &mut Self;
}

/// An extension trait to `Filter`.
///
/// It adds dynamically dispatched alternatives for the `insert` and `contains` methods.
pub trait DynFilter {
    /// Inserts the `item` into the filter.
    fn insert(&mut self, item: Box<dyn DynHash>);

    /// *Indicates* whether `item` is in the filter.
    ///
    /// Never yields false negatives.
    /// Yields false positives roughly at the rate of the filter's `error_rate`.
    fn might_contain(&self, item: Box<dyn DynHash>) -> bool;
}

mod bloom;
mod utils;
pub use bloom::BloomFilter;
