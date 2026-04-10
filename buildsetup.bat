cargo install cargo-packager
$env:PANDORA_RELEASE_VERSION="0.1.0"
cargo build --release --target x86_64-pc-windows-msvc -p integrity_launcher
