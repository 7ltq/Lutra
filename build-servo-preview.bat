@echo off
setlocal
cd /d "%~dp0"
cargo run --release --manifest-path crates/servo-integration/Cargo.toml --target-dir target --features servo-experimental --bin servo-preview -- https://servo.org
if errorlevel 1 echo Servo preview build or run failed.
pause
