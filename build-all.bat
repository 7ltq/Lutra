@echo off
setlocal
cd /d "%~dp0"
rustup toolchain install 1.97.1 --profile minimal
if errorlevel 1 goto fail
cargo +1.97.1 build --release --manifest-path crates/servo-integration/Cargo.toml --target-dir target --features servo-experimental --bin servo-preview
if errorlevel 1 goto fail
"target\release\servo-preview.exe"
exit /b %errorlevel%
:fail
echo Build failed. Review the compiler output above.
pause
exit /b 1
