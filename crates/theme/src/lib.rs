//! Tema condiviso: stesso look scuro e sobrio delle app ArtCraft,
//! con un colore di accento per ogni app.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x16, 0x17, 0x1a);
pub const PANEL: Color32 = Color32::from_rgb(0x1e, 0x20, 0x24);
pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x28, 0x2b, 0x30);
pub const BORDER: Color32 = Color32::from_rgb(0x38, 0x3b, 0x42);
pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe7, 0xea);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9a, 0x9d, 0xa6);

/// Accento di PubliCraft (rosso del logo).
pub const ACCENT_PUBLICRAFT: Color32 = Color32::from_rgb(0xe3, 0x45, 0x30);
/// Accento di FrancyLauncher.
pub const ACCENT_LAUNCHER: Color32 = Color32::from_rgb(0xdc, 0xb4, 0x2a);

/// Applica il tema scuro Craft con l'accento indicato.
pub fn apply(ctx: &egui::Context, accent: Color32) {
    let mut v = Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
    v.faint_bg_color = PANEL_RAISED;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);

    let r = CornerRadius::same(6);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.bg_fill = PANEL_RAISED;
    v.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.bg_fill = PANEL_RAISED;
    v.widgets.hovered.weak_bg_fill = BORDER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent);
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.bg_fill = accent.gamma_multiply(0.6);
    v.widgets.active.weak_bg_fill = accent.gamma_multiply(0.6);
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    ctx.set_visuals(v);

    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}
