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

// ---- Modalità chiara / scura (per le app Craft che la offrono) ----

use std::sync::atomic::{AtomicBool, Ordering};

static DARK: AtomicBool = AtomicBool::new(false);

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

fn mode_file(app: &str) -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("APPDATA")?).join(app).join("theme.txt"))
}

/// Legge la modalità salvata dall'app (`dark` o `light`) e la imposta. Se manca, resta quella attuale.
pub fn load_mode(app: &str) {
    if let Some(text) = mode_file(app).and_then(|p| std::fs::read_to_string(p).ok()) {
        set_dark(text.trim() == "dark");
    }
}

/// Imposta e salva la modalità per l'app.
pub fn save_mode(app: &str, dark: bool) {
    set_dark(dark);
    if let Some(p) = mode_file(app) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, if dark { "dark" } else { "light" });
    }
}

/// Colori della modalità attuale.
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub text: Color32,
    pub text_dim: Color32,
    /// Pannelli laterali e finestre.
    pub panel: Color32,
    pub panel_raised: Color32,
    pub border: Color32,
    /// Barra del titolo e nastro.
    pub bar: Color32,
    /// Area di lavoro dietro ai documenti.
    pub work: Color32,
    /// Campi di testo.
    pub field: Color32,
    /// Sfondo dei pulsanti attivi (accento attenuato).
    pub soft: Color32,
}

pub fn palette(accent: Color32) -> Palette {
    if is_dark() {
        Palette {
            dark: true,
            text: TEXT,
            text_dim: TEXT_DIM,
            panel: PANEL,
            panel_raised: PANEL_RAISED,
            border: BORDER,
            bar: Color32::from_rgb(0x25, 0x27, 0x2c),
            work: Color32::from_rgb(0x14, 0x15, 0x18),
            field: BG,
            soft: accent.gamma_multiply(0.35),
        }
    } else {
        Palette {
            dark: false,
            text: Color32::from_rgb(0x26, 0x26, 0x26),
            text_dim: Color32::from_rgb(0x6b, 0x6b, 0x6b),
            panel: Color32::from_rgb(0xf3, 0xf3, 0xf3),
            panel_raised: Color32::from_rgb(0xe4, 0xe4, 0xe4),
            border: Color32::from_rgb(0xd0, 0xd0, 0xd0),
            bar: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            work: Color32::from_rgb(0xdf, 0xdf, 0xdf),
            field: Color32::WHITE,
            soft: Color32::from_rgb(0xfb, 0xd9, 0xd3),
        }
    }
}

/// Applica il tema (chiaro o scuro, secondo `set_dark`) con l'accento indicato.
pub fn apply_mode(ctx: &egui::Context, accent: Color32) {
    let p = palette(accent);
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    v.override_text_color = Some(p.text);
    v.panel_fill = p.panel;
    v.window_fill = if p.dark { p.panel } else { Color32::WHITE };
    v.extreme_bg_color = p.field;
    v.faint_bg_color = p.panel_raised;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.hyperlink_color = accent;
    v.selection.bg_fill = p.soft;
    v.selection.stroke = Stroke::new(1.0, accent);

    let r = CornerRadius::same(3);
    let idle = if p.dark { p.panel_raised } else { Color32::from_rgb(0xf7, 0xf7, 0xf7) };
    let idle_stroke = if p.dark { p.border } else { Color32::from_rgb(0xcc, 0xcc, 0xcc) };
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.bg_fill = idle;
    v.widgets.inactive.weak_bg_fill = idle;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, idle_stroke);
    v.widgets.inactive.corner_radius = r;
    let hover = if p.dark { p.border } else { p.panel_raised };
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent);
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.bg_fill = p.soft;
    v.widgets.active.weak_bg_fill = p.soft;
    v.widgets.active.bg_stroke = Stroke::new(1.0, accent);
    v.widgets.active.corner_radius = r;
    v.widgets.open.bg_fill = p.soft;
    v.widgets.open.weak_bg_fill = p.soft;
    v.widgets.open.corner_radius = r;
    ctx.set_visuals(v);

    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(9.0, 4.0);
        // Transizioni più morbide per hover e apertura di menu.
        s.animation_time = 0.14;
    });
}
