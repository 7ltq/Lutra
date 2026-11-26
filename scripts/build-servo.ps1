param(
    [string]$TargetDirectory = (Join-Path $PSScriptRoot '..\target')
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$target = [IO.Path]::GetFullPath($TargetDirectory)
$manifest = Join-Path $repo 'crates\servo-integration\Cargo.toml'
$env:CARGO_NET_GIT_FETCH_WITH_CLI = 'true'
$env:CARGO_INCREMENTAL = '0'
if (-not $env:CARGO_BUILD_JOBS) {
    $env:CARGO_BUILD_JOBS = '2'
}

rustup toolchain install 1.97.1 --profile minimal
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo +1.97.1 build --release --manifest-path $manifest --target-dir $target --features servo-experimental --bin servo-preview
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Output (Join-Path $target 'release\servo-preview.exe')
