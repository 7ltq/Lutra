# Resource policy and offline rendering check

Servo 0.7 exposes resource callbacks on both WebViewDelegate and ServoDelegate. Lutra cancels HTTP requests observed by these callbacks and can optionally block a small built-in domain list or user-defined hostnames. Domain matching uses exact names and dot-delimited subdomains. The settings and counters are session-local. These callbacks do not prove complete coverage of every protocol, cache path, or engine request. This is not a full ad blocker, a VPN, a DNS filter or fingerprint protection.

The native Windows workflow now launches two separate smoke processes: a blank-startup check and a rendering fixture check. The latter intercepts a reserved .invalid host only when --smoke-render is passed; other observed requests are canceled in this mode. It requires the expected document title, completion and a frame presentation. It does not verify individual pixels or interaction with page controls.

Do not use the preview for sensitive browsing. Verify rendering, request filtering and dialogs on Windows before release. No claim is made that Peldi Mode, downloads, extensions, permanent profiles, sandboxing or broad website compatibility are finished.
