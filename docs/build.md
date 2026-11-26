# Building Lutra

The native browser uses the published Servo 0.7.0 crate. Earlier builds failed before compilation because Cargo attempted to check out the entire upstream repository on Windows, including long paths in the web platform tests.

The CI workflow uses a short target directory, Git long-path support, Rust 1.97.1 and LLVM 20.1. Build concurrency is limited to two Cargo jobs to reduce build-machine memory pressure. This is unrelated to browser runtime RAM use.

Artifacts are uploaded only after compilation and the configuration check succeed. The generated dependency lockfile and SHA-256 hashes are included. Hashes support integrity checks; they are not signatures. The lockfile is not yet checked into the repository, so builds are not guaranteed reproducible.

The current browser is experimental. A successful CI run is not a security audit or a rendering test.
