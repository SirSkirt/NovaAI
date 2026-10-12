$ErrorActionPreference = "Stop"

Write-Host "Nova Android Setup" -ForegroundColor Cyan
Write-Host ""

if (-not $env:ANDROID_HOME) {
    $candidate = Join-Path $env:LOCALAPPDATA "Android\Sdk"
    if (Test-Path $candidate) {
        $env:ANDROID_HOME = $candidate
        $env:ANDROID_SDK_ROOT = $candidate
    }
}

if (-not $env:ANDROID_HOME) {
    throw "ANDROID_HOME is not set. Install Android Studio + SDK components first."
}

if (-not $env:NDK_HOME) {
    $ndkRoot = Join-Path $env:ANDROID_HOME "ndk"

    if (Test-Path $ndkRoot) {
        $latest = Get-ChildItem $ndkRoot -Directory |
            Sort-Object Name |
            Select-Object -Last 1

        if ($latest) {
            $env:NDK_HOME = $latest.FullName
        }
    }
}

if (-not $env:NDK_HOME) {
    throw "NDK_HOME is not set and no Android NDK installation was found."
}

Write-Host "ANDROID_HOME = $env:ANDROID_HOME" -ForegroundColor Gray
Write-Host "NDK_HOME     = $env:NDK_HOME" -ForegroundColor Gray
Write-Host ""

Write-Host "Installing Rust Android targets..." -ForegroundColor Gray
rustup target add `
    aarch64-linux-android `
    armv7-linux-androideabi `
    i686-linux-android `
    x86_64-linux-android

Write-Host "Installing npm dependencies..." -ForegroundColor Gray
npm install

if (-not (Test-Path ".\src-tauri\gen\android")) {
    Write-Host "Initializing Tauri Android project..." -ForegroundColor Gray
    npm run android:init
}

Write-Host ""
Write-Host "Android support is initialized." -ForegroundColor Green
Write-Host "Run: npm run android:dev" -ForegroundColor Cyan
Write-Host "Or:  npm run android:debug-apk" -ForegroundColor Cyan
