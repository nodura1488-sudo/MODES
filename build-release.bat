@echo off
title Lavunio - build
cd /d "%~dp0"

call "%~dp0build-env.bat"
if errorlevel 1 goto :fail

echo [Lavunio] Building Tauri release...
cd /d "%~dp0src-tauri"
"%CARGO%" build --release
if errorlevel 1 goto :fail

copy /Y "target-msvc\release\lavunio.exe" "..\Lavunio.exe" >nul
if errorlevel 1 goto :fail

echo.
echo [Lavunio] Done: "Lavunio.exe"
if /I not "%~1"=="--no-pause" pause
exit /b 0

:fail
echo.
echo [Lavunio] Build failed.
if /I not "%~1"=="--no-pause" pause
exit /b 1
