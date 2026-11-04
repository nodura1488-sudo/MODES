@echo off
title TMLauncher - installer
cd /d "%~dp0"

if not exist "node_modules\.bin\tauri.cmd" (
  echo [TM] Tauri CLI not found. Run: npm install
  if /I not "%~1"=="--no-pause" pause
  exit /b 1
)

rem Build the portable exe first; it also prepares the MSVC environment.
call build-release.bat --no-pause
if errorlevel 1 goto :fail
cd /d "%~dp0"

echo [TM] Building NSIS installer...
if exist "src-tauri\target-msvc\release\bundle\nsis\*.exe" del /Q "src-tauri\target-msvc\release\bundle\nsis\*.exe"
call "node_modules\.bin\tauri.cmd" build
if errorlevel 1 goto :fail

set "SETUP="
for /f "usebackq delims=" %%f in (`powershell -NoProfile -Command "Get-ChildItem 'src-tauri\target-msvc\release\bundle\nsis\*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName"`) do set "SETUP=%%f"
if not defined SETUP (
  echo [TM] Installer not found in src-tauri\target-msvc\release\bundle\nsis
  goto :fail
)

copy /Y "%SETUP%" "TMLauncher Setup.exe" >nul
if errorlevel 1 goto :fail

echo.
echo [TM] Done: "TMLauncher Setup.exe"
if /I not "%~1"=="--no-pause" pause
exit /b 0

:fail
echo.
echo [TM] Installer build failed.
if /I not "%~1"=="--no-pause" pause
exit /b 1
