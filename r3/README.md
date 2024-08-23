# R3 Implementation

The R3 wrapper is written in Rust and uses a wasm instrumentation library written in C++ (see `../wasm-instrument` submodule).

This directory consists of three binary packages --- `record`, `reduce`, and `replay` --- to perform each stage of the pipeline, 
and a `runner` package to run replays.

Run `cargo build` to build all stages or `cargo build -p <package>` to build specific stages

## Generating/Running replays

The `record_and_replay.sh` script records the provided program, generates the replay in a file `replay.wasm`, and then runs it.

To rerun replay files, use the build `runner` binary (see `-h` for help)

## Implementation Overview
TBD

