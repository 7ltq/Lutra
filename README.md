<p align="center">
  <img src="assets/lutra.png" width="96" height="96" alt="Lutra">
</p>

<h1 align="center">Lutra</h1>

<p align="center">
  <strong>A browser should work for you.</strong><br>
  An independent web browser built with Rust and Servo.
</p>

<p align="center">
  <a href="https://github.com/7ltq/Lutra/actions/workflows/windows.yml"><img alt="Windows build" src="https://github.com/7ltq/Lutra/actions/workflows/windows.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/7ltq/Lutra/actions/workflows/servo-windows.yml"><img alt="Experimental Servo build" src="https://github.com/7ltq/Lutra/actions/workflows/servo-windows.yml/badge.svg?branch=development%2Fprivate-session"></a>
</p>

<p align="center">
  <a href="#why-lutra">Why Lutra?</a> &nbsp;·&nbsp;
  <a href="#where-we-are">Development</a> &nbsp;·&nbsp;
  <a href="#building-lutra">Build</a> &nbsp;·&nbsp;
  <a href="https://github.com/7ltq/Lutra/issues">Report a problem</a>
</p>

---

## Why Lutra?

A web browser should not become another source of unnecessary data collection. Nor should it consume more of your computer's resources than it needs.

Lutra is an attempt to build a browser around those ideas. We are starting with Rust, the Servo web engine and Windows.

### Privacy without promises we cannot keep

Our aim is to reduce what the browser exposes and retains. Lutra does not contain a first-party telemetry service. This does not prevent websites, the engine or other software from communicating over the network.

Lutra does not hide your IP address. It is not an anonymity tool.

### Less work for your computer

A browser should leave resources available for the things you use your computer for. We are working on event-driven rendering, lazy tab allocation and a shared graphics context.

We have not established the memory or CPU consumption of the complete browser. We intend to publish measurements, not estimates.

## Where we are

> [!IMPORTANT]
> Lutra is in development. The Windows interface has been built and tested in CI. The native Servo browser has not yet been verified. Do not use Lutra for sensitive browsing.

| Work | State |
| :--- | :--- |
| Windows interface and Rust unit tests | Verified in CI |
| Native page rendering | Awaiting engine build and runtime verification |
| Native tabs and local new-tab page | In development |
| Temporary session storage and restricted engine APIs | In development |
| Site isolation and operating-system sandboxing | Not implemented |
| Tracking protection and fingerprinting resistance | Not implemented |
| Memory and CPU measurements | Not yet available |

Changes to the native browser are reviewed separately from the default branch.

- [Native tabs](https://github.com/7ltq/Lutra/pull/1)
- [Private sessions](https://github.com/7ltq/Lutra/pull/2)
- [Build results](https://github.com/7ltq/Lutra/actions)

## Building Lutra

Windows is our first target. Install [Rust](https://rustup.rs/) and the Microsoft C++ build tools. For the native renderer, follow the [Servo build documentation](https://book.servo.org/).

From the repository directory:

```powershell
.\build.bat
```

This builds the experimental interface on the default branch. To run its tests:

```powershell
.\test.bat
```

The native Servo renderer is developed on separate branches. Its build instructions and current limitations are documented there. A successful build is not a security audit or a test of page rendering.

Read more about [measuring resource use](docs/performance.md).

## Help improve Lutra

If something does not work, [open an issue](https://github.com/7ltq/Lutra/issues/new). Include the revision, your operating system and the steps needed to reproduce the problem. Leave out credentials, private browsing history and personal files.

Lutra is an independent project. It is not affiliated with Mullvad, the Tor Project or Servo.

<p align="center">
  <sub>Privacy is a goal. Evidence is how we measure progress.</sub>
</p>
