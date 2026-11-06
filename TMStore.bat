@echo off
title TMStore
cd /d "%~dp0"

rem TMStore (Tauri). Double-click this file.

if exist "TMStore.exe" (
  start "" "TMStore.exe"
  exit /b 0
)

if exist "src-tauri\target-msvc\release\tm-launcher.exe" (
  start "" "src-tauri\target-msvc\release\tm-launcher.exe"
  exit /b 0
)

if exist "src-tauri\target\release\tm-launcher.exe" (
  start "" "src-tauri\target\release\tm-launcher.exe"
  exit /b 0
)

echo [TM] Release exe not found, building...
call build-release.bat --no-pause
if exist "TMStore.exe" (
  start "" "TMStore.exe"
  exit /b 0
)
pause
exit /b 1
