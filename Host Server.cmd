@echo off
setlocal
cd /d "%~dp0"
cargo build --release -p asr-coop
if errorlevel 1 (
  pause
  exit /b 1
)
"%~dp0target\release\asr-coop.exe" --bind 0.0.0.0:47771 --economy-store "%~dp0benilla-config\economy\asr-economy"
pause
