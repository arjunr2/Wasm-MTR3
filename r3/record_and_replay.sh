#!/bin/bash

instprefix=inst
replayprefix=replay

# Check if an argument is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <wasm-module-to-record> <args-for-module-run>"
    exit 1
fi

wasmmod=$1
wasm-validate --enable-threads $wasmmod

shift

# Run & record module
RUST_LOG=info ./target/debug/record -i $instprefix.wasm $wasmmod $@
wasm2wat --enable-threads --enable-multi-memory $instprefix.wasm -o $instprefix.wat
echo ""

# Generate replay binary
RUST_LOG=info ./target/debug/replay -o $replayprefix.wasm -w $wasmmod -d $replayprefix.out
echo ""

# Run replay binary
RUST_LOG=debug ./target/debug/runner -v 5 $replayprefix.wasm
