# p99.Rust <!-- omit in toc -->

![Language](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![GitHub release](https://img.shields.io/github/v/release/synesissoftware/p99.Rust.svg)](https://github.com/synesissoftware/p99.Rust/releases/latest)
[![Last Commit](https://img.shields.io/github/last-commit/synesissoftware/p99.Rust)](https://github.com/synesissoftware/p99.Rust/commits/master)
[![Crates.io](https://img.shields.io/crates/v/p99.svg)](https://crates.io/crates/p99)

Low-cost generation of performance percentiles (p50, p90, p99, p99.9, etc.).


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
		- [Dev Dependencies](#dev-dependencies)
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
*   **Ultra-Low Latency Insertion**: Recording a latency measurement (`push_event_time_ns`) takes approximately **11 nanoseconds** (about 35 CPU cycles on modern hardware).
*   **Blazing-Fast Queries**: Querying percentiles (such as `value_at_p99()`) takes only **11 to 17 nanoseconds**, depending on the distribution of events across the buckets.
*   **Instruction-Cache Friendly**: The query methods are designed with a "thin caller / heavy worker" pattern to prevent instruction-cache bloat and maintain high CPU cache locality under real-world workloads.

### Trade-offs & Sacrifices

*   **Logarithmic Precision**: To achieve zero allocation and constant-time operations, `Histogram` sacrifices exact precision. It does not store individual event times. Instead, values are grouped into logarithmic buckets.
*   **Approximation**: Percentile values are approximated using linear interpolation within the bucket boundaries. For very large values, the bucket width is wider, which leads to a wider approximation range. However, for low-latency performance measurements where precision is needed most (the lower nanosecond ranges), the buckets are extremely narrow (e.g., 1ns, 2ns, 4ns wide), providing exceptional resolution.


## Installation

Reference in **Cargo.toml** in the usual way:

```toml
p99 = { version = "0" }
```


## Components

### Constants

No public constants are defined at this time.


### Enumerations

No public enumerations are defined at this time.


### Features

No public crate-specific features are defined at this time.


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
```


## Project Information

### Where to get help

[GitHub Page](https://github.com/synesissoftware/p99.Rust "GitHub Page")


### Contribution guidelines

Defect reports, feature requests, and pull requests are welcome on https://github.com/synesissoftware/p99.Rust.


### Dependencies

**p99.Rust** has no (non-development) dependencies.

#### Dev Dependencies

Crates upon which **p99.Rust** has development dependencies:

* [**criterion**](https://github.com/bheisler/criterion.rs);
* [**test_help-rs**](https://github.com/synesissoftware/test_help-rs);


### License

**p99.Rust** is released under the 3-clause BSD license. See [LICENSE](./LICENSE) for details.


<!-- ########################### end of file ########################### -->
