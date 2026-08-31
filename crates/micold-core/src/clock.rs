//! A monotonic, suspend-inclusive clock (feature 028, data-model G3).
//!
//! The idle rule in `micold-daemon` measures 30 continuous minutes without a client. "Continuous"
//! has to include the time the machine spends asleep — a laptop closed for an hour with nothing
//! connected has been idle for an hour — which rules out `std::time::Instant` on every platform
//! this ships to: it stops on suspend on Linux and macOS.
//!
//! So the reading is the one thing in this feature that is platform-split, behind a single
//! `cfg`-free signature (Constitution Principle VI): `CLOCK_BOOTTIME` on Linux,
//! `mach_continuous_time()` on macOS, `GetTickCount64()` on Windows.
//!
//! Filled in by T008; the module exists from Phase 1 so that every later task touches one file.
