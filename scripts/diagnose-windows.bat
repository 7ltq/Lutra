@echo off
setlocal
cd /d "%~dp0"
set RUST_BACKTRACE=full
set RUST_LOG=info
servo-preview.exe https://example.com > "%~dp0lutra-crash.log" 2>&1
set "LUTRA_EXIT=%ERRORLEVEL%"
echo Exit code: %LUTRA_EXIT%>> "%~dp0lutra-crash.log"
type "%~dp0lutra-crash.log"
pause
exit /b %LUTRA_EXIT%
