#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}
export PANDORA_RELEASE_VERSION=$version

sudo apt-get update --yes && sudo apt-get install --yes libssl-dev libdbus-1-dev libx11-xcb1 libxkbcommon-x11-dev pkg-config libseccomp-dev
cargo build --release --target x86_64-unknown-linux-gnu
strip target/x86_64-unknown-linux-gnu/release/integrity_launcher
mkdir -p dist
mv target/x86_64-unknown-linux-gnu/release/integrity_launcher dist/IntegrityLauncher-Linux-x86_64

cargo install cargo-packager
env -u CARGO_PACKAGER_SIGN_PRIVATE_KEY cargo packager --config '{'\
'  "name": "integrity-launcher",'\
'  "outDir": "./dist",'\
'  "formats": ["deb", "appimage"],'\
'  "productName": "Integrity Launcher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.integrity.launcher",'\
'  "resources": [],'\
'  "authors": ["Moulberry"],'\
'  "binaries": [{ "path": "IntegrityLauncher-Linux-x86_64", "main": true }],'\
'  "icons": ["package/windows_icons/icon_16x16.png", "package/windows_icons/icon_32x32.png", "package/windows_icons/icon_48x48.png", "package/windows_icons/icon_256x256.png"]'\
'}'

mv -f dist/IntegrityLauncher-Linux-x86_64 dist/IntegrityLauncher-Linux-x86_64-Portable
mv -f 'dist/IntegrityLauncher-Linux-x86_64_'$version'_x86_64.AppImage' dist/IntegrityLauncher-Linux-x86_64.AppImage

if [[ -n "$CARGO_PACKAGER_SIGN_PRIVATE_KEY" ]]; then
    cargo packager signer sign dist/IntegrityLauncher-Linux-x86_64-Portable
    cargo packager signer sign dist/IntegrityLauncher-Linux-x86_64.AppImage

    echo "{
    \"version\": \"$version\",
    \"downloads\": {
        \"x86_64\": {
            \"executable\": {
                \"download\": \"https://github.com/Moulberry/IntegrityLauncher/releases/download/v$version/IntegrityLauncher-Linux-x86_64-Portable\",
                \"size\": $(wc -c < dist/IntegrityLauncher-Linux-x86_64-Portable),
                \"sha1\": \"$(sha1sum dist/IntegrityLauncher-Linux-x86_64-Portable | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/IntegrityLauncher-Linux-x86_64-Portable.sig)\"
            },
            \"appimage\": {
                \"download\": \"https://github.com/Moulberry/IntegrityLauncher/releases/download/v$version/IntegrityLauncher-Linux-x86_64.AppImage\",
                \"size\": $(wc -c < dist/IntegrityLauncher-Linux-x86_64.AppImage),
                \"sha1\": \"$(sha1sum dist/IntegrityLauncher-Linux-x86_64.AppImage | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/IntegrityLauncher-Linux-x86_64.AppImage.sig)\"
            }
        }
    }
}" > dist/update_manifest_linux.json

    rm dist/*.sig
fi
