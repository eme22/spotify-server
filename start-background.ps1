Write-Host "Starting Spotify Server in background mode..." -ForegroundColor Green
Write-Host "The server will run in the system tray. Right-click the tray icon to exit." -ForegroundColor Yellow
Write-Host ""

$exePath = Join-Path $PSScriptRoot "target\release\spotify-server.exe"
Start-Process -FilePath $exePath -WindowStyle Hidden

Write-Host "Spotify Server started! Check your system tray for the icon." -ForegroundColor Cyan
Start-Sleep 3
