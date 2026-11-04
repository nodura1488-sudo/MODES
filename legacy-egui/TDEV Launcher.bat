@echo off
title TDEV Launcher
cd /d "%~dp0"

rem TDEV Launcher (Rust). Double-click this file.
rem On first run the release build is compiled automatically.

if exist "TDEV Launcher.exe" (
  start "" "TDEV Launcher.exe"
  exit /b 0
)

if exist "target\release\tdev-launcher.exe" (
  start "" "target\release\tdev-launcher.exe"
  exit /b 0
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo [TDEV] Rust/cargo not found.
  echo Install Rust: https://rustup.rs
  echo.
  pause
  exit /b 1
)

echo [TDEV] First run: building release, this may take a few minutes...
cargo build --release
if errorlevel 1 goto :fail

if exist "target\release\tdev-launcher.exe" (
  start "" "target\release\tdev-launcher.exe"
  exit /b 0
)

:fail
echo.
echo [TDEV] Build failed. Check the messages above.
pause
exit /b 1
