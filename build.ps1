# Integrity Launcher Build Script
# Run: .\build.ps1

param(
    [switch]$Release,
    [switch]$Setup,
    [switch]$Portable,
    [switch]$Clean,
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot

Write-Host "=== Integrity Launcher Build Script ===" -ForegroundColor Cyan
Write-Host "Version: $Version" -ForegroundColor Green

# Clean build artifacts
if ($Clean) {
    Write-Host "[1/5] Cleaning build artifacts..." -ForegroundColor Yellow
    if (Test-Path "target") {
        Remove-Item -Path "target" -Recurse -Force
        Write-Host "  -> Cleaned target directory" -ForegroundColor Gray
    }
    if (Test-Path "installer") {
        Remove-Item -Path "installer" -Recurse -Force
        Write-Host "  -> Cleaned installer directory" -ForegroundColor Gray
    }
}

# Build Rust
Write-Host "[2/5] Building Rust application..." -ForegroundColor Yellow
if ($Release) {
    cargo build --release
    $ExePath = "target\release\integrity_launcher.exe"
    Write-Host "  -> Release build: $ExePath" -ForegroundColor Gray
} else {
    cargo build
    $ExePath = "target\debug\integrity_launcher.exe"
    Write-Host "  -> Debug build: $ExePath" -ForegroundColor Gray
}

if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Build failed - executable not found!" -ForegroundColor Red
    exit 1
}

# Get file size
$FileSize = (Get-Item $ExePath).Length / 1MB
Write-Host "  -> Size: $([math]::Round($FileSize, 2)) MB" -ForegroundColor Green

# Create output directory
$OutputDir = "output\$Version"
if (-not (Test-Path $OutputDir)) {
    New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null
}

# Copy executable
Write-Host "[3/5] Copying build output..." -ForegroundColor Yellow
Copy-Item $ExePath "$OutputDir\integrity_launcher.exe" -Force

# Create portable ZIP
if ($Portable) {
    Write-Host "[4/5] Creating portable package..." -ForegroundColor Yellow
    $ZipName = "IntegrityLauncher-Portable-$Version-win-x64.zip"
    $ZipPath = "installer\$ZipName"
    
    if (-not (Test-Path "installer")) {
        New-Item -Path "installer" -ItemType Directory -Force | Out-Null
    }
    
    # Create ZIP
    Compress-Archive -Path "$OutputDir\integrity_launcher.exe" -DestinationPath $ZipPath -Force
    Write-Host "  -> $ZipPath" -ForegroundColor Green
}

# Create Setup/Installer
if ($Setup) {
    Write-Host "[5/5] Creating installer..." -ForegroundColor Yellow
    
    # Check if Inno Setup is installed
    $InnoSetup = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    if (-not (Test-Path $InnoSetup)) {
        $InnoSetup = "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    }
    
    if (Test-Path $InnoSetup) {
        # Update version in setup.iss
        $setupContent = Get-Content "setup.iss" -Raw
        $setupContent = $setupContent -replace 'MyAppVersion "[\d\.]+"', "MyAppVersion `"$Version`""
        Set-Content -Path "setup.iss" -Value $setupContent -Force
        
        & $InnoSetup "setup.iss"
        Write-Host "  -> Installer created successfully" -ForegroundColor Green
    } else {
        Write-Host "  -> Inno Setup not found. Install from https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
        Write-Host "  -> Skipping installer creation" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Cyan
Write-Host "Output: $OutputDir\integrity_launcher.exe" -ForegroundColor Green
Write-Host "Size: $([math]::Round($FileSize, 2)) MB" -ForegroundColor Green

# List output files
Write-Host ""
Write-Host "Output files:" -ForegroundColor Yellow
Get-ChildItem -Path "output" -Recurse -File | ForEach-Object {
    $size = $_.Length / 1MB
    Write-Host "  $($_.FullName) ($([math]::Round($size, 2)) MB)" -ForegroundColor Gray
}

Get-ChildItem -Path "installer" -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
    $size = $_.Length / 1MB
    Write-Host "  $($_.FullName) ($([math]::Round($size, 2)) MB)" -ForegroundColor Gray
}