#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../package/windows.ico");
    res.set("ProductName", "Integrity Launcher");
    res.set("FileDescription", "Integrity Launcher");
    res.set("CompanyName", "kevanrafa10 (Kriss)");
    res.set("LegalCopyright", "The Architect");
    res.set("InternalName", "IntegrityLauncher");
    res.set("OriginalFilename", "IntegrityLauncher.exe");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}
