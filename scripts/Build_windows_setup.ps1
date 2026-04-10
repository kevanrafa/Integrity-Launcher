param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [switch]$DryRun,
    [switch]$Fast
)

$ErrorActionPreference = "Stop"

function Step($cmd) {
    Write-Host ">> $cmd" -ForegroundColor Cyan
    if (-not $DryRun) {
        Invoke-Expression $cmd
    }
}

$versionNormalized = $Version -replace "^v", ""
$env:PANDORA_RELEASE_VERSION = $versionNormalized

# Clean dist
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "dist" | Out-Null

# ---------------------------
# 🔧 BUILD
# ---------------------------
if ($Fast) {
    Step "cargo build"
    $exePath = "target/debug/integrity_launcher.exe"
} else {
    Step "cargo build --release --target x86_64-pc-windows-msvc"
    $exePath = "target/x86_64-pc-windows-msvc/release/integrity_launcher.exe"
}

# ---------------------------
# 📦 PORTABLE
# ---------------------------
$portableOut = "dist/IntegrityLauncher-Portable.exe"

if (-not $DryRun) {
    Copy-Item $exePath $portableOut -Force
}

# Optional strip
$strip = Get-Command strip -ErrorAction SilentlyContinue
if ($null -ne $strip -and -not $Fast) {
    Step "strip $portableOut"
}

# ---------------------------
# 🧱 SETUP (cargo-packager)
# ---------------------------
# Jangan install tiap run
if (-not (Get-Command cargo-packager -ErrorAction SilentlyContinue)) {
    Step "cargo install cargo-packager"
}

$packagerConfig = @{
    name        = "integrity-launcher"
    productName = "Integrity Launcher"
    version     = $versionNormalized
    identifier  = "com.integrity.launcher"
    outDir      = "./dist"

    binaries = @(@{
        path = "IntegrityLauncher-Portable.exe"
        main = $true
    })

    windows = @{
        installMode = "perMachine"
    }

    icons = @("package/windows.ico")
} | ConvertTo-Json -Depth 10 -Compress

if ($DryRun) {
    Write-Host ">> cargo packager --config '$packagerConfig'" -ForegroundColor Cyan
} else {
    cargo packager --config $packagerConfig
}

# ---------------------------
# 🔍 FIND SETUP FILE (AUTO)
# ---------------------------
$setupFile = Get-ChildItem "dist" -Filter "*setup.exe" | Select-Object -First 1

if ($null -ne $setupFile -and -not $DryRun) {
    Move-Item -Force $setupFile.FullName "dist/IntegrityLauncher-Setup.exe"
}

# ---------------------------
# 🧾 MANIFEST
# ---------------------------
if (-not $DryRun) {

    $portable = Get-Item $portableOut
    $setup    = Get-Item "dist/IntegrityLauncher-Setup.exe"

    $manifest = @{
        version = $versionNormalized
        downloads = @{
            portable = @{
                url  = "https://your.repo/releases/download/v$versionNormalized/IntegrityLauncher-Portable.exe"
                size = $portable.Length
                sha1 = (Get-FileHash $portable.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
            }
            setup = @{
                url  = "https://your.repo/releases/download/v$versionNormalized/IntegrityLauncher-Setup.exe"
                size = $setup.Length
                sha1 = (Get-FileHash $setup.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
            }
        }
    } | ConvertTo-Json -Depth 16

    Set-Content "dist/update_manifest.json" $manifest
}

Write-Host "Build selesai (Portable + Setup + Manifest)" -ForegroundColor Green