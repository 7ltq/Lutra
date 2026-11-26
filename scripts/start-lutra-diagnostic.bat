@echo off
setlocal
cd /d "%~dp0"
set "RUST_BACKTRACE=1"
set "RUST_LOG=warn"
echo Starting Lutra. Diagnostics are saved locally to lutra-startup.log.
"%~dp0servo-preview.exe" > "%~dp0lutra-startup.log" 2>&1
set "EXIT_CODE=%ERRORLEVEL%"
echo Exit code: %EXIT_CODE%>> "%~dp0lutra-startup.log"
type "%~dp0lutra-startup.log"
echo.
echo Lutra exited with code %EXIT_CODE%.
echo Review lutra-startup.log before sharing it.
pause
exit /b %EXIT_CODE%
