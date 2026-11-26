param(
    [Parameter(Mandatory = $true)]
    [string]$Executable
)

$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path $Executable).Path
$logs = Join-Path (Get-Location) 'startup-logs'
New-Item -ItemType Directory -Path $logs -Force | Out-Null
$env:RUST_BACKTRACE = '1'
$env:RUST_LOG = 'warn'

foreach ($check in @(
    @{ Name = 'startup'; Argument = '--smoke-startup'; Marker = 'Startup smoke check passed' },
    @{ Name = 'render'; Argument = '--smoke-render'; Marker = 'Rendering fixture check passed' }
)) {
    $stdout = Join-Path $logs "$($check.Name)-stdout.txt"
    $stderr = Join-Path $logs "$($check.Name)-stderr.txt"
    $process = Start-Process -FilePath $exe -ArgumentList $check.Argument -WorkingDirectory (Split-Path $exe) -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
    try {
        if (-not $process.WaitForExit(60000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            throw "Lutra did not complete the $($check.Name) check within 60 seconds."
        }
        $process.Refresh()
        if ($process.ExitCode -ne 0) { throw "Lutra exited with code $($process.ExitCode)." }
        if (-not (Select-String -Path $stderr -Pattern $check.Marker -SimpleMatch -Quiet)) {
            throw "The $($check.Name) checkpoint was not reached."
        }
    } finally {
        if (Test-Path $stdout) { Get-Content $stdout }
        if (Test-Path $stderr) { Get-Content $stderr }
        $process.Dispose()
    }
}
