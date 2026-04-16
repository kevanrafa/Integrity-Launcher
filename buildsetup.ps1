param(
    [Parameter(Position = 0)]
    [string]$Version,

    [Alias("version")]
    [string]$VersionNamed
)

$ErrorActionPreference = "Stop"

Set-Location -Path $PSScriptRoot

if (-not [string]::IsNullOrWhiteSpace($VersionNamed)) {
    $Version = $VersionNamed
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $cargoTomlPath = Join-Path $PSScriptRoot "crates\pandora_launcher\Cargo.toml"
    if (-not (Test-Path $cargoTomlPath)) {
        throw "Cannot find version source: $cargoTomlPath"
    }

    $versionLine = Select-String -Path $cargoTomlPath -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) {
        throw "Failed to detect version from $cargoTomlPath"
    }

    $Version = $versionLine.Matches[0].Groups[1].Value
}

$Version = $Version.Trim()
if ($Version.StartsWith("v", [System.StringComparison]::OrdinalIgnoreCase)) {
    $Version = $Version.Substring(1)
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    throw "Version is empty after normalization."
}

Write-Host "Building setup installer for version: $Version"

$env:PANDORA_RELEASE_VERSION = $Version

$distDir = Join-Path $PSScriptRoot "dist"
$portableName = "IntegrityLauncher-Windows-x86_64-Portable.exe"
$setupName = "IntegrityLauncher-Windows-x86_64-Setup.exe"
$portablePath = Join-Path $distDir $portableName
$setupPath = Join-Path $distDir $setupName

& cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed." }

New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$builtExe = Join-Path $PSScriptRoot "target\x86_64-pc-windows-msvc\release\integrity_launcher.exe"
if (-not (Test-Path $builtExe)) {
    throw "Built executable not found: $builtExe"
}

$packagerInputExe = Join-Path $distDir "IntegrityLauncher-Windows-x86_64.exe"
Move-Item -LiteralPath $builtExe -Destination $packagerInputExe -Force

& cargo install cargo-packager
if ($LASTEXITCODE -ne 0) { throw "cargo install cargo-packager failed." }

$originalSigningKey = $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY
$hadSigningKey = -not [string]::IsNullOrEmpty($originalSigningKey)

$packagerConfig = @{
    name = "integrity-launcher"
    outDir = "./dist"
    productName = "Integrity Launcher"
    version = $Version
    identifier = "com.integrity.launcher"
    resources = @()
    authors = @("Moulberry (Former)", "Kevanrafa10")
    binaries = @(
        @{
            path = "IntegrityLauncher-Windows-x86_64.exe"
            main = $true
        }
    )
    icons = @("package/windows.ico")
} | ConvertTo-Json -Depth 6 -Compress

try {
    if ($hadSigningKey) {
        Remove-Item Env:CARGO_PACKAGER_SIGN_PRIVATE_KEY -ErrorAction SilentlyContinue
    }

    & cargo packager --config $packagerConfig
    if ($LASTEXITCODE -ne 0) { throw "cargo packager failed." }
}
finally {
    if ($hadSigningKey) {
        $env:CARGO_PACKAGER_SIGN_PRIVATE_KEY = $originalSigningKey
    }
}

Move-Item -LiteralPath $packagerInputExe -Destination $portablePath -Force

$setupFromPackager = Join-Path $distDir ("IntegrityLauncher-Windows-x86_64_{0}_x64-setup.exe" -f $Version)
if (-not (Test-Path $setupFromPackager)) {
    $candidate = Get-ChildItem -Path $distDir -Filter "*_x64-setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($null -eq $candidate) {
        throw "Setup output not found in dist directory."
    }
    $setupFromPackager = $candidate.FullName
}

Move-Item -LiteralPath $setupFromPackager -Destination $setupPath -Force

if ($hadSigningKey) {
    Write-Host "Signing executable..."
    & cargo packager signer sign $portablePath
    if ($LASTEXITCODE -ne 0) { throw "cargo packager signer sign failed." }

    $portableFile = Get-Item -LiteralPath $portablePath
    $sha1 = (Get-FileHash -Algorithm SHA1 -Path $portablePath).Hash.ToLowerInvariant()
    $sigPath = "$portablePath.sig"
    $sig = ""
    if (Test-Path $sigPath) {
        $sig = (Get-Content -Path $sigPath -Raw).Trim()
    }

    $manifestPath = Join-Path $distDir "update_manifest_windows.json"
    $manifest = @{
        version = $Version
        downloads = @{
            x86_64 = @{
                executable = @{
                    download = "https://github.com/Moulberry/IntegrityLauncher/releases/download/v$Version/$portableName"
                    size = [int64]$portableFile.Length
                    sha1 = $sha1
                    sig = $sig
                }
            }
        }
    } | ConvertTo-Json -Depth 8

    Set-Content -Path $manifestPath -Value $manifest -Encoding UTF8
    Remove-Item -Path (Join-Path $distDir "*.sig") -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Build complete!"
Write-Host "Output files are in the 'dist' folder:"
Write-Host "  - dist\$portableName"
Write-Host "  - dist\$setupName"
if ($hadSigningKey) {
    Write-Host "  - dist\update_manifest_windows.json"
}
