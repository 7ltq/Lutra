# Building Lutra

The native browser uses the published Servo 0.7.0 crate. Earlier builds failed before compilation because Cargo attempted to check out the entire upstream repository on Windows, including long paths in the web platform tests.

The CI workflow uses Windows Server 2022 with Visual Studio 2022, a short target directory, Git long-path support, Rust 1.97.1 and LLVM 20.1.8. This matches the Visual Studio generation and LLVM series used by Servo's upstream Windows workflow. The previous build used Visual Studio 2026 with LLVM 20, which failed in mozangle because the C++ standard library headers required Clang 22 or newer. CI now checks that Clang can parse a standard C++ header before compiling Servo. Build concurrency is limited to two Cargo jobs to reduce build-machine memory pressure. This is unrelated to browser runtime RAM use.

Artifacts are uploaded only after compilation and the configuration check succeed. The generated dependency lockfile and SHA-256 hashes are included. Hashes support integrity checks; they are not signatures. The lockfile is not yet checked into the repository, so builds are not guaranteed reproducible.

The current browser is experimental. A successful CI run is not a security audit or a rendering test.

The packaged executable is also tested in a real window using `--smoke-startup`. It must initialize the engine and graphics context, submit its first interface frame, remain running briefly and exit cleanly. This checks startup, not web-page rendering or pixel correctness. Hosted runners may lack a usable graphics environment. The build package is retained for diagnosis even if the window test fails. Startup diagnostics are uploaded separately.

To diagnose a local startup failure, run `start-lutra-diagnostic.bat` from the extracted package. It saves a local log and displays the exit code. No log is sent by the browser. Review its contents before sharing it.
