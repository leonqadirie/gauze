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

        assert!(bloom.might_contain(a) == true);
        assert!(bloom.might_contain(b) == true);
        assert!(bloom.might_contain(c) == true);
        for i in 0..inserts {
            assert!(bloom.might_contain(i) == true);
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

        assert!(bloom.might_contain(Box::new(a)) == true);
        assert!(bloom.might_contain(Box::new(b)) == true);
        assert!(bloom.might_contain(Box::new(c)) == true);
        for i in 0..inserts {
            assert!(bloom.might_contain(Box::new(i)) == true);
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
