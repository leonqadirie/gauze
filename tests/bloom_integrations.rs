#[cfg(test)]
mod tests {
    use gauze::{BloomFilter, Filter};

    #[test]
    fn test_it_works() {
        let capacity = 1_003;
        let target_err_rate = 0.001;
        let mut bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter.");
        let a = "a";
        let b = Vec::<bool>::new();
        let c = [0; 2];

        let inserts = capacity - 3;

        for i in 0..inserts {
            bloom.insert(i);
        }

        bloom.insert(a);
        bloom.insert(&b);
        bloom.insert(c);

        assert!(bloom.contains(a) == true);
        assert!(bloom.contains(b) == true);
        assert!(bloom.contains(c) == true);
        for i in 0..inserts {
            assert!(bloom.contains(i) == true);
        }
    }

    #[test]

    fn test_count_approx() {
        let capacity = 100;
        let target_err_rate = 0.001;
        let mut bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter");

        let inserts = capacity / 2;

        for i in 0..inserts {
            bloom.insert(i);
        }

        assert!(bloom.count_approx().abs_diff(inserts) < inserts / 20);
    }
}
