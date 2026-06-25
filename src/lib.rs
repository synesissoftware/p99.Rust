// src/lib.rs : `Histogram`

#[rustfmt::skip]
use std::{
    fmt as std_fmt,
    time as std_time,
};

/// Low-cost performance percentile histogram using 64-buckets.
///
/// # Note:
/// This histogram tracks event durations with nanosecond precision across
/// 64 logarithmic power-of-two spacing buckets. This is extremely efficient
/// and suited for high-frequency low-overhead timing measurements.
#[derive(Clone)]
pub struct Histogram {
    event_count: usize,

    event_time_total: u64,
    has_overflowed: bool,

    min_event_time: Option<u64>,
    max_event_time: Option<u64>,

    buckets: [u64; 64],
}

// API functions

impl Histogram {
    //
}

// Mutating methods

impl Histogram {
    /// Clears the instance, resetting all values to the equivalent of a
    /// newly constructed instance.
    pub fn clear(&mut self) {
        *self = Default::default();
    }

    /// Pushes an event with the given [`std_time::Duration`].
    ///
    /// # Note:
    /// The value obtained from `Duration#as_nanos()` is truncated to
    /// `u64`.
    pub fn push_event_duration(
        &mut self,
        duration: std_time::Duration,
    ) -> bool {
        return self.push_event_time_ns(duration.as_nanos() as u64);
    }

    /// Pushes an event with the given number of nanoseconds.
    pub fn push_event_time_ns(
        &mut self,
        time_in_ns: u64,
    ) -> bool {
        if self.try_add_ns_to_total_and_update_minmax_and_count_(time_in_ns) {
            self.event_count += 1;

            let bucket = Self::bucket_index(time_in_ns);
            self.buckets[bucket] += 1;

            true
        } else {
            false
        }
    }

    /// Pushes an event with the given number of microseconds.
    pub fn push_event_time_us(
        &mut self,
        time_in_us: u64,
    ) -> bool {
        if let Some(time_in_ns) = time_in_us.checked_mul(1_000) {
            let r = self.push_event_time_ns(time_in_ns);

            r
        } else {
            self.has_overflowed = true;

            false
        }
    }

    /// Pushes an event with the given number of milliseconds.
    pub fn push_event_time_ms(
        &mut self,
        time_in_ms: u64,
    ) -> bool {
        if let Some(time_in_ns) = time_in_ms.checked_mul(1_000_000) {
            let r = self.push_event_time_ns(time_in_ns);

            r
        } else {
            self.has_overflowed = true;

            false
        }
    }

    /// Pushes an event with the given number of seconds.
    pub fn push_event_time_s(
        &mut self,
        time_in_s: u64,
    ) -> bool {
        if let Some(time_in_ns) = time_in_s.checked_mul(1_000_000_000) {
            let r = self.push_event_time_ns(time_in_ns);

            r
        } else {
            self.has_overflowed = true;

            false
        }
    }
}

// Non-mutating methods

impl Histogram {
    /// Returns the count of events in a specific bucket.
    pub fn bucket_value(
        &self,
        index: usize,
    ) -> Option<u64> {
        if index < 64 {
            Some(self.buckets[index])
        } else {
            None
        }
    }

    /// Returns a reference to all 64 buckets.
    pub fn buckets(&self) -> &[u64; 64] {
        &self.buckets
    }

    /// Number of events counted.
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    /// Returns the total event time in nanoseconds, if no overflow occurred.
    pub fn event_time_total(&self) -> Option<u64> {
        if self.has_overflowed {
            None
        } else {
            Some(self.event_time_total)
        }
    }

    /// Returns the total event time in nanoseconds, regardless of whether
    /// overflow has occurred.
    pub fn event_time_total_raw(&self) -> u64 {
        self.event_time_total
    }

    /// Indicates whether overflow has occurred.
    pub fn has_overflowed(&self) -> bool {
        self.has_overflowed
    }

    /// Returns the minimum event time observed, if any.
    pub fn min_event_time(&self) -> Option<u64> {
        self.min_event_time
    }

    /// Returns the maximum event time observed, if any.
    pub fn max_event_time(&self) -> Option<u64> {
        self.max_event_time
    }

