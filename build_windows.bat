@echo off
setlocal enabledelayedexpansion

if "%1"=="" (
    echo Missing version argument
    exit /b 1
)

set version=%1
set version=%version:#v=%
set PANDORA_RELEASE_VERSION=%version%

cargo build --release --target x86_64-pc-windows-msvc
if errorlevel 1 exit /b 1

strip target\x86_64-pc-windows-msvc\release\integrity_launcher.exe
if errorlevel 1 exit /b 1

mkdir dist 2>nul

move target\x86_64-pc-windows-msvc\release\integrity_launcher.exe dist\IntegrityLauncher-Windows-x86_64.exe
if errorlevel 1 exit /b 1

cargo install cargo-packager
if errorlevel 1 exit /b 1

:: Temporarily unset CARGO_PACKAGER_SIGN_PRIVATE_KEY if it exists
set SAVED_KEY=
if defined CARGO_PACKAGER_SIGN_PRIVATE_KEY (
    set SAVED_KEY=!CARGO_PACKAGER_SIGN_PRIVATE_KEY!
    set CARGO_PACKAGER_SIGN_PRIVATE_KEY=
)

cargo packager --config { "name": "integrity-launcher", "outDir": "./dist", "productName": "Integrity Launcher", "version": "%version%", "identifier": "com.integrity.launcher", "resources": [], "authors": ["Moulberry"], "binaries": [{ "path": "IntegrityLauncher-Windows-x86_64.exe", "main": true }], "icons": ["package/windows.ico"] }

:: Restore the environment variable if it was set
if defined SAVED_KEY (
    set CARGO_PACKAGER_SIGN_PRIVATE_KEY=!SAVED_KEY!
    set SAVED_KEY=
)
if errorlevel 1 exit /b 1

move /Y dist\IntegrityLauncher-Windows-x86_64.exe dist\IntegrityLauncher-Windows-x86_64-Portable.exe
if errorlevel 1 exit /b 1

move /Y "dist\IntegrityLauncher-Windows-x86_64_%version%_x64-setup.exe" dist\IntegrityLauncher-Windows-x86_64-Setup.exe
if errorlevel 1 exit /b 1

if not "%CARGO_PACKAGER_SIGN_PRIVATE_KEY%"=="" (
    cargo packager signer sign dist\IntegrityLauncher-Windows-x86_64-Portable.exe
    if errorlevel 1 exit /b 1

    :: Get file size in bytes
    for %%A in (dist\IntegrityLauncher-Windows-x86_64-Portable.exe) do set size=%%~zA

    :: Get SHA1 hash (using certutil or PowerShell)
    set sha1=
    for /f "tokens=1" %%A in ('certutil -hashfile dist\IntegrityLauncher-Windows-x86_64-Portable.exe SHA1 ^| findstr /v "hash" ^| findstr /v "CertUtil"') do (
        if not defined sha1 set sha1=%%A
    )
    if "!sha1!"=="" (
        for /f "tokens=1" %%A in ('powershell -command "Get-FileHash -Algorithm SHA1 dist\IntegrityLauncher-Windows-x86_64-Portable.exe | ForEach-Object { $_.Hash.ToLower() }"') do set sha1=%%A
    )

    :: Get signature content (base64 encode the .sig file)
    set sig=
    if exist dist\IntegrityLauncher-Windows-x86_64-Portable.exe.sig (
        for /f "usebackq delims=" %%A in ("dist\IntegrityLauncher-Windows-x86_64-Portable.exe.sig") do set sig=%%A
    )

    (
        echo {
        echo     "version": "%version%",
        echo     "downloads": {
        echo         "x86_64": {
        echo             "executable": {
        echo                 "download": "https://github.com/Moulberry/IntegrityLauncher/releases/download/v%version%/IntegrityLauncher-Windows-x86_64-Portable.exe",
        echo                 "size": !size!,
        echo                 "sha1": "!sha1!",
        echo                 "sig": "!sig!"
        echo             }
        echo         }
        echo     }
        echo }
    ) > dist\update_manifest_windows.json

    del dist\*.sig 2>nul
)

endlocal

pause