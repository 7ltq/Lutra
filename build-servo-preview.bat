@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -File "scripts\build-servo.ps1" -TargetDirectory "%~dp0target"
if errorlevel 1 goto fail
"target\release\servo-preview.exe" https://servo.org
exit /b %errorlevel%
:fail
echo Build failed. Review the compiler output above.
pause
exit /b 1
