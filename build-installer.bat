@echo off
title Lavunio - installer
cd /d "%~dp0"

if not exist "node_modules\.bin\tauri.cmd" (
  echo [Lavunio] Tauri CLI not found. Run: npm install
  if /I not "%~1"=="--no-pause" pause
  exit /b 1
)

call "%~dp0build-env.bat"
if errorlevel 1 goto :fail

echo [Lavunio] Building Tauri release + NSIS installer...
if exist "src-tauri\target-msvc\release\bundle\nsis\*.exe" del /Q "src-tauri\target-msvc\release\bundle\nsis\*.exe"
call "node_modules\.bin\tauri.cmd" build
if errorlevel 1 goto :fail

copy /Y "src-tauri\target-msvc\release\lavunio.exe" "Lavunio.exe" >nul
if errorlevel 1 goto :fail

set "SETUP="
for /f "usebackq delims=" %%f in (`powershell -NoProfile -Command "Get-ChildItem 'src-tauri\target-msvc\release\bundle\nsis\*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName"`) do set "SETUP=%%f"
if not defined SETUP (
  echo [Lavunio] Installer not found in src-tauri\target-msvc\release\bundle\nsis
  goto :fail
)

copy /Y "%SETUP%" "Lavunio Setup.exe" >nul
if errorlevel 1 goto :fail

echo.
echo [Lavunio] Done: "Lavunio.exe" and "Lavunio Setup.exe"
if /I not "%~1"=="--no-pause" pause
exit /b 0

:fail
echo.
echo [Lavunio] Installer build failed.
if /I not "%~1"=="--no-pause" pause
exit /b 1
