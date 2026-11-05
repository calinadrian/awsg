@echo off
rem Run the native game against the 1.12.1 client data in wow-data\Data.
rem Builds this checkout's player game first if needed; pass --debug for a debug build.
setlocal
set "ROOT=%~dp0"
set "WOW_DATA=%ROOT%wow-data\Data"
if not exist "%WOW_DATA%\terrain.MPQ" (
  echo No game data in %WOW_DATA% - copy a WoW 1.12.1 client's Data\*.MPQ there.
  pause
  exit /b 1
)
cd /d "%ROOT%"
if "%~1"=="--debug" (
  cargo run -p warcraft-survivors --no-default-features
) else (
  cargo run --release -p warcraft-survivors --no-default-features
)
if errorlevel 1 (
  echo.
  echo Game startup failed. See the error above and benilla-config\Diagnostics for crash reports.
  pause
  exit /b 1
)
