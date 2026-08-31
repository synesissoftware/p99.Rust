# p99.Rust <!-- omit in toc -->

Low-cost generation of performance percentiles (p50, p90, p99, p99.9, etc.).

![Language](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Crates.io](https://img.shields.io/crates/v/p99.svg)](https://crates.io/crates/p99)
[![GitHub release](https://img.shields.io/github/v/release/synesissoftware/p99.Rust.svg)](https://github.com/synesissoftware/p99.Rust/releases/latest)
[![Last Commit](https://img.shields.io/github/last-commit/synesissoftware/p99.Rust)](https://github.com/synesissoftware/p99.Rust/commits/master)
[![MSRV](https://img.shields.io/badge/MSRV-1.74-lightgrey)](https://github.com/rust-lang/rust/releases/tag/1.74.0)
[![CI](https://github.com/synesissoftware/p99.Rust/actions/workflows/ci.yml/badge.svg)](https://github.com/synesissoftware/p99.Rust/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/p99/badge.svg)](https://docs.rs/p99)


## Table of Contents <!-- omit in toc -->

- [Introduction](#introduction)
- [How It Works](#how-it-works)
- [Performance \& Trade-offs](#performance--trade-offs)
  - [Performance Claims](#performance-claims)
  - [Trade-offs \& Sacrifices](#trade-offs--sacrifices)
- [Installation](#installation)
- [Components](#components)
  - [Constants](#constants)
  - [Enumerations](#enumerations)
  - [Features](#features)
    - [Enabling `binary-scaling`](#enabling-binary-scaling)
    - [Benchmark Results](#benchmark-results)
  - [Functions](#functions)
  - [Macros](#macros)
  - [Structures](#structures)
    - [`Histogram`](#histogram)
      - [Definition](#definition)
      - [Minimal Example](#minimal-example)
  - [Traits](#traits)
- [Examples](#examples)
- [Project Information](#project-information)
  - [Where to get help](#where-to-get-help)
  - [Contribution guidelines](#contribution-guidelines)
  - [Dependencies](#dependencies)
    - [Development Dependencies](#development-dependencies)
  - [License](#license)


## Introduction

**p99** is a lightweight, low-overhead library designed for generating real-time performance percentiles in high-frequency or latency-sensitive environments.

**p99.Rust** is the **Rust** implementation.


## How It Works

`Histogram` is a low-overhead, zero-allocation, fixed-size structure designed to track event durations (typically in nanoseconds) using 64 logarithmic buckets.

*   **Logarithmic Bucketing**: The bucket boundaries are spaced as powers of two:
    *   Bucket `0` represents `[0, 1]` nanoseconds;
    *   Bucket `1` represents `[2, 3]` nanoseconds;
    *   Bucket `2` represents `[4, 7]` nanoseconds;
    *   Bucket `i` represents `[2^i, 2^(i+1) - 1]` nanoseconds.
*   **Branchless Indexing**: Finding the correct bucket index for an incoming duration is extremely fast and branchless. It is computed in a few CPU instructions using the CPU's leading-zeros count intrinsic (`u64::leading_zeros`).
*   **Linear Interpolation**: Percentile queries iterate through the buckets to find the target rank and perform linear interpolation within the matching bucket to approximate the exact percentile duration.


## Performance & Trade-offs

### Performance Claims

*   **Zero Allocation**: `Histogram` does not allocate memory on the heap during creation, event insertion, or percentile queries. It is a compact (~576-byte) structure that can reside entirely on the stack or be embedded in other structures.
*   **Low-Latency Operations**: The fixed bucket layout is designed for low-latency insertion and percentile queries. Actual timings depend on the processor, compiler, build profile, and workload.
*   **Instruction-Cache Friendly**: The query methods are designed with a "thin caller / heavy worker" pattern to prevent instruction-cache bloat and maintain high CPU cache locality under real-world workloads.

The statements above describe implementation characteristics or design goals,
not guaranteed timings. Measured results from the checked-in Criterion
benchmark are reported below.

### Trade-offs & Sacrifices

*   **Logarithmic Precision**: To achieve zero allocation and constant-time operations, `Histogram` sacrifices exact precision. It does not store individual event times. Instead, values are grouped into logarithmic buckets.
*   **Approximation**: Percentile values are approximated using linear interpolation within the bucket boundaries. For very large values, the bucket width is wider, which leads to a wider approximation range. However, for low-latency performance measurements where precision is needed most (the lower nanosecond ranges), the buckets are extremely narrow (e.g., 1ns, 2ns, 4ns wide), providing exceptional resolution.
*   **`binary-scaling` Accuracy**: When the `binary-scaling` feature is enabled, the percentile target rank is computed using a $2^{32}$ fixed-point approximation. The pre-encoded multiplier for each percentile (e.g., `3_865_470_566 >> 32` ≈ `0.9000` for p90) differs from the true decimal value by less than $10^{-9}$, which is far below the approximation error introduced by the logarithmic bucketing itself. In practice this has no measurable impact on percentile accuracy.


## Installation

Reference the current release in **Cargo.toml** in the usual way:

```toml
p99 = { version = "0.0.3" }
```

To enable the optional binary-scaling optimization:

```toml
p99 = { version = "0.0.3", features = ["binary-scaling"] }
```


## Components

### Constants

No public constants are defined at this time.


### Enumerations

No public enumerations are defined at this time.


### Features

The following crate features are available:

* **`"binary-scaling"`** *(opt-in)*: Replaces integer division in the integer-based percentile methods (`#value_at_p90()`, `#value_at_p95()`, `#value_at_p99()`, etc.) with $2^{32}$ fixed-point binary scaling. Each percentile multiplier (e.g., `0.90` for p90) is pre-encoded as a `u32` constant and the target rank is computed via a single multiplication and a 32-bit right-shift, avoiding the cost of integer division entirely. This yields a **~1.5x to 2x speedup** for percentile queries with a negligible loss of accuracy (the scaled multiplier differs from the true value by less than $10^{-9}$). The generic `#value_at_percentile(f64)` method is unaffected by this feature;

* **`"null-feature"`** *(opt-in)*: A no-op feature that has no effect on the compiled library. It exists to simplify driver scripts and CI pipelines that conditionally pass `--features` flags, allowing a feature list to always be present even when no real features are needed;


#### Enabling `binary-scaling`

Add the feature in your **Cargo.toml**:

```toml
[dependencies]
p99 = { version = "0.0.3", features = ["binary-scaling"] }
```

Or, when building from the command line:

```bash
# Default (standard integer division)
cargo run --example build_histogram

# With binary scaling enabled
cargo run --example build_histogram --features binary-scaling
```

#### Benchmark Results

Measured with [**criterion**](https://github.com/bheisler/criterion.rs) on
100,000 events per workload, using the release profile on an Apple M-series
machine. Criterion's default measurement configuration was used. The exact
machine, operating-system, compiler, and Criterion output are not recorded,
so these results are illustrative rather than guaranteed. Only the
integer-based percentile methods are affected; the generic
`value_at_percentile(f64)` method is unchanged.

Run the benchmark in the release profile with
`cargo bench --bench histogram --release`, then repeat with
`--features binary-scaling` to reproduce the comparison.

| Method | Default | `binary-scaling` | Improvement |
|---|---:|---:|---:|
| `value_at_p90()` | 23.25 ns | 21.63 ns | **-7.0%** |
| `value_at_p99()` (dense) | 16.52 ns | 14.88 ns | **-10.4%** |
| `value_at_p99()` (wide) | 23.39 ns | 21.55 ns | **-7.9%** |
| `value_at_p99_99()` | 23.32 ns | 21.64 ns | **-7.2%** |

Methods using simple fractional multipliers (p50 = 1/2, p75 = 3/4) already compile to bit-shifts without this feature, so they show no change.


### Functions

No public functions are defined at this time.


### Macros

No public macros are defined at this time.


### Structures

The following public structures are defined in the current version:

#### `Histogram`

A low-cost, zero-allocation, 64-bucket logarithmic histogram designed for recording event durations in nanoseconds and querying high-resolution percentiles.

##### Definition

```rust
pub struct Histogram {
    // ... private fields ...
}
```

##### Minimal Example

Here is a simple example of how to initialize a `Histogram`, record event times, and query percentiles:

```rust
use p99::Histogram;
use std::time::Duration;

fn main() {
    // 1. Initialize a default histogram (zero-allocated, ~576 bytes on the stack)
    let mut histogram = Histogram::default();

    // 2. Record event times using various units
    histogram.push_event_time_ns(150); // 150 ns
    histogram.push_event_time_us(5);   // 5 microseconds (5,000 ns)
    histogram.push_event_time_ms(10);  // 10 milliseconds (10,000,000 ns)
    histogram.push_event_duration(Duration::from_nanos(250));

    // 3. Retrieve basic statistics
    assert_eq!(histogram.event_count(), 4);
    assert_eq!(histogram.min_event_time(), Some(150));
    assert_eq!(histogram.max_event_time(), Some(10_000_000));
    assert_eq!(histogram.event_time_total(), Some(10_005_400));

    // 4. Query percentiles
    // Generic floating-point query (e.g., 90th percentile)
    if let Some(p90) = histogram.value_at_percentile(90.0) {
        println!("p90: {} ns", p90);
    }

    // Fast, optimized integer-based percentile wrappers
    if let Some(p50) = histogram.value_at_p50() {
        println!("p50 (median): {} ns", p50);
    }
    if let Some(p99) = histogram.value_at_p99() {
        println!("p99: {} ns", p99);
    }
}
```


### Traits

No public traits are defined at this time.


## Examples

An example program showing `Histogram` usage is provided in [**examples/build_histogram.rs**](./examples/build_histogram.rs).

It simulates a histogram of event times generated by `std::thread::sleep` delays under a custom PRNG.

The number of iterations can be configured via the `P99_TRIES` environment variable:

```bash
# Run with the default of 100 tries
cargo run --example build_histogram

# Run with 1000 tries
P99_TRIES=1000 cargo run --example build_histogram

# Run with binary-scaling enabled (faster percentile queries)
cargo run --example build_histogram --features binary-scaling
```


## Project Information

### Where to get help

[GitHub Page](https://github.com/synesissoftware/p99.Rust "GitHub Page")


### Contribution guidelines

Defect reports, feature requests, and pull requests are welcome on https://github.com/synesissoftware/p99.Rust.


### Dependencies

**p99.Rust** has no (non-development) dependencies.

#### Development Dependencies

Crates upon which **p99.Rust** has development dependencies:

* [**criterion**](https://github.com/bheisler/criterion.rs);
* [**test_help-rs**](https://github.com/synesissoftware/test_help-rs);

**Cargo.lock** is retained so local and CI validation use a reproducible
dependency graph. Commands that consume the lockfile use `--locked`.


### License

**p99.Rust** is released under the 3-clause BSD license. See [LICENSE](./LICENSE) for details.


<!-- ########################### end of file ########################### -->