    /// Returns the approximated duration (in nanoseconds) at the given
    /// percentile.
    ///
    /// # Parameters
    /// - `percentile`: A float value representing the desired percentile
    ///   (e.g., `50.0` for p50, `99.0` for p99); value is clamped to
    ///   the range `[0.0, 100.0]`;
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    pub fn value_at_percentile(
        &self,
        percentile: f64,
    ) -> Option<u64> {
        if self.event_count == 0 {
            return None;
        }

        let p = percentile.clamp(0.0, 100.0);

        if p <= 0.0 {
            let r = self.min_event_time;

            return r;
        }

        if p >= 100.0 {
            let r = self.max_event_time;

            return r;
        }

        let target_rank = self.event_count as f64 * (p / 100.0);
        let mut accumulated = 0u64;

        // Iterate forwards because spatial clustering of latency events means
        // that active events cluster heavily in the lower-indexed buckets.
        // Higher-indexed buckets (e.g., above 30) are almost always empty
        // in high-performance loops. Iterating forwards allows the loop
        // to terminate much earlier (typically under 20 iterations).
        for i in 0..64 {
            let count = self.buckets[i];

            if count > 0 {
                let prev_accumulated = accumulated;
                accumulated += count;

                if accumulated as f64 >= target_rank {
                    let (lower, upper) = Self::bucket_range(i).unwrap_or((0, u64::MAX));

                    let target_offset = target_rank - prev_accumulated as f64;
                    let range_width = if i == 63 {
                        (u64::MAX - lower) as f64
                    } else {
                        (upper - lower) as f64
                    };

                    let fraction = target_offset / count as f64;
                    let interpolated = lower as f64 + (range_width * fraction);
                    let mut value = interpolated.round() as u64;

                    if let Some(min) = self.min_event_time {
                        if value < min {
                            value = min;
                        }
                    }

                    if let Some(max) = self.max_event_time {
                        if value > max {
                            value = max;
                        }
                    }

                    let r = Some(value);

                    return r;
                }
            }
        }

        let r = self.max_event_time;

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 50th
    /// percentile (p50).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p50(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 1) / 2;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 75th
    /// percentile (p75).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p75(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 3) / 4;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 90th
    /// percentile (p90).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p90(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 90) / 100;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 95th
    /// percentile (p95).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p95(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 95) / 100;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99th
    /// percentile (p99).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 99) / 100;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99.5th
    /// percentile (p99.5).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99_5(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 995) / 1_000;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99.9th
    /// percentile (p99.9).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99_9(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 999) / 1_000;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99.99th
    /// percentile (p99.99).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99_99(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 9_999) / 10_000;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99.999th
    /// percentile (p99.999).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99_999(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 99_999) / 100_000;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }

    /// Returns the approximated duration (in nanoseconds) at the 99.9999th
    /// percentile (p99.9999).
    ///
    /// # Return
    /// Returns `Some(value_in_ns)` if the histogram contains one or more
    /// events; otherwise, returns `None`.
    #[inline(always)]
    pub fn value_at_p99_999_9(&self) -> Option<u64> {
        let target_rank = (self.event_count as u128 * 999_999) / 1_000_000;
        let r = self.value_at_target_rank_impl(target_rank as u64);

        r
    }
}

// Trait implementations

impl std_fmt::Debug for Histogram {
    fn fmt(
        &self,
        f: &mut std_fmt::Formatter<'_>,
    ) -> std_fmt::Result {
        struct BucketsDebug<'a>(&'a [u64; 64], bool);

        impl std_fmt::Debug for BucketsDebug<'_> {
            fn fmt(
                &self,
                f: &mut std_fmt::Formatter<'_>,
            ) -> std_fmt::Result {
                struct PowerOfTwoKey(usize);

                impl std_fmt::Debug for PowerOfTwoKey {
                    fn fmt(
                        &self,
                        f: &mut std_fmt::Formatter<'_>,
                    ) -> std_fmt::Result {
                        write!(f, "\"2^{}\"", self.0)
                    }
                }

                let mut m = f.debug_map();
                for (i, &count) in self.0.iter().enumerate() {
                    if count > 0 {
                        if self.1 {
                            m.entry(&PowerOfTwoKey(i), &count);
                        } else {
                            m.entry(&i, &count);
                        }
                    }
                }
                m.finish()
            }
        }

