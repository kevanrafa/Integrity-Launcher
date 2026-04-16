@echo off
setlocal enabledelayedexpansion

set "version="
if /I "%~1"=="--version" (
    set "version=%~2"
) else (
    set "version=%~1"
)

if /I "!version:~0,1!"=="v" set "version=!version:~1!"
if not "!version!"=="" set "version=!version:#v=!"

if "!version!"=="" (
    for /f "tokens=2 delims== " %%A in ('findstr /B /C:"version" crates\pandora_launcher\Cargo.toml') do (
        set "version=%%~A"
    )
)

if "!version!"=="" (
    echo Failed to detect version from crates/pandora_launcher/Cargo.toml
    echo Usage: buildsetup.bat [version]
    echo   or: buildsetup.bat --version [version]
    exit /b 1
)

echo Building setup installer for version: !version!
call "%~dp0build_windows.bat" !version!
if errorlevel 1 exit /b 1

echo.
echo Setup installer created:
echo   dist\IntegrityLauncher-Windows-x86_64-Setup.exe

endlocal
