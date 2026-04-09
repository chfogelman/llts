use super::*;

#[test]
fn test_compute_max() {
    assert_eq!(Log2Histogram::<6, 10>::compute_true_max(2), 5);
    assert_eq!(Log2Histogram::<10, 10>::compute_true_max(2), 11);
    assert_eq!(Log2Histogram::<11, 10>::compute_true_max(2), 13);
    assert_eq!(Log2Histogram::<9, 8>::compute_true_max(0), 1023);
    assert_eq!(Log2Histogram::<9, 8>::compute_true_max(1), 47);
    assert_eq!(Log2Histogram::<2048, 10>::compute_true_max(4), u64::MAX);
}

#[test]
fn test_precision_bits() {
    assert_eq!(Log2Histogram::<9, 10>::compute_precision_bits(), 1);
    assert_eq!(Log2Histogram::<10, 10>::compute_precision_bits(), 2);
    assert_eq!(Log2Histogram::<256, 10_000_000>::compute_precision_bits(), 3);
    assert_eq!(Log2Histogram::<9, 7>::compute_precision_bits(), 63);
}

#[test]
fn test_calc_bin() {
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(0), Some(0));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(1), Some(1));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(2), Some(2));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(3), Some(3));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(5), Some(5));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(8), Some(8));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(9), Some(8));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(10), Some(9));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(11), Some(9));
    assert_eq!(Log2Histogram::<10, 10>::get_bin_idx(1000), None);
}

#[test]
fn test_add() {
    let mut histogram = (0..=9).collect::<Log2Histogram<10, 10>>();

    assert_eq!(histogram.bins[0].count, 1);

    assert_eq!(histogram.bins[8].count, 2);
    assert_eq!(histogram.bins[8].max, 9);
    //assert_eq!(histogram.bins[8].min, 8);

    histogram.add(10).unwrap();
    histogram.add(10).unwrap();

    assert_eq!(histogram.bins[9].count, 2);
    assert_eq!(histogram.bins[9].max, 10);
}

#[test]
fn test_percentiles_basic() {
    let histogram = (0..100).collect::<Log2Histogram<100, 1>>();
    let pcs = histogram.compute_percentiles(&[0.25, 0.50, 0.75, 0.99]);
    assert_eq!(pcs, [25, 50, 75, 99]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_percentiles_mul() {
    let mut histogram = Log2Histogram::<2048, 10_000_000>::new();
    for x in 0..1_000_000 {
        histogram.add_all((1..=10).map(|y| x * y));
    }
    // True vals:  [853541, 2099413, 4138897, 8999991, 9899990]
    let expected = [853541, 2099414, 4138900, 9018667, 9899994];
    assert_eq!(
        histogram.compute_percentiles(&[0.25, 0.50, 0.75, 0.99, 0.999]),
        expected
    );
    assert_eq!(histogram.mean(), 2749997.25);

    // True val:                2295828.653957477
    assert_eq!(histogram.std(), 2295828.6539659696);
}

#[test]
fn test_percentiles_exp() {
    let mut histogram = Log2Histogram::<2048, 100_000_000>::new();
    for x in 0..1000u64 {
        histogram.add_all((1..=3).map(|y| x.pow(y)));
    }
    // True vals:  [713, 194796, 15484936, 909881407, 988050915]
    let expected = [713, 195112, 15625000, 910795077, 991026973];
    assert_eq!(
        histogram.compute_percentiles(&[0.25, 0.50, 0.75, 0.99, 0.999]),
        expected
    );
    assert_eq!(histogram.mean(), 83277861.0);
    assert_eq!(histogram.std(), 201289373.7545);
}