        if f.alternate() {
            f.debug_struct("Histogram")
                .field("event_count", &self.event_count)
                .field("event_time_total", &self.event_time_total())
                .field("has_overflowed", &self.has_overflowed)
                .field("min_event_time", &self.min_event_time)
                .field("max_event_time", &self.max_event_time)
                .field("buckets", &BucketsDebug(&self.buckets, true))
                .finish()
        } else {
            f.debug_struct("Histogram")
                .field("n", &self.event_count)
                .field("∑", &self.event_time_total())
                .field("∞", &self.has_overflowed)
                .field("↓", &self.min_event_time)
                .field("↑", &self.max_event_time)
                .field("b", &BucketsDebug(&self.buckets, false))
                .finish()
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            event_count: 0,
            event_time_total: 0,
            has_overflowed: false,
            min_event_time: None,
            max_event_time: None,
            buckets: [0; 64],
        }
    }
}

// Implementation

impl Histogram {
    /// Calculates the bucket index for a given elapsed time in nanoseconds.
    ///
    /// The index is computed logarithmic-wise based on the power of two of
    /// the value. Specifically, it maps:
    /// - `0` and `1` to bucket `0`;
    /// - `2` and `3` to bucket `1`;
    /// - `4` to `7` to bucket `2`;
    /// - `8` to `15` to bucket `3`;
    /// - ...
    /// - `(1 << 63)` to `u64::MAX` to bucket `63`;
    ///
    /// This is extremely fast because it is implemented via the CPU's
    /// `leading_zeros()` instruction, avoiding loop and branching logic.
    #[doc(hidden)]
    #[inline]
    pub fn bucket_index(time_in_ns: u64) -> usize {
        if time_in_ns <= 1 {
            0
        } else {
            (64 - time_in_ns.leading_zeros() - 1) as usize
        }
    }

    /// Returns the inclusive range `(lower_bound, upper_bound)` of
    /// nanoseconds represented by the given bucket index.
    ///
    /// - Index `0` represents `[0, 1]` nanoseconds;
    /// - Any index `i` from `1` to `63` represents `[2^i, 2^(i+1) - 1]`;
    #[doc(hidden)]
    pub fn bucket_range(index: usize) -> Option<(u64, u64)> {
        if index >= 64 {
            None
        } else if index == 0 {
            Some((0, 1))
        } else {
            let lower = 1u64 << index;
            let upper = if index == 63 {
                u64::MAX
            } else {
                (1u64 << (index + 1)) - 1
            };

            Some((lower, upper))
        }
    }

    fn try_add_ns_to_total_and_update_minmax_and_count_(
        &mut self,
        time_in_ns: u64,
    ) -> bool {
        if self.has_overflowed {
            return false;
        }

        match self.event_time_total.checked_add(time_in_ns) {
            Some(new_total) => {
                self.event_time_total = new_total;

                match self.min_event_time {
                    Some(min_event_time) => {
                        if time_in_ns < min_event_time {
                            self.min_event_time = Some(time_in_ns);
                        }
                    },
                    None => {
                        self.min_event_time = Some(time_in_ns);
                    },
                }

                match self.max_event_time {
                    Some(max_event_time) => {
                        if time_in_ns > max_event_time {
                            self.max_event_time = Some(time_in_ns);
                        }
                    },
                    None => {
                        self.max_event_time = Some(time_in_ns);
                    },
                }

                true
            },
            None => {
                self.has_overflowed = true;

                false
            },
        }
    }

