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

Invoke-Step 'cargo build --release --target aarch64-apple-darwin'
Invoke-Step 'cargo build --release --target x86_64-apple-darwin'
Invoke-Step 'strip target/aarch64-apple-darwin/release/integrity_launcher'
Invoke-Step 'strip target/x86_64-apple-darwin/release/integrity_launcher'

if (-not $DryRun) {
    New-Item -ItemType Directory -Path "dist" -Force | Out-Null
}

Invoke-Step 'lipo -create -output dist/IntegrityLauncher-macOS-Universal target/x86_64-apple-darwin/release/integrity_launcher target/aarch64-apple-darwin/release/integrity_launcher'

Invoke-Step 'cargo install cargo-packager'

$packagerConfig = @{
    name        = "integrity-launcher"
    outDir      = "./dist"
    formats     = @("dmg", "app")
    productName = "IntegrityLauncher"
    version     = $versionNormalized
    identifier  = "com.integrity.launcher"
    resources   = @()
    authors     = @("Moulberry")
    binaries    = @(@{ path = "IntegrityLauncher-macOS-Universal"; main = $true })
    icons       = @("package/mac.icns")
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
        -Path "dist/IntegrityLauncher-macOS-Universal" `
        -Destination "dist/IntegrityLauncher-macOS-Universal-Portable"

    $dmgFile = Get-ChildItem "dist/IntegrityLauncher*.dmg" | Select-Object -First 1
    if ($null -ne $dmgFile) {
        Move-Item -Force -Path $dmgFile.FullName -Destination "dist/IntegrityLauncher.dmg"
    }

    tar -czf dist/IntegrityLauncher.app.tar.gz dist/IntegrityLauncher.app
    Remove-Item -Recurse -Force dist/IntegrityLauncher.app
}

if ($env:CARGO_PACKAGER_SIGN_PRIVATE_KEY) {
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher-macOS-Universal-Portable'
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher.dmg'
    Invoke-Step 'cargo packager signer sign dist/IntegrityLauncher.app.tar.gz'

    if (-not $DryRun) {
        $portable = Get-Item "dist/IntegrityLauncher-macOS-Universal-Portable"
        $appTar = Get-Item "dist/IntegrityLauncher.app.tar.gz"
        $portableSha1 = (Get-FileHash $portable.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $appTarSha1 = (Get-FileHash $appTar.FullName -Algorithm SHA1).Hash.ToLowerInvariant()
        $portableSig = (Get-Content -Raw "dist/IntegrityLauncher-macOS-Universal-Portable.sig").Trim()
        $appTarSig = (Get-Content -Raw "dist/IntegrityLauncher.app.tar.gz.sig").Trim()

        $manifest = @{
            version = $versionNormalized
            downloads = @{
                universal = @{
                    executable = @{
                        download = "https://github.com/Moulberry/IntegrityLauncher/releases/download/v$versionNormalized/IntegrityLauncher-macOS-Universal-Portable"
                        size     = $portable.Length
                        sha1     = $portableSha1
                        sig      = $portableSig
                    }
                    app = @{
                        download = "https://github.com/Moulberry/IntegrityLauncher/releases/download/v$versionNormalized/IntegrityLauncher.app.tar.gz"
                        size     = $appTar.Length
                        sha1     = $appTarSha1
                        sig      = $appTarSig
                    }
                }
            }
        } | ConvertTo-Json -Depth 16

        Set-Content -Path "dist/update_manifest_macos.json" -Value $manifest -NoNewline
        Remove-Item "dist/*.sig" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "macOS build script selesai." -ForegroundColor Green
