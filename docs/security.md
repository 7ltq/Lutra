# Security boundaries

Lutra is not ready for sensitive browsing.

The current experimental browser has the following restrictions:

- The address parser accepts HTTPS destinations without URL credentials. Unsupported schemes are rejected rather than opened through the operating system.
- Web-view navigation requests are checked against the same policy.
- Permission requests received through the Servo delegate are denied.
- Browser shortcuts and address-bar keyboard input are separated from page keyboard input.

These restrictions do not establish that every request uses HTTPS, that every permission path is covered, or that pages cannot access one another's data. They have not undergone a security audit.

Web views share the same engine and graphics context. There is no claimed site isolation, sandbox, independent storage partition, or anonymity guarantee. The engine is an upstream dependency with its own security and compatibility limitations.

Before release, the project needs verified process and storage boundaries, a comprehensive network policy, an update mechanism, input-routing tests, and a review of engine callbacks and resource handling.

## Manual checks

Use non-sensitive test pages only. Verify that a new tab has no page, that HTTPS navigation works, and that HTTP, file, JavaScript and external-protocol destinations are rejected. Check that switching tabs preserves each page, closing a tab removes its web view, and typing into the address bar does not send characters to web content. Check permission requests, redirects, window resizing, and shutdown.

Report the revision and the exact steps to reproduce a problem. Do not include private browsing history, credentials or personal files.
