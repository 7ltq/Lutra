@echo off
setlocal
cd /d "%~dp0"
cargo build --release --manifest-path crates/servo-integration/Cargo.toml --target-dir target --features servo-experimental --bin servo-preview
if errorlevel 1 goto fail
cargo build --release -p browser-ui
if errorlevel 1 goto fail
"target\release\browser-ui.exe"
exit /b %errorlevel%
:fail
echo Build failed. Review the compiler output above.
pause
exit /b 1
