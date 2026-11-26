# Measuring resource use

There are no verified memory or CPU measurements for Lutra yet.

The shell and renderer are separate processes. A measurement of the shell alone is not a measurement of a working browser.

On Windows, run the following while Lutra is open:

```powershell
.\scripts\measure-resources.ps1 -RootProcessId 1234 -Seconds 30
```

Replace 1234 with the process ID of the browser interface. The script samples the interface and its observed descendants. It reports private memory, working set and CPU time locally. It does not transmit the results.

Run the same workload with an empty browser, one page, and several pages. Repeat the measurement after a fixed warm-up period and use the same pages, hardware and power settings.

The sum of process working sets can count shared pages more than once. Private memory is not the same as physical RAM. Short-lived processes can escape sampling, and PID reuse is a limitation. The script does not measure GPU memory.

The renderer limit is currently a process-count limit, not a memory limit. Automatic suspension is not enabled.

The experimental renderer now uses an offscreen GPU rendering context for the page and composites it into the same window as the navigation controls. This avoids an explicit CPU readback in the composition path. It still allocates GPU resources and has not been benchmarked. The event loop waits when the page is not animating, and uses a timed wake for animation while the window is visible.

Native tabs share an engine and GPU rendering context. Empty tabs do not allocate a web view. Selecting a tab hides the previous view; closing a tab drops its owned view. Hiding a view is not suspension, and does not promise that the page stops executing. The native tab count is capped at 32, not by measured memory use. Benchmark these changes before making claims about savings.

Session data uses a new temporary directory on each run. This changes persistence, not a measured RAM budget. It may increase cold-start work because previous session caches are not reused. Measure the difference before making performance claims.
