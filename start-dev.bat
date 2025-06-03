@echo off
setlocal

echo Starting Spotify Server in development mode...
echo Console output will be visible. Press Ctrl+C to exit or use the system tray.
echo.

REM Build full path to the executable
set "EXE_PATH=%~dp0target\release\spotify-server.exe"

REM Run with -dev parameter to show console output
"%EXE_PATH%" -dev

endlocal
