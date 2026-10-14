@echo off
setlocal
set NEOTHESIA_HEADLESS_CHECK=
cd /d "%~dp0"
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\prepare-desktop.ps1"
if errorlevel 1 (
  echo Neothesia could not prepare the current desktop version.
  pause
  exit /b 1
)
start "" /D "%~dp0target\debug" "%~dp0target\debug\neothesia-desktop.exe" %*