    fn value_at_target_rank_impl(
        &self,
        target_rank: u64,
    ) -> Option<u64> {
        if self.event_count == 0 {
            return None;
        }

        let mut accumulated = 0u64;

        // Iterate forwards because spatial clustering of latency events means
        // that active events cluster heavily in the lower-indexed buckets.
        // Higher-indexed buckets (e.g., above 30) are almost always empty
        // in high-performance loops. Iterating forwards allows the loop
        // to terminate much earlier (typically under 20 iterations).
        for i in 0..64 {
            let count = self.buckets[i];

            if count > 0 {
                let prev_accumulated = accumulated;
                accumulated += count;

                if accumulated >= target_rank {
                    let (lower, upper) = Self::bucket_range(i).unwrap_or((0, u64::MAX));

                    let target_offset = target_rank - prev_accumulated;

                    let interpolated = if target_offset == 0 {
                        lower
                    } else {
                        let range_width = if i == 63 { u64::MAX - lower } else { upper - lower };

                        if let Some(prod) = range_width.checked_mul(target_offset) {
                            lower + prod / count
                        } else {
                            let val_u128 =
                                lower as u128 + (range_width as u128 * target_offset as u128) / count as u128;

                            val_u128 as u64
                        }
                    };

                    let mut value = interpolated;

                    if let Some(min) = self.min_event_time {
                        if value < min {
                            value = min;
                        }
                    }

                    if let Some(max) = self.max_event_time {
                        if value > max {
                            value = max;
                        }
                    }

                    let r = Some(value);

                    return r;
                }
            }
        }

        let r = self.max_event_time;

        r
    }
}

// Tests

#[cfg(test)]
mod test_helpers {
    #![allow(non_snake_case)]
    #![allow(unused)]
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    #![cfg_attr(debug_assertions, allow(unused_imports))]

    use super::Histogram;

    #[rustfmt::skip]
    use test_helpers::{
        assert_scalar_eq_approx,
        multiplier,
    };

    use std::time as std_time;

    #[test]
    fn TEST_Histogram_Debug() {

        // empty
        {
            let h = Histogram::default();

            let expected = "Histogram { n: 0, ∑: Some(0), ∞: false, ↓: None, ↑: None, b: {} }";
            assert_eq!(expected, format!("{:?}", h));
        }

        // populated
        {
            let mut h = Histogram::default();

            let expected = "Histogram { n: 3, ∑: Some(500), ∞: false, ↓: Some(100), ↑: Some(200), b: {6: 1, 7: 2} }";

            assert!(h.push_event_time_ns(100)); // bucket 6
            assert!(h.push_event_time_ns(200)); // bucket 7
            assert!(h.push_event_time_ns(200)); // bucket 7

            assert_eq!(expected, format!("{:?}", h));
        }
    }

    #[test]
    fn TEST_Histogram_Debug_alternate() {

        // empty
        {
            let h = Histogram::default();

            let expected = r#"Histogram {
    event_count: 0,
    event_time_total: Some(
        0,
    ),
    has_overflowed: false,
    min_event_time: None,
    max_event_time: None,
    buckets: {},
}"#;
            assert_eq!(expected, format!("{:#?}", h));
        }

