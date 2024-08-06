use performance_metrics::{ read_cpu_timer, get_cpu_frequency_estimate };
use std::{
    io::{ stdout, Write },
    os::windows::fs::MetadataExt,
    fs
};

/// Number of seconds to wait for a new hi/lo count before ending the test.
const MAX_WAIT_TIME_SECONDS: f64 = 3.0;
const FILENAME: &str = "haversine_pairs.json";
const MEGABYTES: u64 = 1024 * 1024;
const GIGABYTES: u64 = MEGABYTES * 1024;
const LINE_CLEAR: [u8; 50] = [b' '; 50];

struct TimeTestResult { cycles_elapsed: u64, bytes_processed: Option<u64> }

type TimeTest = fn() -> TimeTestResult;
struct RepetitionTester {
    tests: Vec<(TimeTest, &'static str)>
}

impl RepetitionTester {
    fn new() -> Self {
        Self {
            tests: Vec::new()
        }
    }

    fn register_test(&mut self, test: TimeTest, test_name: &'static str) {
        self.tests.push((test, test_name));
    }

    fn run_tests(&self) {
        let cpu_freq = get_cpu_frequency_estimate(100);
        let mut stdout = stdout();
        let max_cycles_to_wait = (MAX_WAIT_TIME_SECONDS * cpu_freq as f64) as u64;

        for (do_test, test_name) in &self.tests {
            let mut total_cycles = 0u64;
            let mut total_bytes = 0u64;
            let mut cycles_since_last_min = 0u64;
            let mut min = TimeTestResult { cycles_elapsed: u64::MAX, bytes_processed: None };
            let mut max = TimeTestResult { cycles_elapsed: u64::MIN, bytes_processed: None };
            let mut iterations = 0;

            println!("====== {test_name} ======");
            loop {
                iterations += 1;

                let test_result = (*do_test)();
                cycles_since_last_min += test_result.cycles_elapsed;
                total_cycles += test_result.cycles_elapsed;
                if let Some(bytes_processed) = test_result.bytes_processed {
                    total_bytes += bytes_processed;
                }

                if test_result.cycles_elapsed > max.cycles_elapsed { max = test_result; }
                else if test_result.cycles_elapsed < min.cycles_elapsed {
                    cycles_since_last_min = 0;

                    // printing through stdout with print! and println! only actually flush the buffer when
                    // a newline is encountered. If we want to use carriage return and update a single line
                    // then we have to write to and flush stdout manually.
                    _ = stdout.write_all(&LINE_CLEAR);
                    let seconds = test_result.cycles_elapsed as f64 / cpu_freq as f64;
                    _ = stdout.write_all(format!(
                        "\rMin: {} ({:.4}ms)",
                        test_result.cycles_elapsed,
                        seconds * 1000.0
                    ).as_bytes());

                    if let Some(bytes_processed) = test_result.bytes_processed {
                        let gb_per_second = bytes_processed as f64 / GIGABYTES as f64 / seconds;
                        _ = stdout.write_all(format!(" {:.4}gb/s", gb_per_second).as_bytes());
                    }

                    _ = stdout.flush();

                    min = test_result;
                }

                if cycles_since_last_min > max_cycles_to_wait {
                    break;
                }
            }

            println!();

            let seconds = max.cycles_elapsed as f64 / cpu_freq as f64;
            print!("Max: {} ({:.4}ms)", max.cycles_elapsed, seconds * 1000.0);
            if let Some(bytes_processed) = max.bytes_processed {
                print!(" {:.4}gb/s", bytes_processed as f64 / GIGABYTES as f64 / seconds);
            }
            println!();

            let avg_cycles = total_cycles / iterations;
            let avg_seconds = avg_cycles as f64 / cpu_freq as f64;
            print!("Avg: {} ({:.4}ms)", avg_cycles, avg_seconds * 1000.0);
            if total_bytes > 0 {
                let avg_bytes = total_bytes / iterations;
                print!(" {:.4}gb/s", avg_bytes as f64 / GIGABYTES as f64 / avg_seconds);
            }
            println!();
            println!();
        }
    }
}

fn read_with_fs_read() -> TimeTestResult {
    let file_size = fs::metadata(FILENAME).expect("failed to read file metadata").file_size();

    let cycles_begin = read_cpu_timer();
    _ = fs::read(FILENAME);
    let cycles_elapsed = read_cpu_timer() - cycles_begin;

    TimeTestResult { cycles_elapsed, bytes_processed: Some(file_size) }
}

fn main() {
    let mut repetition_tester = RepetitionTester::new();
    repetition_tester.register_test(read_with_fs_read, "read with fs read");
    repetition_tester.run_tests();
}
