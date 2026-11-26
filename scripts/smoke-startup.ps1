param(
    [Parameter(Mandatory = $true)]
    [string]$Executable
)

$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path $Executable).Path
$logs = Join-Path (Get-Location) 'startup-logs'
New-Item -ItemType Directory -Path $logs -Force | Out-Null
$stdout = Join-Path $logs 'stdout.txt'
$stderr = Join-Path $logs 'stderr.txt'
$env:RUST_BACKTRACE = '1'
$env:RUST_LOG = 'warn'
$process = Start-Process -FilePath $exe -ArgumentList '--smoke-startup' -WorkingDirectory (Split-Path $exe) -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
try {
    if (-not $process.WaitForExit(60000)) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        throw 'Lutra did not complete its startup check within 60 seconds.'
    }
    $process.Refresh()
    if ($process.ExitCode -ne 0) {
        throw "Lutra exited with code $($process.ExitCode)."
    }
    if (-not (Select-String -Path $stderr -Pattern 'Startup smoke check passed' -SimpleMatch -Quiet)) {
        throw 'The first-frame checkpoint was not reached.'
    }
} finally {
    if (Test-Path $stdout) { Get-Content $stdout }
    if (Test-Path $stderr) { Get-Content $stderr }
    $process.Dispose()
}
