# Spotify Server - Development Mode
Write-Host "Starting Spotify Server in development mode..." -ForegroundColor Green
Write-Host "Console output will be visible. Press Ctrl+C to exit or use the system tray." -ForegroundColor Yellow
Write-Host ""

$exePath = Join-Path $PSScriptRoot "target\release\SpotifyServer.exe"

# Run with development flag
& $exePath --dev
