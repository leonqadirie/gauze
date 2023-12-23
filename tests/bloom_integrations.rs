#[cfg(test)]
mod tests {
    use gauze::{BloomFilter, Filter};

    #[test]
    fn it_works() {
        let capacity = 1_003;
        let target_err_rate = 0.001;
        let mut bloom =
            BloomFilter::new(capacity, target_err_rate).expect("couldn't construct Bloom filter.");
        let a = "a";
        let b = Vec::<bool>::new();
        let c = [0; 2];

        let add_count = capacity - 3;

        for i in 0..add_count {
            bloom.add(i);
        }

        bloom.add(a);
        bloom.add(&b);
        bloom.add(c);

        assert!(bloom.contains(a) == true);
        assert!(bloom.contains(b) == true);
        assert!(bloom.contains(c) == true);
        for i in 0..add_count {
            assert!(bloom.contains(i) == true);
        }
    }
}
