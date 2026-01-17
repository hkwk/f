fn main() {
    #[cfg(target_os = "windows")]
    {
        // This uses the winres crate to embed an .ico into the produced exe.
        // Requires a Windows SDK (rc.exe) on MSVC toolchain, or windres on GNU toolchain.
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/app.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
