# Wasm R3: Record-Reduce-Replay ISA-agnostic binaries

## Overview

TBD

Refer to the [r3](r3) directory for implementation details and instructions on running R3

## Support

Depending on the Wasm interface used by recorded programs, host-level support may be required to faithfully replay some binaries.
Primary support is currently geared towards replaying Linux programs against [WALI](https://github.com/arjunr2/WALI.git).

| Feature | Status | Debug? |
| ------- | ------ | ----- |
| Threading | TBD | N |
| Futex | TBD | N |
| Memory-Mapping | :heavy_check_mark: | N |
| File Descriptors | TBD | N |
| PIDs/TIDs | TBD | N |
| Signals | TBD | N |
| Stdout Write | :heavy_check_mark: | Y |

The **Debug** column highlights features only added for debugging outputs (may be stubbed out by host if not needed).
