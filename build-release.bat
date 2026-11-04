@echo off
title TMLauncher - build
cd /d "%~dp0"

rem Prefer the MSVC toolchain: it produces a single exe without WebView2Loader.dll.
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" (
  for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSPATH=%%i"
)
if defined VSPATH if exist "%VSPATH%\VC\Auxiliary\Build\vcvars64.bat" call "%VSPATH%\VC\Auxiliary\Build\vcvars64.bat" >nul

set "CARGO="
for /d %%d in ("C:\Program Files\Rust stable MSVC*") do if exist "%%~d\bin\cargo.exe" set "CARGO=%%~d\bin\cargo.exe"
if not defined CARGO if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "CARGO=%USERPROFILE%\.cargo\bin\cargo.exe"
if not defined CARGO set "CARGO=cargo"
for %%i in ("%CARGO%") do set "RUST_BIN=%%~dpi"
if defined RUST_BIN set "PATH=%RUST_BIN%;%PATH%"
if "%CARGO%"=="cargo" (
  where cargo >nul 2>nul
  if errorlevel 1 (
    echo [TM] Rust/cargo not found. Install Rust: https://rustup.rs
    pause
    exit /b 1
  )
) else (
  if not exist "%CARGO%" (
    echo [TM] Rust/cargo not found. Install Rust: https://rustup.rs
    pause
    exit /b 1
  )
)

echo [TM] Building Tauri release...
cd /d "%~dp0src-tauri"
set "CARGO_TARGET_DIR=target-msvc"
"%CARGO%" clean -p tm-launcher >nul 2>nul
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
