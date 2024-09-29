# Libraries

## performance_metrics
A collection of functions and a global profiler useful for measuring program performance and throughput.

### Using the Global Profiler
The global profiler is intended to have minimal runtime performance impact and be easy to drop-in/drop-out of whatever code is being profiled.
The global profiler is used indirectly via several provided macros:

#### `init_profiler!`
Use this macro at the very start of a program to initialize the global profiler.

#### `end_and_print_profile_info!`
Use this macro at the very end of a program to print profiling information.
At the very least if the `profile` and `profile_function` macros aren't used or if the binary is compiled without the profiling feature enabled, the estimated runtime of the entire program will be printed.

#### `profile!`
Note that any code withing a `profile!` block will leak outside of the macro (i.e. `profile` is _not_ hygienic).
This is by design so that `profile` can be added/removed without any code modifications aside from some indentation.

```rust
// ...
// A profile section.
// The label is required but bytes_expr and no_manual_drop are not.
// Note the semicolon at the end of the section args.
profile! { "label" [bytes_expr] no_manual_drop;
    // ...
}
// ...
```
The `bytes_expr` is an expression that, when evaluated at runtime, will provide the number of bytes expected to be processed in the following code.
This number is used to provide the GB/s throughput measurement.

The following is an example of a `bytes_expr`:
```rust
// Slightly modified for brevity.
// This profile block profiles how long it takes to read the given file and will
// report the cycle count, millisecond duration, and gb/s throughput at the end of
// of the program when end_and_print_profile_info! is used.
profile! { "file read" [ fs::metadata(&haversine_json_filename).unwrap().file_size() ];
    let haversine_json = fs::read(&haversine_json_filename)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", haversine_json_filename, err));
}
```

`no_manual_drop` is an optional explicit identifier.
By default, a profile section is manually dropped with `drop` at the very end of a `profile!` block.
If the block being profiled returns a value, add the `no_manual_drop` identifier after the label and `bytes_expr` (if supplied) to allow the compiler to implicitly drop the profile section when it goes out of scope.

#### `profile_function!`
```rust
// At the moment, in order to use bytes_expr a custom label must be provided.
// The bytes_expr is not required, however.
#[profile_function("custom label", bytes_expr)]
fn my_function(...) { ... }

// The name of the function is implicitly used as the label.
#[profile_function]
fn my_other_function(...) { ... }
```

## profiling_proc_macros
Proc macro crate that implements the `profile!` and `profile_function!` macros that performance_metrics re-exports.

## repetition_tester
The repetition tester registers tests and runs each one repeatedly until no faster time is measured within a given interval, at which point the minimum, maximum, and average run times are reported.

A repetition test can be any function or closure with the signature `(&mut TestParams) -> TimeTestResult`, where `TestParams` is a generic type as determined by the data passed into `RepetitionTester::new`.
The `TestParams` are mutably shared among all tests to allow for things like buffer re-use if so desired.

Test results contain information about the number of cycles elapsed, the number of page faults reported by Windows, and the number of bytes processed.
To supply test results, the test should utilize `TimeTestSection` to gather performance-related data.
```rust
fn the_test(shared_params: &mut SharedParamsType) -> TimeTestResult {
    // ... setup ...
    let section = TimeTestSection::begin();
    // ... do stuff that should be profiled ...
    let results = section.end(bytes_processed as u64);
    // ... cleanup ...

    results
}
```

The following is an example of output for a given test:
```
FORMAT
Min/Max/Avg: cycles | milliseconds | throughput gb/s | page faults/pages mapped

Min - The fastest time
Max - The slowest time
Avg - The average time

EXAMPLE
====== 1 write per iteration ======
Min: 4194325 (0.9869ms) 3.9582gb/s 0pf
Max: 8475287 (1.9941ms) 1.9589gb/s 0pf
Avg: 4252477 (1.0006ms) 3.9041gb/s 0pf
```

To register a test, the function/closure and a label to use as the test name should be supplied.
```rust
struct SharedParams { /* ... */ }

fn the_test(shared_params: &mut SharedParams) -> TimeTestResult {
    let section = TimeTestSection::begin();
    // ...
    section.end(bytes_processed as u64)
}

fn main() {
    let mut repetition_tester = RepetitionTester::new(SharedParams { /* ... */ });
    repetition_tester.register_test(the_test, "short description/name of test");
    repetition_tester.run_tests(); // run the suite in a loop forever.
}
```
See the suites in `repetition_tests` for more examples of how to use the repetition tester.
