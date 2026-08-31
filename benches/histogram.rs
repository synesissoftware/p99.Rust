// benches/histogram.rs : benchmarking `Histogram`

#![allow(non_snake_case)]

use p99::Histogram;

#[rustfmt::skip]
use criterion::{
    BatchSize,
    Criterion,
    criterion_group,
    criterion_main,
};

#[rustfmt::skip]
use std::{
    hint as std_hint,
};

// Helper functions

fn build_sequential_histogram() -> Histogram {
    let mut h = Histogram::default();
    for i in 1..=100_000 {
        let _ = h.push_event_time_ns(i * 10);
    }

    h
}

fn build_wide_range_histogram() -> Histogram {
    let mut h = Histogram::default();
    let mut state = 12_345u64;
    for _ in 1..=100_000 {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let val = (state % 10_000_000_000) + 1;
        let _ = h.push_event_time_ns(val);
    }

    h
}

fn bench_percentile_comparison<F>(
    c : &mut Criterion,
    h : &Histogram,
    percentile_name : &str,
    workload_name : &str,
    float_val : f64,
    int_method : F,
) where
    F : Fn(&Histogram) -> Option<u64> + Copy,
{
    let float_str = if float_val.fract() == 0.0 {
        format!("{:.1}", float_val)
    } else {
        format!("{}", float_val)
    };

    let float_id = format!("`Histogram::value_at_percentile({})` [{}]", float_str, workload_name);
    c.bench_function(&float_id, |b| {
        b.iter(|| {
            std_hint::black_box(h.value_at_percentile(std_hint::black_box(float_val)));
        })
    });

    let int_id = format!("`Histogram::value_at_p{}()` [{}]", percentile_name, workload_name);
    c.bench_function(&int_id, |b| {
        b.iter(|| {
            std_hint::black_box(int_method(std_hint::black_box(h)));
        })
    });
}

// Benchmarks

fn BENCHMARK_bucket_index_SMALL(c : &mut Criterion) {
    let id = "`Histogram::bucket_index(1)`".to_string();

    c.bench_function(&id, |b| {
        b.iter(|| {
            let idx = Histogram::bucket_index(std_hint::black_box(1));

            let _ = std_hint::black_box(idx);
        })
    });
}

fn BENCHMARK_bucket_index_LARGE(c : &mut Criterion) {
    let id = "`Histogram::bucket_index(u64::MAX)`".to_string();

    c.bench_function(&id, |b| {
        b.iter(|| {
            let idx = Histogram::bucket_index(std_hint::black_box(u64::MAX));

            let _ = std_hint::black_box(idx);
        })
    });
}

fn BENCHMARK_push_event_time_ns(c : &mut Criterion) {
    let id = "`Histogram::push_event_time_ns()`".to_string();

    c.bench_function(&id, |b| {
        b.iter_batched_ref(
            Histogram::default,
            |h| {
                std_hint::black_box(h.push_event_time_ns(std_hint::black_box(12_345)));
            },
            BatchSize::SmallInput,
        )
    });
}

fn BENCHMARK_clear(c : &mut Criterion) {
    let id = "`Histogram::clear()`".to_string();

    c.bench_function(&id, |b| {
        b.iter_batched_ref(
            || {
                let mut h = Histogram::default();
                h.push_event_time_ns(100);
                h.push_event_time_ns(200);

                h
            },
            |h| {
                h.clear();
                std_hint::black_box(h);
            },
            BatchSize::SmallInput,
        )
    });
}

fn BENCHMARK_percentile_queries(c : &mut Criterion) {
    // 1. Benchmark under dense sequential 100k events
    let seq_h = build_sequential_histogram();
    bench_percentile_comparison(c, &seq_h, "99", "100k events", 99.0, |h| h.value_at_p99());

    // 2. Benchmark under sparse wide-range 100k events
    let wide_h = build_wide_range_histogram();
    bench_percentile_comparison(c, &wide_h, "50", "100k wide-range events", 50.0, |h| h.value_at_p50());
    bench_percentile_comparison(c, &wide_h, "75", "100k wide-range events", 75.0, |h| h.value_at_p75());
    bench_percentile_comparison(c, &wide_h, "90", "100k wide-range events", 90.0, |h| h.value_at_p90());
    bench_percentile_comparison(c, &wide_h, "99", "100k wide-range events", 99.0, |h| h.value_at_p99());
    bench_percentile_comparison(c, &wide_h, "99_99", "100k wide-range events", 99.99, |h| {
        h.value_at_p99_99()
    });
}

// Macros

criterion_group!(
    benches,
    BENCHMARK_bucket_index_SMALL,
    BENCHMARK_bucket_index_LARGE,
    BENCHMARK_push_event_time_ns,
    BENCHMARK_clear,
    BENCHMARK_percentile_queries,
);
criterion_main!(benches);
