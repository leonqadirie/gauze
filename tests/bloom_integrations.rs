#[cfg(test)]
mod tests {
    use gauze::BloomFilter;

    #[test]
    fn test_it_works() {
        use gauze::Filter;

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
    fn test_it_works_dynamically() {
        use gauze::DynFilter;

        let capacity = 1_003;
        let target_err_rate = 0.001;
        let mut bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter.");
        let a = "a";
        let b = Vec::<bool>::new();
        let c = [0; 2];

        let inserts = capacity - 3;

        for i in 0..inserts {
            DynFilter::insert(&mut bloom, Box::new(i));
        }

        bloom.insert(Box::new(a));
        bloom.insert(Box::new(b.clone()));
        bloom.insert(Box::new(c));

        assert!(bloom.contains(Box::new(a)) == true);
        assert!(bloom.contains(Box::new(b)) == true);
        assert!(bloom.contains(Box::new(c)) == true);
        for i in 0..inserts {
            assert!(bloom.contains(Box::new(i)) == true);
        }
    }

    #[test]
    fn test_count_approx() {
        use gauze::Filter;

        let capacity = 100;
        let target_err_rate = 0.001;
        let mut bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter");

        let inserts = capacity / 2;

        for i in 0..inserts {
            bloom.insert(i);
        }

        assert!(bloom.count_approx().abs_diff(inserts) < inserts / 15);
    }
}
