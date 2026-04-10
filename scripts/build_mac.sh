#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}
export PANDORA_RELEASE_VERSION=$version

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

strip target/aarch64-apple-darwin/release/integrity_launcher
strip target/x86_64-apple-darwin/release/integrity_launcher

mkdir -p dist

lipo -create -output dist/IntegrityLauncher-macOS-Universal target/x86_64-apple-darwin/release/integrity_launcher target/aarch64-apple-darwin/release/integrity_launcher

cargo install cargo-packager
env -u CARGO_PACKAGER_SIGN_PRIVATE_KEY cargo packager --config '{'\
'  "name": "integrity-launcher",'\
'  "outDir": "./dist",'\
'  "formats": ["dmg", "app"],'\
'  "productName": "IntegrityLauncher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.integrity.launcher",'\
'  "resources": [],'\
'  "authors": ["Moulberry"],'\
'  "binaries": [{ "path": "IntegrityLauncher-macOS-Universal", "main": true }],'\
'  "icons": ["package/mac.icns"]'\
'}'

mv -f dist/IntegrityLauncher-macOS-Universal dist/IntegrityLauncher-macOS-Universal-Portable
mv -f dist/IntegrityLauncher*.dmg dist/IntegrityLauncher.dmg
tar -czf dist/IntegrityLauncher.app.tar.gz dist/IntegrityLauncher.app
rm -r dist/IntegrityLauncher.app

if [[ -n "$CARGO_PACKAGER_SIGN_PRIVATE_KEY" ]]; then
    cargo packager signer sign dist/IntegrityLauncher-macOS-Universal-Portable
    cargo packager signer sign dist/IntegrityLauncher.dmg
    cargo packager signer sign dist/IntegrityLauncher.app.tar.gz

    echo "{
    \"version\": \"$version\",
    \"downloads\": {
        \"universal\": {
            \"executable\": {
                \"download\": \"https://github.com/Moulberry/IntegrityLauncher/releases/download/v$version/IntegrityLauncher-macOS-Universal-Portable\",
                \"size\": $(wc -c < dist/IntegrityLauncher-macOS-Universal-Portable),
                \"sha1\": \"$(sha1sum dist/IntegrityLauncher-macOS-Universal-Portable | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/IntegrityLauncher-macOS-Universal-Portable.sig)\"
            },
            \"app\": {
                \"download\": \"https://github.com/Moulberry/IntegrityLauncher/releases/download/v$version/IntegrityLauncher.app.tar.gz\",
                \"size\": $(wc -c < dist/IntegrityLauncher.app.tar.gz),
                \"sha1\": \"$(sha1sum dist/IntegrityLauncher.app.tar.gz | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/IntegrityLauncher.app.tar.gz.sig)\"
            }
        }
    }
}" > dist/update_manifest_macos.json

    rm dist/*.sig
fi
