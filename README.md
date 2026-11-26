# Lutra

A web browser in development. Built with Rust and Servo.

The aim is to give people more control over their browsing, without making the browser a source of unnecessary data collection. It should use no more resources than it needs.

## Before you use Lutra

Lutra is an experimental project, not a browser ready for daily use. Do not rely on it for sensitive browsing.

The interface and the web engine currently run separately. Site isolation, sandboxing, tracking protection, storage partitioning and resistance to browser fingerprinting have not been implemented. The privacy settings in the source code do not yet enforce these protections.

Lutra does not include its own telemetry service. This does not prevent websites, network providers or engine dependencies from communicating over the network. Lutra does not hide your IP address.

We have not established how much memory the browser uses. We intend to publish measurements rather than estimates.

## Build on Windows

Install Rust and the Visual Studio C++ build tools. For the engine, also follow the [Servo Windows build instructions](https://book.servo.org/building/windows.html) and [embedding instructions](https://book.servo.org/embedding/overview.html).

- `build.bat`: build the interface.
- `test.bat`: run the workspace tests.
- `build-all.bat`: build the experimental Servo renderer and start the interface.

The experimental renderer targets Servo v0.7.0. The Windows workflow tests the interface on each push. The renderer build is opt-in through the workflow dispatch input. A green interface build does not establish that Servo works.

See [the build results](https://github.com/7ltq/Lutra/actions) for the state of each revision.

## Development

The next step is to render web pages inside the browser window. After that, we can measure resource use and implement browser protections at the engine boundary.

Lutra is an independent project. It is not affiliated with Mullvad, the Tor Project or Servo.
