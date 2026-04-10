param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# Jalankan command sebagai proses native (bukan Invoke-Expression)
# supaya warnings di stderr tidak dianggap error oleh PowerShell
function Invoke-Step {
    param([Parameter(Mandatory = $true)][string]$Command)

    Write-Host ">> $Command" -ForegroundColor Cyan
    if (-not $DryRun) {
        $tokens = $Command -split '\s+'
        $exe    = $tokens[0]
        $rest   = $tokens[1..($tokens.Length - 1)]

        & $exe @rest

        if ($LASTEXITCODE -ne 0) {
            throw "Command failed (exit $LASTEXITCODE): $Command"
        }
    }
}

# Setup
$versionNormalized = $Version -replace "^v", ""
$env:PANDORA_RELEASE_VERSION = $versionNormalized

New-Item -ItemType Directory -Path "dist" -Force | Out-Null

# Build
Invoke-Step 'cargo build --release --target x86_64-pc-windows-msvc'

$strip = Get-Command strip -ErrorAction SilentlyContinue
if ($null -ne $strip) {
    Invoke-Step 'strip target/x86_64-pc-windows-msvc/release/integrity_launcher.exe'
}

if (-not $DryRun) {
    Move-Item -Force `
        -Path "target/x86_64-pc-windows-msvc/release/integrity_launcher.exe" `
        -Destination "dist/IntegrityLauncher-Windows-x86_64.exe"
}

# Package (NSIS Setup + Portable)
Invoke-Step 'cargo install cargo-packager'

$packagerConfig = @{
    name        = "integrity-launcher"
    outDir      = "./dist"
    productName = "Integrity Launcher"
    version     = $versionNormalized
    identifier  = "com.integrity.launcher"
    resources   = @()
    authors     = @("Moulberry (Former), Developed by Kevanrafa1")
    binaries    = @(@{ path = "IntegrityLauncher-Windows-x86_64.exe"; main = $true })
    icons       = @("package/windows.ico")
    formats     = @("nsis")
} | ConvertTo-Json -Depth 8 -Compress

if ($DryRun) {
    Write-Host ">> cargo packager --config <json>" -ForegroundColor Cyan
} else {
    $savedSignKey = $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY
    Remove-Item Env:\CARGO_PACKAGER_SIGN_PRIVATE_KEY -ErrorAction SilentlyContinue

    & cargo packager --config $packagerConfig
    if ($LASTEXITCODE -ne 0) { throw "cargo packager failed (exit $LASTEXITCODE)" }

    if ($null -ne $savedSignKey) {
        $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY = $savedSignKey
    }
}

# Rename output files
if (-not $DryRun) {
    Move-Item -Force `
        -Path "dist/IntegrityLauncher-Windows-x86_64.exe" `
        -Destination "dist/IntegrityLauncher-Windows-x86_64-Portable.exe"

    $setupCandidates = @(
        "dist/IntegrityLauncher-Windows-x86_64_${versionNormalized}_x64-setup.exe",
        "dist/integrity-launcher_${versionNormalized}_x64-setup.exe",
        "dist/Integrity Launcher_${versionNormalized}_x64-setup.exe"
    )

    $setupFound = $false
    foreach ($candidate in $setupCandidates) {
        if (Test-Path $candidate) {
            Move-Item -Force -Path $candidate -Destination "dist/IntegrityLauncher-Windows-x86_64-Setup.exe"
            Write-Host "Setup ditemukan: $candidate" -ForegroundColor Green
            $setupFound = $true
            break
        }
    }

    if (-not $setupFound) {
        $fallback = Get-ChildItem "dist" -Filter "*setup*.exe" | Select-Object -First 1
        if ($null -ne $fallback) {
            Move-Item -Force -Path $fallback.FullName -Destination "dist/IntegrityLauncher-Windows-x86_64-Setup.exe"
            Write-Host "Setup ditemukan (fallback): $($fallback.Name)" -ForegroundColor Yellow
        } else {
            Write-Warning "File setup tidak ditemukan di dist/. Cek output cargo-packager di atas."
        }
    }
}

# Signing (opsional)
if ($env:CARGO_PACKAGER_SIGN_PRIVATE_KEY) {
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher-Windows-x86_64-Portable.exe'

    if (-not $DryRun) {
        $portable     = Get-Item "dist/IntegrityLauncher-Windows-x86_64-Portable.exe"
        $portableSha1 = (Get-FileHash $portable.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $portableSig  = Get-Content -Raw "dist/IntegrityLauncher-Windows-x86_64-Portable.exe.sig"

        $manifest = @{
            version   = $versionNormalized
            downloads = @{
                x86_64 = @{
                    executable = @{
                        download = "https://github.com/kevanrafa/IntegrityLauncher/releases/download/v${versionNormalized}/IntegrityLauncher-Windows-x86_64-Portable.exe"
                        size     = $portable.Length
                        sha1     = $portableSha1
                        sig      = $portableSig.Trim()
                    }
                }
            }
        } | ConvertTo-Json -Depth 16

        Set-Content -Path "dist/update_manifest_windows.json" -Value $manifest -NoNewline
        Remove-Item "dist/*.sig" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host ""
Write-Host "Build selesai! Output di dist/" -ForegroundColor Green
Write-Host "  Portable : dist/IntegrityLauncher-Windows-x86_64-Portable.exe" -ForegroundColor Cyan
Write-Host "  Setup    : dist/IntegrityLauncher-Windows-x86_64-Setup.exe" -ForegroundColor Cyan