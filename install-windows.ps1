# install.ps1

$ErrorActionPreference = "Stop"

Write-Host "Building Make It So..."
cargo build --release

$binPath = ".\target\release\mis.exe"
$installPath = "$env:USERPROFILE\.cargo\bin\mis.exe"

if (-Not (Test-Path $binPath)) {
    Write-Host "❌ Build failed: binary not found at $binPath"
    exit 1
}

Write-Host "🚀 Installing to $installPath"
Copy-Item -Path $binPath -Destination $installPath -Force

Write-Host "✅ Make It So installed successfully!"

$misPath = Get-Command mis -ErrorAction SilentlyContinue

if ($misPath) {
    & mis --version
} else {
    Write-Host "⚠️ Make It So not found in PATH"
}
