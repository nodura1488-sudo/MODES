@echo off
title TDEV Launcher
cd /d "%~dp0"

rem Fast launcher run: double-click this file.
rem If dependencies are missing (first run) they are installed automatically.

if not exist "node_modules\electron\dist\electron.exe" (
  echo [TDEV] First run: installing dependencies, this may take a few minutes...
  call npm install --no-audit --no-fund
  if errorlevel 1 goto :fail
)

echo [TDEV] Starting TDEV Launcher...
"node_modules\.bin\electron.cmd" .
if errorlevel 1 goto :fail
exit /b 0

:fail
echo.
echo [TDEV] ERROR! Check the messages above.
echo [TDEV] Is Node.js installed? https://nodejs.org
pause
