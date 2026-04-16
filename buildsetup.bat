@echo off
setlocal enabledelayedexpansion

set "version=%~1"
if not "%version%"=="" set "version=%version:#v=%"

if "%version%"=="" (
    for /f "tokens=1,* delims==" %%A in ('findstr /R /C:"^version *= *\"" crates\pandora_launcher\Cargo.toml') do (
        set "version_raw=%%B"
    )
    if defined version_raw (
        set "version=!version_raw: =!"
        set "version=!version:"=!"
    )
)

if "%version%"=="" (
    echo Failed to detect version from crates/pandora_launcher/Cargo.toml
    echo Usage: buildsetup.bat [version]
    exit /b 1
)

echo Building setup installer for version: %version%
call "%~dp0build_windows.bat" %version%
if errorlevel 1 exit /b 1

echo.
echo Setup installer created:
echo   dist\IntegrityLauncher-Windows-x86_64-Setup.exe

endlocal
