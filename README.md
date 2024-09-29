This repository contains coursework for Casey Muratori's [Performance-Aware Programming series](https://www.computerenhance.com/p/table-of-contents), implemented in Rust.
The [official perfaware repository](https://github.com/cmuratori/computer_enhance/tree/main/perfaware) is implemented in C/C++.

I am currently in the process of adding markdown documentation to this repository.

## Build Dependencies
- [rust toolchain](https://www.rust-lang.org/tools/install)
- [nasm](https://www.nasm.us/) (for some tests in the haversine portion)

## 8086_sim (perfaware part 1)
This is a partial decoder/simulator of the original 16-bit [Intel 8086](https://en.wikipedia.org/wiki/Intel_8086) from 1979.
I did not implement decoding and simulation of _every_ instruction the 8086 supports, nor did I implement more advanced things like the 8086's memory segmentation model; the primary goal of part 1 is to understand how x86 assembly functions and to highlight certain facts such as its instruction set being variable-length.

This project does not contain any OS/CPU-dependent code so it should be able to build and run on any platform.

## haversine (perfaware parts 2+)
An in-depth exploration of CPU performance from the perspective of a real-world problem: reading and parsing data from a large JSON file and computing the [haversine distance](https://en.wikipedia.org/wiki/Haversine_formula) between pairs of points.
This directory contains code to generate and parse JSON data, perform the haversine calculation, profile code performance, probe various aspects of CPU frontend and backend, and more.

The binaries and libraries in this directory specifically target Windows and x86_64 CPUs.
