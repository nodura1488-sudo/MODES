@echo off
title TMLauncher - build
cd /d "%~dp0"

call "%~dp0build-env.bat"
if errorlevel 1 goto :fail

echo [TM] Building Tauri release...
cd /d "%~dp0src-tauri"
"%CARGO%" build --release
if errorlevel 1 goto :fail

copy /Y "target-msvc\release\tm-launcher.exe" "..\TMLauncher.exe" >nul
if errorlevel 1 goto :fail

echo.
echo [TM] Done: "TMLauncher.exe"
if /I not "%~1"=="--no-pause" pause
exit /b 0

:fail
echo.
echo [TM] Build failed.
if /I not "%~1"=="--no-pause" pause
exit /b 1
