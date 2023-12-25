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
//!         if !bloom.contains(i) {
//!             false_negatives += 1;
//!         }
//!     }
//!     println!("False negatives: {false_negatives}");
//!
//!     // But it can yield some false positives
//!     let mut false_positives = 0;
//!     for i in 0..inserts {
//!         if bloom.contains(inserts + i) {
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
//!     println!("Number of hash functions used: {}", bloom.hash_fn_count());
//!     println!("The filter's actual error rate: {}", bloom.error_rate());
//! }
//! ```

use core::hash::Hash;
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
}

pub trait Filter {
    fn insert(&mut self, item: impl Hash) -> &mut Self;
    fn contains(&self, item: impl Hash) -> bool;
    fn count_approx(&self) -> usize;
    fn reset(&mut self) -> &mut Self;
    fn bit_count(&self) -> usize;
    fn error_rate(&self) -> f64;
    fn hash_fn_count(&self) -> usize;
}

pub mod bloom;
pub use bloom::BloomFilter;
