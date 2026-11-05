@echo off
rem Shared build environment: MSVC toolchain + Rust + cargo target dir.
cd /d "%~dp0"

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
    exit /b 1
  )
) else (
  if not exist "%CARGO%" (
    echo [TM] Rust/cargo not found. Install Rust: https://rustup.rs
    exit /b 1
  )
)

set "CARGO_TARGET_DIR=target-msvc"
exit /b 0
