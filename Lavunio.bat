@echo off
title Lavunio
cd /d "%~dp0"

rem Lavunio (Tauri). Double-click this file.

if exist "Lavunio.exe" (
  start "" "Lavunio.exe"
  exit /b 0
)

if exist "src-tauri\target-msvc\release\lavunio.exe" (
  start "" "src-tauri\target-msvc\release\lavunio.exe"
  exit /b 0
)

if exist "src-tauri\target\release\lavunio.exe" (
  start "" "src-tauri\target\release\lavunio.exe"
  exit /b 0
)

echo [Lavunio] Release exe not found, building...
call build-release.bat --no-pause
if exist "Lavunio.exe" (
  start "" "Lavunio.exe"
  exit /b 0
)
pause
exit /b 1
