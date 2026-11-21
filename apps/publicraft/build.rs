fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "PubliCraft");
        res.set("FileDescription", "PubliCraft");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icona dell'eseguibile non incorporata: {e}");
        }
    }
}
