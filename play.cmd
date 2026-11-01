@echo off
rem Run the native game against the 1.12.1 client data in wow-data\Data.
rem Builds first if needed; pass --debug to skip the release profile.
setlocal
set "ROOT=%~dp0"
set "WOW_DATA=%ROOT%wow-data\Data"
if not exist "%WOW_DATA%\terrain.MPQ" (
  echo No game data in %WOW_DATA% - copy a WoW 1.12.1 client's Data\*.MPQ there.
  exit /b 1
)
cd /d "%ROOT%"
if "%~1"=="--debug" (
  cargo run -p warcraft-survivors
) else (
  cargo run --release -p warcraft-survivors
)
