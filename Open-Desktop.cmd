@echo off
set NEOTHESIA_HEADLESS_CHECK=
cd /d "%~dp0"
if not exist "target\debug\neothesia-desktop.exe" (
  cd /d "%~dp0neothesia-web"
  call npm.cmd run desktop:build:dev
  if errorlevel 1 (pause & exit /b 1)
)
start "" /D "%~dp0target\debug" "%~dp0target\debug\neothesia-desktop.exe" %*
