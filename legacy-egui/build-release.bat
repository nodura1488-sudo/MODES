@echo off
title TDEV Launcher - build
cd /d "%~dp0"
echo [TDEV] Building release...
cargo build --release
if errorlevel 1 goto :fail
copy /Y "target\release\tdev-launcher.exe" "TDEV Launcher.exe" >nul
echo.
echo [TDEV] Done: "TDEV Launcher.exe"
pause
exit /b 0

:fail
echo.
echo [TDEV] Build failed.
pause
exit /b 1
