// examples/build_histogram.rs : example program showing `Histogram`

use p99::Histogram;

#[rustfmt::skip]
use std::{
    env as std_env,
    thread as std_thread,
    time as std_time,
};

struct SimpleRng {
    state : u64,
}

// API functions

impl SimpleRng {
    fn new(seed : u64) -> Self {
        Self {
            state : seed
        }
    }
}

// Mutating methods

impl SimpleRng {
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);

        self.state
    }
}

// Helper functions

fn main() {
    let range = match std_env::var("P99_RANGE") {
        Ok(val) => {
            match val.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!(
                        "Warning: failed to parse P99_RANGE value '{}', defaulting to 1,000,000",
                        val
                    );

                    1_000_000
                },
            }
        },
        Err(_) => 1_000_000,
    };
    let tries = match std_env::var("P99_TRIES") {
        Ok(val) => {
            match val.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("Warning: failed to parse P99_TRIES value '{}', defaulting to 100", val);

                    100
                },
            }
        },
        Err(_) => 100,
    };

    println!("Running Histogram example with {} tries...", tries);

    let mut histogram = Histogram::default();
    let mut rng = SimpleRng::new(12_345);

    for _ in 0..tries {
        // Generate a random delay from 1 to <range> nanoseconds.
        let delay_ns = (rng.next() % range as u64) + 1;
        let start = std_time::Instant::now();

        if delay_ns < 1_000_000 {
            let busy_wait_start = std_time::Instant::now();
            while busy_wait_start.elapsed().as_nanos() < delay_ns as u128 {
                std::hint::black_box(1);
            }
        } else {
            std_thread::sleep(std_time::Duration::from_micros(delay_ns));
        }

        let elapsed = start.elapsed();
        histogram.push_event_duration(elapsed);
    }

    println!("\nHistogram printed via `{{:#?}}` format:\n");
    println!("{:#?}", histogram);

    println!("\nPercentiles (approximated):");
    println!("  p50 (f64):        {:?} ns", histogram.value_at_percentile(50.0));
    println!("  p50 (integer):    {:?} ns", histogram.value_at_p50());
    println!("  p75 (integer):    {:?} ns", histogram.value_at_p75());
    println!("  p90 (integer):    {:?} ns", histogram.value_at_p90());
    println!("  p95 (integer):    {:?} ns", histogram.value_at_p95());
    println!("  p99 (integer):    {:?} ns", histogram.value_at_p99());
    println!("  p99.5 (integer):  {:?} ns", histogram.value_at_p99_5());
    println!("  p99.9 (integer):  {:?} ns", histogram.value_at_p99_9());
    println!("  p99.99 (integer): {:?} ns", histogram.value_at_p99_99());
}
