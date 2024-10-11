# Repetition Tests
This directory contains various suites of repetition-based performance tests.
Every test in a given suite is repeatedly run until no faster time is recorded within a given interval (e.g. 5 seconds).
Once a repetition test has completed, the minimum, maximum, and average performance results are printed and the repetition tester moves to the next test in the suite.

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

It's worth noting that sometimes the reported data throughput is misleading as many tests count `buffer_size` loop iterations even when operating on multiple bytes of data from the buffer each iteration.
That is to say, many tests report how long it takes to get through `buffer_size` iterations rather than how long it takes to process `buffer_size` bytes.

Many test suites run forever until manually terminated.
This is useful if you want to inspect the process with something like [Windows' builtin performance monitor](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-server-2008-r2-and-2008/cc749154(v%3dws.11)) to monitor things like the number of page faults generated.

My personal results and analyses of many of these tests can be found in `analysis.txt`, though note that these tests and their results are highly CPU-dependent and thus are designed to target and probe my AMD Ryzen 9 5900x (Zen 3).

## Build Dependencies
- [nasm](https://www.nasm.us/) - many of the following repetition tests link directly to ASM so ensure the netwide assembler is installed and added to your PATH.

## Suites

### asm_buffer_writes
This suite was more of just a sandbox used to figure out how to link assembly with Rust.
It tests variations of a simple assembly loop that writes values to a buffer.
- `mov_all_bytes_asm` - Just a normal assembly loop that moves values into a buffer.
- `nop_all_bytes_asm` - Replaced `mov` instruction with a 3-byte `nop`.
- `cmp_all_bytes_asm` - Replaced `mov` instruction with a `cmp` instruction.
- `dec_all_bytes_asm` - Loop that just decrements a counter then returns.

You may find an interesting result if you run this suite: `mov_all_bytes_asm` _may_ be measurably slower than the other asm functions, which should all have comparable run times.
At least on my AMD Ryzen 9 5900X, I believe this is happening because the processor's load/store unit can only perform a maximum of two memory stores (one if it's a 128- or 256-bit store) per cycle whereas the integer execution unit can perform four simple integer operations (subtractions in this case) per cycle.
That's just the best guess I can give right now with my limited level of experience with this sort of analysis.

I haven't verified this but it would make sense if `nop` didn't produce any micro-ops that the backend had to execute.

To run:
```
cargo r --profile reptest --bin reptest_asm_buffer_writes
```

### asm_nop_loops
This suite of tests demonstrates that we can bottleneck CPU performance on the front end by forcing it to decode instruction streams containing various amounts and sizes of `nop` instructions.
Since `nop` instructions likely don't get decoded into any micro-op, we can stall out the micro-op queue and thus the back end can't do any work for several cycles.

To run:
```
cargo r --profile reptest --bin reptest_asm_nop_loops
```

### branch_prediction
Tests investigating the behavior of branch prediction and the performance cost of branch misprediction.
There is only one asm function that loops over a buffer and for each value conditionally jumps over a `nop` if it's not zero. 
Each test is just a call to this asm function using a buffer filled with different patterns of data.
Perhaps unsurprisingly, running the code using buffers filled with random data incurs more branch mispredictions, at least on my AMD Ryzen 9 5900X.
Newer chips with more powerful branch predictors may perform somewhat better but probably not as well as the other non-randomized tests in the suite.

To run:
```
cargo r --profile reptest --bin reptest_branch_prediction
```

### cache_size
Suite investigating various CPU cache sizes using power-of-two address masks.
When a consecutive test is significantly slower than the previous test you know you've probably forced the CPU to grab data from the next cache level or from RAM.
It also highlights just how fast cache operations are compared to loads from RAM.

To run:
```
cargo r --profile reptest --bin reptest_cache_size
```

### code_alignment
Tests investigating the performance of code aligned to various byte boundaries.
The results should showcase the penalties of executing misaligned code or code that straddles cache line boundaries.

To run:
```
cargo r --profile reptest --bin reptest_code_alignment
```

### file_reads
Performance tests of various file read methods:
- A simple, baseline write to all bytes of a buffer as a control
- Rust's `fs::read`
- A buffered read
- libc `fread`
- Windows' `ReadFile`

Most of the tests in this suite have two versions that are run back-to-back:
- A version using a shared buffer among all tests and test invocations.
- A version using a newly-allocated buffer per test invocation.

A run of this suite should highlight the performance impacts that page faults can have.
See the `probe_page_fault_behavior` binary crate for a more in-depth exploration of page faulting behavior on Windows.

To run:
```
cargo r --profile reptest --bin reptest_file_reads [file_name]
```
- `file_name` - The name of a decently-sized file. A large file can be easily generated by running the haversine generator to create `haversine_pairs.json` (2.5 million pairs will produce a JSON file of ~250mb).

### probe_read_exec_ports
This suite is designed to probe the number of CPU execution ports that can perform memory loads/reads by testing the performance of functions that vary in the number of memory read `mov`s they do per loop iteration.
When performance stops rising, you know you've maxed out the number of loads that can be performed per cycle.

For example, on my AMD Ryzen 9 5900X, performance maxes out at three reads per iteration.
Checking the software optimization guide for AMD family 19h (Zen 3) processors confirms in section 2.12 that the load-store unit has three pipelines, each of which can do one memory operation per cycle, and that all three operations can be loads.

To run:
```
cargo r --profile reptest --bin reptest_read_exec_ports
```

### probe_write_exec_ports
Tests that probe the number of CPU execution ports that can perform memory writes/stores.
These tests measure the performance of functions that vary in the number of memory write `mov`s they do per loop iteration.
As with probe_read_exec_ports, when performance stops rising, you know you've maxed out the number of writes that can be performed per cycle.

On my AMD Ryzen 9 5900X, performance maxes out at two writes per iteration.
Again, checking section 2.12 of the software optimization guide for AMD family 19h (Zen 3) processors, two of the three load-store execution ports can be used for memory writes per cycles (or just one per cycle if the store is 128- or 256-bits).

To run:
```
cargo r --profile reptest --bin reptest_write_exec_ports
```

### read_widths
Performance tests of reading through a buffer using various read widths both without vector registers (32-bit, 64-bit) and with vector registers (128-bit (xmm), 256-bit (ymm)).
Note that these tests were designed with my specific processor in mind, so each loop iteration only does a max of three reads (see probe_read_exec_ports for more details).
My CPU does NOT support AVX-512 so there are no tests for it.

To run:
```
cargo r --profile reptest --bin reptest_read_widths
```

### simple_buffer_writes
This suite just contains two different tests, each of which allocs a buffer then reads through it in a different direction (i.e. forward/backward).
It's another test of Windows' page faulting behavior and demonstrates (at least on Win 11 as of 2024-09-29) that writing bytes forward through a buffer that hasn't been mapped to physical RAM yet is actually faster than writing bytes backward through a buffer.
See the `probe_page_fault_behavior` binary crate for more information.
Basically, when writing through pages in a forward direction Windows applies a 16-page premap after touching the first 16 pages, but it does _not_ do this when writing backwards through unmapped pages.

To run:
```
cargo r --profile reptest --bin reptest_simple_buffer_writes
```

### unaligned_load_penalties
A suite of tests investigating the performance penalties of loading data that straddles cache line boundaries, forcing the CPU to load additional cache lines and stitch results together from multiple load buffers.
These tests were designed to load data purely from L1 cache, assuming an L1 data cache size of 32kb.
We read 1gb worth of bytes from a portion of the L1 data cache by repeatedly loading data within a small region of memory, looping over the region multiple times per test.

To run:
```
cargo r --profile reptest --bin reptest_unaligned_load_penalties
```
