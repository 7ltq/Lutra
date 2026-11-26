@echo off
cd /d "%~dp0"
cargo test --workspace
if errorlevel 1 exit /b 1
cargo check --workspace
exit /b %errorlevel%
