@echo off
title TMStore - installer
cd /d "%~dp0"

if not exist "node_modules\.bin\tauri.cmd" (
  echo [TM] Tauri CLI not found. Run: npm install
  if /I not "%~1"=="--no-pause" pause
  exit /b 1
)

call "%~dp0build-env.bat"
if errorlevel 1 goto :fail

echo [TM] Building Tauri release + NSIS installer...
if exist "src-tauri\target-msvc\release\bundle\nsis\*.exe" del /Q "src-tauri\target-msvc\release\bundle\nsis\*.exe"
call "node_modules\.bin\tauri.cmd" build
if errorlevel 1 goto :fail

copy /Y "src-tauri\target-msvc\release\tm-launcher.exe" "TMStore.exe" >nul
if errorlevel 1 goto :fail

set "SETUP="
for /f "usebackq delims=" %%f in (`powershell -NoProfile -Command "Get-ChildItem 'src-tauri\target-msvc\release\bundle\nsis\*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName"`) do set "SETUP=%%f"
if not defined SETUP (
  echo [TM] Installer not found in src-tauri\target-msvc\release\bundle\nsis
  goto :fail
)

copy /Y "%SETUP%" "TMStore Setup.exe" >nul
if errorlevel 1 goto :fail

echo.
echo [TM] Done: "TMStore.exe" and "TMStore Setup.exe"
if /I not "%~1"=="--no-pause" pause
exit /b 0

:fail
echo.
echo [TM] Installer build failed.
if /I not "%~1"=="--no-pause" pause
exit /b 1
