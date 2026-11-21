//! Associazione dei file .pubcraft a PubliCraft (solo Windows, per l'utente corrente):
//! doppio clic apre il documento e l'icona del file è quella dell'app.

#[cfg(windows)]
pub fn register() {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Ok(exe) = std::env::current_exe() else { return };
    let exe = exe.to_string_lossy().into_owned();
    let command = format!("\"{exe}\" \"%1\"");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Già registrato con questo percorso: niente da fare.
    if let Ok(k) = hkcu.open_subkey(r"Software\Classes\PubliCraft.Document\shell\open\command") {
        if k.get_value::<String, _>("").ok().as_deref() == Some(command.as_str()) {
            return;
        }
    }

    let run = || -> std::io::Result<()> {
        for ext in [".pubcraft", ".pcraft"] {
            let (k, _) = hkcu.create_subkey(format!(r"Software\Classes\{ext}"))?;
            k.set_value("", &"PubliCraft.Document")?;
        }
        let (k, _) = hkcu.create_subkey(r"Software\Classes\PubliCraft.Document")?;
        k.set_value("", &"Documento PubliCraft")?;
        let (k, _) = hkcu.create_subkey(r"Software\Classes\PubliCraft.Document\DefaultIcon")?;
        k.set_value("", &format!("\"{exe}\",0"))?;
        let (k, _) = hkcu.create_subkey(r"Software\Classes\PubliCraft.Document\shell\open\command")?;
        k.set_value("", &command)?;
        Ok(())
    };
    if run().is_ok() {
        // Dice a Esplora risorse che le associazioni (e le icone) sono cambiate.
        unsafe {
            windows_sys::Win32::UI::Shell::SHChangeNotify(0x0800_0000, 0, std::ptr::null(), std::ptr::null());
        }
    }
}

#[cfg(not(windows))]
pub fn register() {}