        // populated
        {
            let mut h = Histogram::default();

            assert!(h.push_event_time_ns(100)); // bucket 6
            assert!(h.push_event_time_ns(10_000)); // bucket 13
            assert!(h.push_event_time_ns(10_001)); // bucket 13

            let expected = r#"Histogram {
    event_count: 3,
    event_time_total: Some(
        20101,
    ),
    has_overflowed: false,
    min_event_time: Some(
        100,
    ),
    max_event_time: Some(
        10001,
    ),
    buckets: {
        "2^6": 1,
        "2^13": 2,
    },
}"#;
            assert_eq!(expected, format!("{:#?}", h));
        }
    }

    #[test]
    fn TEST_Histogram_Default() {
        let h = Histogram::default();

        assert_eq!(0, h.event_count());
        assert_eq!(Some(0), h.event_time_total());
        assert_eq!(0, h.event_time_total_raw());
        assert!(!h.has_overflowed());
        assert_eq!(None, h.min_event_time());
        assert_eq!(None, h.max_event_time());

        for i in 0..64 {
            assert_eq!(0, h.buckets()[i]);
            assert_eq!(Some(0), h.bucket_value(i));
        }

        assert_eq!(None, h.bucket_value(64));
    }

    #[test]
    fn TEST_Histogram_bucket_index() {
        assert_eq!(0, Histogram::bucket_index(0));
        assert_eq!(0, Histogram::bucket_index(1));

        assert_eq!(1, Histogram::bucket_index(2));
        assert_eq!(1, Histogram::bucket_index(3));

        assert_eq!(2, Histogram::bucket_index(4));
        assert_eq!(2, Histogram::bucket_index(7));

        assert_eq!(3, Histogram::bucket_index(8));
        assert_eq!(3, Histogram::bucket_index(15));

        assert_eq!(4, Histogram::bucket_index(16));
        assert_eq!(4, Histogram::bucket_index(31));

        assert_eq!(10, Histogram::bucket_index(1024));
        assert_eq!(10, Histogram::bucket_index(2047));

        assert_eq!(63, Histogram::bucket_index(1u64 << 63));
        assert_eq!(63, Histogram::bucket_index(u64::MAX));
    }

    #[test]
    fn TEST_Histogram_bucket_range() {
        assert_eq!(Some((0, 1)), Histogram::bucket_range(0));
        assert_eq!(Some((2, 3)), Histogram::bucket_range(1));
        assert_eq!(Some((4, 7)), Histogram::bucket_range(2));
        assert_eq!(Some((8, 15)), Histogram::bucket_range(3));
        assert_eq!(Some((16, 31)), Histogram::bucket_range(4));
        assert_eq!(Some((1024, 2047)), Histogram::bucket_range(10));
        assert_eq!(Some((1u64 << 63, u64::MAX)), Histogram::bucket_range(63));

        assert_eq!(None, Histogram::bucket_range(64));
    }

    #[test]
    fn TEST_Histogram_PUSH_EVENTS() {
        let mut h = Histogram::default();

        assert!(h.push_event_time_ns(1));
        assert!(h.push_event_time_ns(3));
        assert!(h.push_event_time_us(10));
        assert!(h.push_event_time_ms(5));
        assert!(h.push_event_time_s(2));
        assert!(h.push_event_duration(std_time::Duration::from_nanos(100)));

        assert_eq!(6, h.event_count());
        assert!(!h.has_overflowed());
        assert_eq!(Some(1), h.min_event_time());
        assert_eq!(Some(2_000_000_000), h.max_event_time());
        assert_eq!(Some(2_005_010_104), h.event_time_total());

        assert_eq!(1, h.buckets()[0]);
        assert_eq!(1, h.buckets()[1]);
        assert_eq!(1, h.buckets()[6]);
        assert_eq!(1, h.buckets()[13]);
        assert_eq!(1, h.buckets()[22]);
        assert_eq!(1, h.buckets()[30]);

        h.clear();

        assert_eq!(0, h.event_count());
        assert_eq!(Some(0), h.event_time_total());
    }

    #[test]
    fn TEST_Histogram_OVERFLOW() {
        let mut h = Histogram::default();
        assert!(h.push_event_time_ns(u64::MAX));
        assert_eq!(Some(u64::MAX), h.event_time_total());
        assert!(!h.has_overflowed());

        assert!(!h.push_event_time_ns(1));
        assert!(h.has_overflowed());
        assert_eq!(None, h.event_time_total());
        assert_eq!(u64::MAX, h.event_time_total_raw());
    }

    #[test]
    fn TEST_Histogram_PERCENTILES_EMPTY() {
        let h = Histogram::default();

        assert_eq!(None, h.value_at_percentile(50.0));
        assert_eq!(None, h.value_at_p50());
        assert_eq!(None, h.value_at_p99());
    }

    #[test]
    fn TEST_Histogram_PERCENTILES_SINGLE_EVENT() {
        let mut h = Histogram::default();
        assert!(h.push_event_time_ns(100));

        assert_eq!(Some(100), h.value_at_percentile(0.0));
        assert_eq!(Some(100), h.value_at_percentile(50.0));
        assert_eq!(Some(100), h.value_at_percentile(99.0));
        assert_eq!(Some(100), h.value_at_percentile(100.0));

        assert_eq!(Some(100), h.value_at_p50());
        assert_eq!(Some(100), h.value_at_p90());
        assert_eq!(Some(100), h.value_at_p99());
        assert_eq!(Some(100), h.value_at_p99_999_9());
    }

    #[test]
    fn TEST_Histogram_PERCENTILES_INTERPOLATION() {
        let mut h = Histogram::default();
        assert!(h.push_event_time_ns(100)); // Bucket 6 [64, 127]
        assert!(h.push_event_time_ns(200)); // Bucket 7 [128, 255]

        let p50 = h.value_at_percentile(50.0);
        let p99 = h.value_at_percentile(99.0);

        assert!(p50.is_some());
        assert!(p99.is_some());

        assert!(p50.unwrap() >= 100 && p50.unwrap() <= 200);
        assert!(p99.unwrap() >= 100 && p99.unwrap() <= 200);

        assert_eq!(Some(100), h.value_at_percentile(0.0));
        assert_eq!(Some(200), h.value_at_percentile(100.0));

        // Int-based equivalents
        assert!(h.value_at_p50().unwrap() >= 100);
        assert!(h.value_at_p99().unwrap() <= 200);
    }

    #[test]
    fn TEST_Histogram_PERCENTILES_WIDE_RANGE() {
        let mut h = Histogram::default();

        // Push events spanning a vast range of magnitudes
        let values = [
            1,              // 1 ns (Bucket 0)
            10,             // 10 ns (Bucket 3)
            100,            // 100 ns (Bucket 6)
            1_000,          // 1 us (Bucket 9)
            10_000,         // 10 us (Bucket 13)
            100_000,        // 100 us (Bucket 16)
            1_000_000,      // 1 ms (Bucket 19)
            10_000_000,     // 10 ms (Bucket 23)
            100_000_000,    // 100 ms (Bucket 26)
            1_000_000_000,  // 1 s (Bucket 29)
            10_000_000_000, // 10 s (Bucket 33)
        ];

        for &v in &values {
            assert!(h.push_event_time_ns(v));
        }

        assert_eq!(values.len(), h.event_count());
        assert_eq!(Some(1), h.min_event_time());
        assert_eq!(Some(10_000_000_000), h.max_event_time());

        // Check that percentiles are strictly monotonic
        let p50 = h.value_at_p50().unwrap();
        let p75 = h.value_at_p75().unwrap();
        let p90 = h.value_at_p90().unwrap();
        let p95 = h.value_at_p95().unwrap();
        let p99 = h.value_at_p99().unwrap();
        let p99_5 = h.value_at_p99_5().unwrap();
        let p99_9 = h.value_at_p99_9().unwrap();
        let p99_99 = h.value_at_p99_99().unwrap();
        let p99_999 = h.value_at_p99_999().unwrap();
        let p99_999_9 = h.value_at_p99_999_9().unwrap();

        assert!(p50 <= p75);
        assert!(p75 <= p90);
        assert!(p90 <= p95);
        assert!(p95 <= p99);
        assert!(p99 <= p99_5);
        assert!(p99_5 <= p99_9);
        assert!(p99_9 <= p99_99);
        assert!(p99_99 <= p99_999);
        assert!(p99_999 <= p99_999_9);

        // Verify bounds
        assert!(p50 >= 1);
        assert!(p99_999_9 <= 10_000_000_000);
    }

    #[test]
    fn TEST_Histogram_PERCENTILES_MANY_EVENTS() {
        let mut h = Histogram::default();
        let count = 100_000;

        // Push 100,000 linear events from 1 to 100,000 ns
        for i in 1..=count {
            assert!(h.push_event_time_ns(i as u64));
        }

        assert_eq!(count, h.event_count());
        assert_eq!(Some(1), h.min_event_time());
        assert_eq!(Some(count as u64), h.max_event_time());

        // Check approximated percentiles against theoretical values.
        // Because of the logarithmic bucket spacing, larger values have wider buckets,
        // so the approximation error will be larger for higher percentiles, but they
        // should still be reasonably close.
        let p50 = h.value_at_p50().unwrap();
        let p90 = h.value_at_p90().unwrap();
        let p99 = h.value_at_p99().unwrap();
        let p99_9 = h.value_at_p99_9().unwrap();

        // p50 theoretical = 50,000. Bucket 15 is [32768, 65535] (width 32768).
        // Since bucket 15 is fully populated, the linear interpolation is extremely accurate.
        assert_eq!(50_000, p50, "p50 was {}, expected exactly 50000", p50);

        // p90 theoretical = 90,000. Bucket 16 is [65536, 131071] (width 65535).
        // Since we only populated up to 100,000, bucket 16 is partially populated.
        // The linear interpolation assumes events are spread up to 131,071, which
        // yields an unclamped estimate of ~112,055. This is correctly clamped
        // to the actual maximum seen event time of 100,000.
        assert_eq!(100_000, p90, "p90 was {}, expected clamped to 100000", p90);

        // p99 theoretical = 99,000. Also in bucket 16, also clamped to max.
        assert_eq!(100_000, p99, "p99 was {}, expected clamped to 100000", p99);

        // p99.9 theoretical = 99,900. Also in bucket 16, also clamped to max.
        assert_eq!(100_000, p99_9, "p99.9 was {}, expected clamped to 100000", p99_9);

        // Check monotonicity of all integer percentiles
        let p75 = h.value_at_p75().unwrap();
        let p95 = h.value_at_p95().unwrap();
        let p99_5 = h.value_at_p99_5().unwrap();
        let p99_99 = h.value_at_p99_99().unwrap();
        let p99_999 = h.value_at_p99_999().unwrap();
        let p99_999_9 = h.value_at_p99_999_9().unwrap();

        assert!(p50 <= p75);
        assert!(p75 <= p90);
        assert!(p90 <= p95);
        assert!(p95 <= p99);
        assert!(p99 <= p99_5);
        assert!(p99_5 <= p99_9);
        assert!(p99_9 <= p99_99);
        assert!(p99_99 <= p99_999);
        assert!(p99_999 <= p99_999_9);
    }

    #[test]
    fn TEST_Histogram_COMPARE_FLOAT_AND_INT_PERCENTILES() {
        let mut h = Histogram::default();

        // Push some values representing a realistic latency distribution
        for i in 1..=10_000 {
            let val = (i * i) % 1_000_000;
            assert!(h.push_event_time_ns(val as u64));
        }

        // Compare float vs int percentiles
        let float_p50 = h.value_at_percentile(50.0).unwrap();
        let int_p50 = h.value_at_p50().unwrap();
        assert_scalar_eq_approx!(float_p50 as f64, int_p50 as f64, multiplier(0.01));

        let float_p75 = h.value_at_percentile(75.0).unwrap();
        let int_p75 = h.value_at_p75().unwrap();
        assert_scalar_eq_approx!(float_p75 as f64, int_p75 as f64, multiplier(0.01));

        let float_p90 = h.value_at_percentile(90.0).unwrap();
        let int_p90 = h.value_at_p90().unwrap();
        assert_scalar_eq_approx!(float_p90 as f64, int_p90 as f64, multiplier(0.01));

        let float_p95 = h.value_at_percentile(95.0).unwrap();
        let int_p95 = h.value_at_p95().unwrap();
        assert_scalar_eq_approx!(float_p95 as f64, int_p95 as f64, multiplier(0.01));

        let float_p99 = h.value_at_percentile(99.0).unwrap();
        let int_p99 = h.value_at_p99().unwrap();
        assert_scalar_eq_approx!(float_p99 as f64, int_p99 as f64, multiplier(0.01));

        let float_p99_5 = h.value_at_percentile(99.5).unwrap();
        let int_p99_5 = h.value_at_p99_5().unwrap();
        assert_scalar_eq_approx!(float_p99_5 as f64, int_p99_5 as f64, multiplier(0.01));

        let float_p99_9 = h.value_at_percentile(99.9).unwrap();
        let int_p99_9 = h.value_at_p99_9().unwrap();
        assert_scalar_eq_approx!(float_p99_9 as f64, int_p99_9 as f64, multiplier(0.01));

        let float_p99_99 = h.value_at_percentile(99.99).unwrap();
        let int_p99_99 = h.value_at_p99_99().unwrap();
        assert_scalar_eq_approx!(float_p99_99 as f64, int_p99_99 as f64, multiplier(0.01));

        let float_p99_999 = h.value_at_percentile(99.999).unwrap();
        let int_p99_999 = h.value_at_p99_999().unwrap();
        assert_scalar_eq_approx!(float_p99_999 as f64, int_p99_999 as f64, multiplier(0.01));

        let float_p99_999_9 = h.value_at_percentile(99.9999).unwrap();
        let int_p99_999_9 = h.value_at_p99_999_9().unwrap();
        assert_scalar_eq_approx!(float_p99_999_9 as f64, int_p99_999_9 as f64, multiplier(0.01));
    }
}
