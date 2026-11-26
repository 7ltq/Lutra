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
$release = Join-Path $target 'release'
$build = Join-Path $release 'build'
$eglCandidates = @(Get-ChildItem -LiteralPath $build -Directory -Filter 'mozangle-*' |
    ForEach-Object { Get-ChildItem -LiteralPath $_.FullName -Recurse -File -Filter 'libEGL.dll' } |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.DirectoryName 'libGLESv2.dll') -PathType Leaf })

if ($eglCandidates.Count -eq 0) {
    throw 'The ANGLE runtime libraries were not produced by the mozangle build.'
}

$pairs = @($eglCandidates | ForEach-Object {
    $egl = $_.FullName
    $gles = Join-Path $_.DirectoryName 'libGLESv2.dll'
    [pscustomobject]@{
        Egl = $egl
        Gles = $gles
        Hash = "$((Get-FileHash -LiteralPath $egl -Algorithm SHA256).Hash):$((Get-FileHash -LiteralPath $gles -Algorithm SHA256).Hash)"
    }
})

if (@($pairs | Select-Object -ExpandProperty Hash -Unique).Count -ne 1) {
    throw 'Several different ANGLE library pairs were found. Build in a clean target directory.'
}

$pair = $pairs[0]
Copy-Item -LiteralPath $pair.Egl -Destination (Join-Path $release 'libEGL.dll') -Force
Copy-Item -LiteralPath $pair.Gles -Destination (Join-Path $release 'libGLESv2.dll') -Force
Write-Output 'ANGLE runtime libraries staged.'
Write-Output (Join-Path $release 'servo-preview.exe')
