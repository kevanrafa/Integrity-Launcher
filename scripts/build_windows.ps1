param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Command
    )

    Write-Host ">> $Command" -ForegroundColor Cyan
    if (-not $DryRun) {
        Invoke-Expression $Command
    }
}

$versionNormalized = $Version -replace "^v", ""
$env:PANDORA_RELEASE_VERSION = $versionNormalized

New-Item -ItemType Directory -Path "dist" -Force | Out-Null

Invoke-Step 'cargo build --release --target x86_64-pc-windows-msvc'

# Optional strip, only if available in PATH.
$strip = Get-Command strip -ErrorAction SilentlyContinue
if ($null -ne $strip) {
    Invoke-Step 'strip target/x86_64-pc-windows-msvc/release/integrity_launcher.exe'
}

if (-not $DryRun) {
    Move-Item -Force `
        -Path "target/x86_64-pc-windows-msvc/release/integrity_launcher.exe" `
        -Destination "dist/IntegrityLauncher-Windows-x86_64.exe"
}

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
} | ConvertTo-Json -Depth 8 -Compress

if ($DryRun) {
    Write-Host ">> cargo packager --config '$packagerConfig'" -ForegroundColor Cyan
} else {
    $savedSignKey = $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY
    Remove-Item Env:\CARGO_PACKAGER_SIGN_PRIVATE_KEY -ErrorAction SilentlyContinue
    cargo packager --config $packagerConfig
    if ($null -ne $savedSignKey) {
        $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY = $savedSignKey
    }
}

if (-not $DryRun) {
    Move-Item -Force `
        -Path "dist/IntegrityLauncher-Windows-x86_64.exe" `
        -Destination "dist/IntegrityLauncher-Windows-x86_64-Portable.exe"

    $setupFrom = "dist/IntegrityLauncher-Windows-x86_64_${versionNormalized}_x64-setup.exe"
    if (Test-Path $setupFrom) {
        Move-Item -Force -Path $setupFrom -Destination "dist/IntegrityLauncher-Windows-x86_64-Setup.exe"
    }
}

if ($env:CARGO_PACKAGER_SIGN_PRIVATE_KEY) {
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher-Windows-x86_64-Portable.exe'

    if (-not $DryRun) {
        $portable = Get-Item "dist/IntegrityLauncher-Windows-x86_64-Portable.exe"
        $portableSha1 = (Get-FileHash $portable.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $portableSig = Get-Content -Raw "dist/IntegrityLauncher-Windows-x86_64-Portable.exe.sig"

        $manifest = @{
            version = $versionNormalized
            downloads = @{
                x86_64 = @{
                    executable = @{
                        download = "https://github.com/kevanrafa/IntegrityLauncher/releases/download/v$versionNormalized/IntegrityLauncher-Windows-x86_64-Portable.exe"
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

Write-Host "Windows build script selesai." -ForegroundColor Green
