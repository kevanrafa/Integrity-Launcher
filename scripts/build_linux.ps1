param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$SkipAptInstall,
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

if (-not $SkipAptInstall) {
    Invoke-Step 'sudo apt-get update --yes'
    Invoke-Step 'sudo apt-get install --yes libssl-dev libdbus-1-dev libx11-xcb1 libxkbcommon-x11-dev pkg-config libseccomp-dev'
}

Invoke-Step 'cargo build --release --target x86_64-unknown-linux-gnu'
Invoke-Step 'strip target/x86_64-unknown-linux-gnu/release/integrity_launcher'

if (-not $DryRun) {
    New-Item -ItemType Directory -Path "dist" -Force | Out-Null
    Move-Item -Force `
        -Path "target/x86_64-unknown-linux-gnu/release/integrity_launcher" `
        -Destination "dist/IntegrityLauncher-Linux-x86_64"
}

Invoke-Step 'cargo install cargo-packager'

$packagerConfig = @{
    name        = "integrity-launcher"
    outDir      = "./dist"
    formats     = @("deb", "appimage")
    productName = "Integrity Launcher"
    version     = $versionNormalized
    identifier  = "com.integrity.launcher"
    resources   = @()
    authors     = @("Moulberry")
    binaries    = @(@{ path = "IntegrityLauncher-Linux-x86_64"; main = $true })
    icons       = @(
        "package/windows_icons/icon_16x16.png",
        "package/windows_icons/icon_32x32.png",
        "package/windows_icons/icon_48x48.png",
        "package/windows_icons/icon_256x256.png"
    )
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
        -Path "dist/IntegrityLauncher-Linux-x86_64" `
        -Destination "dist/IntegrityLauncher-Linux-x86_64-Portable"

    $appImageFrom = "dist/IntegrityLauncher-Linux-x86_64_${versionNormalized}_x86_64.AppImage"
    if (Test-Path $appImageFrom) {
        Move-Item -Force -Path $appImageFrom -Destination "dist/IntegrityLauncher-Linux-x86_64.AppImage"
    }
}

if ($env:CARGO_PACKAGER_SIGN_PRIVATE_KEY) {
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher-Linux-x86_64-Portable'
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher-Linux-x86_64.AppImage'

    if (-not $DryRun) {
        $portable = Get-Item "dist/IntegrityLauncher-Linux-x86_64-Portable"
        $appImage = Get-Item "dist/IntegrityLauncher-Linux-x86_64.AppImage"
        $portableSha1 = (Get-FileHash $portable.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $appImageSha1 = (Get-FileHash $appImage.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $portableSig = (Get-Content -Raw "dist/IntegrityLauncher-Linux-x86_64-Portable.sig").Trim()
        $appImageSig = (Get-Content -Raw "dist/IntegrityLauncher-Linux-x86_64.AppImage.sig").Trim()

        $manifest = @{
            version = $versionNormalized
            downloads = @{
                x86_64 = @{
                    executable = @{
                        download = "https://github.com/Moulberry/IntegrityLauncher/releases/download/v$versionNormalized/IntegrityLauncher-Linux-x86_64-Portable"
                        size     = $portable.Length
                        sha1     = $portableSha1
                        sig      = $portableSig
                    }
                    appimage = @{
                        download = "https://github.com/Moulberry/IntegrityLauncher/releases/download/v$versionNormalized/IntegrityLauncher-Linux-x86_64.AppImage"
                        size     = $appImage.Length
                        sha1     = $appImageSha1
                        sig      = $appImageSig
                    }
                }
            }
        } | ConvertTo-Json -Depth 16

        Set-Content -Path "dist/update_manifest_linux.json" -Value $manifest -NoNewline
        Remove-Item "dist/*.sig" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Linux build script selesai." -ForegroundColor Green
