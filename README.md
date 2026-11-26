# Lutra

Rust-first Windows browser prototype with an experimental Servo renderer.

## Build

Install stable Rust and Microsoft C++ Build Tools. Run `build.bat` for the shell, `test.bat` for tests, or `build-all.bat` for the experimental renderer and shell.

The shell and renderer are separate executables. Native in-tab rendering, sandboxing, privacy enforcement, automatic tab suspension and measured resource savings are not implemented. The preview process limit is not a RAM budget.

The Windows workflow tests and builds the shell. The experimental Servo check is manually enabled. The approved otter artwork is represented here by a reduced-resolution application icon; the original artwork and multi-resolution ICO are retained in the v0.7.0 source archive.

## Development

Prioritize native Servo integration, verified Windows builds, low idle CPU usage, measured memory consumption and isolation. No performance or security guarantees are made.
