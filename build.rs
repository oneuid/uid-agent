fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        let version = env!("CARGO_PKG_VERSION");
        res.set("FileDescription", "UID.one Endpoint Security Agent");
        res.set("ProductName", "UID.one Agent");
        res.set("OriginalFilename", "uid-agent.exe");
        res.set("CompanyName", "UID.one Technologies");
        res.set("LegalCopyright", "Copyright © 2026 UID.one. All rights reserved.");
        res.set("ProductVersion", version);
        res.set("FileVersion", version);
        res.compile().unwrap();
    }
}
