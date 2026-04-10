#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}
export PANDORA_RELEASE_VERSION=$version

cargo build --release --target x86_64-pc-windows-msvc
strip target/x86_64-pc-windows-msvc/release/integrity_launcher.exe

mkdir -p dist

mv target/x86_64-pc-windows-msvc/release/integrity_launcher dist/IntegrityLauncher-Windows-x86_64.exe

cargo install cargo-packager
env -u CARGO_PACKAGER_SIGN_PRIVATE_KEY cargo packager --config '{'\
'  "name": "integrity-launcher",'\
'  "outDir": "./dist",'\
'  "productName": "Integrity Launcher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.integrity.launcher",'\
'  "resources": [],'\
'  "authors": ["Moulberry"],'\
'  "binaries": [{ "path": "IntegrityLauncher-Windows-x86_64.exe", "main": true }],'\
'  "icons": ["package/windows.ico"]'\
'}'

mv -f dist/IntegrityLauncher-Windows-x86_64.exe dist/IntegrityLauncher-Windows-x86_64-Portable.exe
mv -f 'dist/IntegrityLauncher-Windows-x86_64_'$version'_x64-setup.exe' dist/IntegrityLauncher-Windows-x86_64-Setup.exe

if [[ -n "$CARGO_PACKAGER_SIGN_PRIVATE_KEY" ]]; then
    cargo packager signer sign dist/IntegrityLauncher-Windows-x86_64-Portable.exe

    echo "{
    \"version\": \"$version\",
    \"downloads\": {
        \"x86_64\": {
            \"executable\": {
                \"download\": \"https://github.com/Moulberry/IntegrityLauncher/releases/download/v$version/IntegrityLauncher-Windows-x86_64-Portable.exe\",
                \"size\": $(wc -c < dist/IntegrityLauncher-Windows-x86_64-Portable.exe),
                \"sha1\": \"$(sha1sum dist/IntegrityLauncher-Windows-x86_64-Portable.exe | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/IntegrityLauncher-Windows-x86_64-Portable.exe.sig)\"
            }
        }
    }
}" > dist/update_manifest_windows.json

    rm dist/*.sig
fi
