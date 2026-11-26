# Lutra

A web browser in development. Built with Rust and Servo.

Browsing should not require giving up privacy. A browser should use no more resources than it needs.

## Before you use Lutra

Lutra is an experimental project. Do not rely on it for sensitive browsing.

The native browser now has a tab strip and a local new-tab page. Opening an empty tab does not create a web view or request a default website. The engine itself is initialized at startup. Only the selected web view is painted.

The experimental navigation policy rejects non-HTTPS addresses and denies permission requests. This is not a guarantee that all network traffic uses HTTPS. It does not provide anonymity.

The tabs currently share a Servo engine and rendering context. They are not isolated security boundaries. Site isolation, operating-system sandboxing, storage partitioning, tracking protection and fingerprinting resistance remain unimplemented. Do not use this build as a substitute for a privacy browser.

The experimental browser requests temporary storage from Servo and assigns a new temporary data directory to each run. It attempts to remove that directory when it exits. Files may remain after a crash or failed cleanup. Selected device and communication APIs are disabled through engine preferences. These changes are not a promise of anonymous browsing or secure deletion.

Lutra has no first-party telemetry service. Websites and other components can still communicate over the network.

We have not established the memory or CPU use of the complete browser. We intend to publish measurements, not estimates.

## Build on Windows

Install Rust, the Visual Studio C++ build tools and the prerequisites in the [Servo build documentation](https://book.servo.org/).

Run `build-all.bat` to build and launch the native experimental browser. The legacy interface can still be built with `build.bat`. Use `test.bat` for the workspace tests.

The experimental engine uses the published Servo 0.7.0 Cargo package. We do not download the entire Servo source repository to build the browser. The Windows build runs a configuration and session cleanup check before publishing an artifact. This check does not start the graphics engine or verify page rendering. See [build results](https://github.com/7ltq/Lutra/actions) for each revision. Compilation is not a runtime or security test.

## Development

See [security boundaries](docs/security.md) and [resource measurement](docs/performance.md) for what the current implementation does and does not provide.

Lutra is an independent project. It is not affiliated with Mullvad, the Tor Project or Servo.

If the native browser closes on startup, run `start-lutra-diagnostic.bat` in the extracted CI package. Review the local log before sharing it. The startup check in CI does not verify web-page rendering.
