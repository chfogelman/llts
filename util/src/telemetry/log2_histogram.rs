#[derive(Clone, Copy, Debug)]
pub struct Bin {
    pub count: u64,
    pub min:   u64,
    pub max:   u64,
}

impl Bin {
    const fn new() -> Self {
        Self {
            count: 0,
            min:   u64::MAX,
            max:   0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Log2Histogram<const BINS: usize, const MAX_VALUE: u64> {
    count:        u64,
    sum:          u64,
    sum_sq:       f64,
    bins:         [Bin; BINS],
    overflow_bin: Bin,
}

impl<const BINS: usize, const MAX_VALUE: u64> Log2Histogram<BINS, MAX_VALUE> {
    pub const PRECISION_BITS: u32 = Self::compute_precision_bits();

    const RANGE_SIZE: u64 = 1u64 << Self::PRECISION_BITS;
    const MASK: u64 = Self::RANGE_SIZE - 1;

    pub fn new() -> Self {
        Self {
            count:        0,
            sum:          0,
            sum_sq:       0.,
            bins:         [Bin::new(); BINS],
            overflow_bin: Bin::new(),
        }
    }

    /// Adds `val` to the histogram state and returns the corresponding bin.
    ///
    /// If the value is above MAX_VALUE, uses the histogram's overflow bin
    pub fn add(&mut self, val: u64) -> Result<&Bin, &Bin> {
        let update_bin = |bin: &mut Bin| {
            bin.count += 1;
            bin.min = bin.min.min(val);
            bin.max = bin.max.max(val);
        };
        self.count += 1;
        self.sum += val;
        self.sum_sq += (val * val) as f64;
        if let Some(idx) = Self::get_bin_idx(val) {
            let bin = &mut self.bins[idx];
            update_bin(bin);
            Ok(bin)
        } else {
            let bin = &mut self.overflow_bin;
            update_bin(bin);
            Err(bin)
        }
    }

    /// Adds all the values in the iterator to the histogram state.
    ///
    /// NOTE: does not notify caller if overflow bin is used
    pub fn add_all(&mut self, vals: impl IntoIterator<Item = u64>) {
        for val in vals {
            let _ = self.add(val);
        }
    }

    /// Computes percentiles. Inputs must be floating-point values between 0 and 1.
    pub fn compute_percentiles<const N: usize>(&self, pcts: &[f64; N]) -> [u64; N] {
        assert!(N > 0, "Must provide percentiles");
        assert!(pcts.is_sorted(), "Desired percentiles must be provided in sorted order!");
        let mut vals = [0; N];
        let mut tally = 0;
        let mut pi = 0;
        'outer: for bin in &self.bins {
            // There may be multiple percentiles filled out by a bin.
            //
            // To eke a bit more accuracy out of the approximate histogram,
            // we can linearly interpolate between the min and max value.
            // `limit` is the number of items that constitutes the percentile,
            // This bin's `min` corresponds to the current tally,
            // and `max` to tally + bin.count
            // (x_0, y_0) = (tally, min)
            // (x_1, y_1) = (tally + bin.count, max)
            // x = limit
            while let limit = (pcts[pi] * self.count as f64) as u64
                && tally <= limit
                && limit < tally + bin.count
            {
                vals[pi] = ((tally + bin.count - limit) * bin.min + (limit - tally) * bin.max) / bin.count;
                pi += 1;
                if pi == vals.len() {
                    break 'outer;
                }
            }
            tally += bin.count;
        }
        vals
    }

    pub fn mean(&self) -> f64 {
        self.sum as f64 / self.count as f64
    }

    pub fn var(&self) -> f64 {
        ((self.sum_sq) / self.count as f64) - self.mean() * self.mean()
    }

    pub fn std(&self) -> f64 {
        self.var().sqrt()
    }

    fn get_bin_idx(val: u64) -> Option<usize> {
        if val <= Self::RANGE_SIZE {
            Some(val as usize)
        } else {
            let bits = val.ilog2() as u64;
            let shift = bits.saturating_sub(Self::PRECISION_BITS as u64);
            let offset = (val >> shift) & Self::MASK;
            let bin = (offset + ((shift + 1) << Self::PRECISION_BITS)) as usize;
            if bin < BINS { Some(bin) } else { None }
        }
    }

    /// Computes the number of precision bits possible given the number of bins and the max value
    ///
    /// Panics if the number of bins is too small to support `MAX_VALUE` with any number of
    /// precision bits
    const fn compute_precision_bits() -> u32 {
        // Special case:
        // If we can give each point up to the requested max its own bin, then do so.
        if MAX_VALUE < BINS as u64 {
            return 63;
        }

        // I imagine this computation has a closed-form solution. I am, regrettably, too
        // smooth-brained to find it. Instead, I simply do a binary search to find the maximum
        // number of precision bits that doesn't prevent storing the provided maximum value
        let (mut lo, mut hi) = (0, 64);
        assert!(Self::compute_true_max(lo) > MAX_VALUE, "Insufficient histogram size");
        loop {
            if hi - lo <= 1 {
                break lo;
            }
            let mid = lo + (hi - lo) / 2;
            // This is a decreasing function, so the binary search proceeds in reverse order.
            if Self::compute_true_max(mid) >= MAX_VALUE {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }

    const fn compute_true_max(precision_bits: u32) -> u64 {
        let range_size = 1 << precision_bits;
        let num_ranges = BINS as u64 / range_size;
        let tail_bins = BINS as u64 % range_size;
        if num_ranges == 0 {
            return 1;
        } else if num_ranges + 1 >= u64::BITS as u64 {
            return u64::MAX;
        }
        let tail_bins_max = 1 << (num_ranges + 1);
        let tail_bin_size = 1 << (num_ranges + 1).saturating_sub(precision_bits as u64);
        tail_bins_max + tail_bins * tail_bin_size - 1
    }
}

impl<const BINS: usize, const MAX_VALUE: u64> Default for Log2Histogram<BINS, MAX_VALUE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const BINS: usize, const MAX_VALUE: u64> FromIterator<u64> for Log2Histogram<BINS, MAX_VALUE> {
    fn from_iter<T: IntoIterator<Item = u64>>(iter: T) -> Self {
        let mut histogram = Self::new();
        for point in iter {
            let _ = histogram.add(point);
        }
        histogram
    }
}

#[cfg(test)]
mod test;
