@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>&1
if errorlevel 1 (
  echo Install Rust and the Visual Studio C++ build tools.
  pause
  exit /b 1
)
cargo build --release -p browser-ui
if errorlevel 1 (
  echo Build failed.
  pause
  exit /b 1
)
echo Build succeeded: target\release\browser-ui.exe
pause
