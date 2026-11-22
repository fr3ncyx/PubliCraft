//! Colori di PubliCraft: seguono la modalità chiara o scura scelta in Opzioni.

use egui::Color32;

const ACCENT: Color32 = craft_theme::ACCENT_PUBLICRAFT;

fn p() -> craft_theme::Palette {
    craft_theme::palette(ACCENT)
}

pub fn text() -> Color32 {
    p().text
}
pub fn text_dim() -> Color32 {
    p().text_dim
}
pub fn panel() -> Color32 {
    p().panel
}
pub fn panel_raised() -> Color32 {
    p().panel_raised
}
pub fn border() -> Color32 {
    p().border
}
/// Sfondo del nastro (titolo, schede e comandi).
pub fn ribbon() -> Color32 {
    p().bar
}
/// Area di lavoro dietro alle pagine.
pub fn work() -> Color32 {
    p().work
}
/// Accento attenuato per i pulsanti attivi.
pub fn accent_soft() -> Color32 {
    p().soft
}

pub fn apply(ctx: &egui::Context) {
    craft_theme::apply_mode(ctx, ACCENT);
}
