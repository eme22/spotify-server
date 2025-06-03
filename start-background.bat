@echo off
setlocal

echo Starting Spotify Server in background mode...
echo The server will run in the system tray. Right-click the tray icon to exit.
echo.

REM Build full path to the executable
set "EXE_PATH=%~dp0target\release\spotify-server.exe"

REM Use PowerShell to launch it hidden (like in the .ps1)
powershell -Command "Start-Process -FilePath '%EXE_PATH%' -WindowStyle Hidden"

echo Spotify Server started! Check your system tray for the icon.
timeout /t 3 /nobreak >nul

endlocal