//! Caratteri: legge quelli di Windows (se ci sono) e li registra con il nome mostrato nella barra.
//! Se un file manca si ripiega sui caratteri incorporati di egui.

use egui::{FontData, FontDefinitions, FontFamily};
use publicraft_doc::Family;
use std::sync::{Arc, OnceLock};

static LOADED: OnceLock<Vec<Family>> = OnceLock::new();

fn file(f: Family) -> Option<&'static str> {
    Some(match f {
        Family::Sans => "segoeui.ttf",
        Family::Arial => "arial.ttf",
        Family::Times => "times.ttf",
        Family::Calibri => "calibri.ttf",
        Family::Georgia => "georgia.ttf",
        Family::Verdana => "verdana.ttf",
        Family::Tahoma => "tahoma.ttf",
        Family::Consolas => "consola.ttf",
        Family::Courier => "cour.ttf",
        Family::Mono => return None,
    })
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let defaults = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let dir = std::path::Path::new(r"C:\Windows\Fonts");

    let mut symbols = None;
    if let Ok(bytes) = std::fs::read(dir.join("seguisym.ttf")) {
        fonts.font_data.insert("segoe-symbols".into(), Arc::new(FontData::from_owned(bytes)));
        symbols = Some("segoe-symbols".to_string());
    }

    let mut loaded = Vec::new();
    for fam in Family::ALL {
        let Some(name) = file(fam) else { continue };
        let Ok(bytes) = std::fs::read(dir.join(name)) else { continue };
        let key = format!("pc-{}", fam.label());
        fonts.font_data.insert(key.clone(), Arc::new(FontData::from_owned(bytes)));
        let chain: Vec<String> = std::iter::once(key.clone()).chain(symbols.iter().cloned()).chain(defaults.iter().cloned()).collect();
        if fam == Family::Sans {
            // Anche l'interfaccia usa Segoe UI.
            fonts.families.insert(FontFamily::Proportional, chain.clone());
        }
        fonts.families.insert(FontFamily::Name(fam.label().into()), chain);
        loaded.push(fam);
    }
    if let Some(s) = &symbols {
        // Simboli di riserva anche per i caratteri incorporati.
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            let list = fonts.families.entry(fam).or_default();
            if !list.contains(s) {
                list.push(s.clone());
            }
        }
    }
    let _ = LOADED.set(loaded);
    ctx.set_fonts(fonts);
}

/// Famiglia egui da usare per il carattere scelto.
pub fn family(f: Family) -> FontFamily {
    if LOADED.get().is_some_and(|l| l.contains(&f)) {
        FontFamily::Name(f.label().into())
    } else if matches!(f, Family::Mono | Family::Consolas | Family::Courier) {
        FontFamily::Monospace
    } else {
        FontFamily::Proportional
    }
}
