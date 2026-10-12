$ErrorActionPreference = "Stop"

Write-Host "Nova Tauri V4.3.11" -ForegroundColor Cyan
Write-Host "Installing npm dependencies..." -ForegroundColor Gray
npm install

Write-Host ""
Write-Host "Starting Tauri development window..." -ForegroundColor Green
npm run dev
