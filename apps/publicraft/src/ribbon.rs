//! Barra multifunzione in stile Publisher: schede File, Home e Visualizza.

use super::*;
use egui::{Align, CornerRadius, FontId, Layout, Painter, Response, Sense, Stroke, Ui};
use egui::Rect as ERect;
use publicraft_doc::{AlignKind, CaseKind, Family, ListKind, Wrap, change_case};

const ACCENT: Color32 = craft_theme::ACCENT_PUBLICRAFT;
const COMMANDS: [&str; 15] = [
    "Nuovo", "Apri", "Salva", "Salva con nome", "Annulla", "Ripeti", "Trova e sostituisci", "Casella di testo", "Rettangolo", "Ellisse", "Immagine", "Tabella", "Pagina master", "Pagina doppia", "Importa PDF",
];
const BODY_H: f32 = 70.0;
const TILE_W: f32 = 58.0;
const SIZES: [f32; 18] = [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 60.0, 72.0, 96.0];
const PALETTE: [[u8; 3]; 20] = [
    [0x00, 0x00, 0x00], [0x40, 0x40, 0x40], [0x80, 0x80, 0x80], [0xc0, 0xc0, 0xc0], [0xff, 0xff, 0xff],
    [0xc0, 0x00, 0x00], [0xff, 0x00, 0x00], [0xff, 0x80, 0x00], [0xff, 0xc0, 0x00], [0xff, 0xff, 0x00],
    [0x00, 0x80, 0x00], [0x00, 0xb0, 0x50], [0xe3, 0x45, 0x30], [0x00, 0xb0, 0xf0], [0x00, 0x70, 0xc0],
    [0x00, 0x20, 0x60], [0x70, 0x30, 0xa0], [0xc0, 0x00, 0x80], [0x80, 0x40, 0x00], [0x9c, 0x65, 0x00],
];

fn size_label(s: f32) -> String {
    if s.fract() == 0.0 { format!("{}", s as i32) } else { format!("{s:.1}") }
}

fn step_size(cur: f32, up: bool) -> f32 {
    if up {
        SIZES.iter().copied().find(|s| *s > cur + 0.01).unwrap_or(cur)
    } else {
        SIZES.iter().rev().copied().find(|s| *s < cur - 0.01).unwrap_or(cur)
    }
}

fn dim() -> Color32 {
    Color32::from_gray(0xb4)
}

/// Pulsante grande con icona sopra e nome sotto.
fn tile(ui: &mut Ui, glyph: &str, label: &str, enabled: bool, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(TILE_W, BODY_H), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::ZERO, look::accent_soft());
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), egui::StrokeKind::Inside);
    } else if enabled {
        let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
        if t > 0.0 {
            p.rect_filled(rect, CornerRadius::ZERO, look::panel_raised().gamma_multiply(t));
        }
    }
    let col = if enabled { look::text() } else { dim() };
    p.text(egui::pos2(rect.center().x, rect.top() + 21.0), egui::Align2::CENTER_CENTER, glyph, FontId::proportional(25.0), if enabled { ACCENT.gamma_multiply(1.25) } else { dim() });
    let g = p.layout(label.to_string(), FontId::proportional(12.0), col, TILE_W - 4.0);
    p.galley(egui::pos2(rect.center().x - g.size().x / 2.0, rect.top() + 39.0), g, col);
    resp
}

/// Pulsante piccolo con testo.
fn sbtn(ui: &mut Ui, text: impl Into<egui::WidgetText>, tip: &str, enabled: bool, active: bool, w: f32) -> Response {
    let r = ui.add_enabled(enabled, egui::Button::new(text).selected(active).min_size(vec2(w, 24.0)));
    if tip.is_empty() { r } else { r.on_hover_text(tip).on_disabled_hover_text(tip) }
}

/// Pulsante piccolo con icona disegnata.
fn icon_btn(ui: &mut Ui, tip: &str, enabled: bool, active: bool, draw: impl FnOnce(&Painter, ERect, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 24.0), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::ZERO, look::accent_soft());
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), egui::StrokeKind::Inside);
    } else if enabled {
        let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
        if t > 0.0 {
            p.rect_filled(rect, CornerRadius::ZERO, look::panel_raised().gamma_multiply(t));
            p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, look::border().gamma_multiply(t)), egui::StrokeKind::Inside);
        }
    }
    draw(p, rect, if enabled { look::text() } else { dim() });
    if tip.is_empty() { resp } else { resp.on_hover_text(tip).on_disabled_hover_text(tip) }
}

fn align_lines(p: &Painter, r: ERect, c: Color32, kind: DAlign) {
    let widths = [14.0, 9.0, 14.0, 7.0];
    for (i, w) in widths.iter().enumerate() {
        let w = if kind == DAlign::Justify { 14.0 } else { *w };
        let y = r.top() + 6.0 + i as f32 * 3.8;
        let x0 = match kind {
            DAlign::Left | DAlign::Justify => r.center().x - 7.0,
            DAlign::Center => r.center().x - w / 2.0,
            DAlign::Right => r.center().x + 7.0 - w,
        };
        p.hline(x0..=x0 + w, y, Stroke::new(1.5, c));
    }
}

fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    let r = ui
        .vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 3.0);
            ui.scope(|ui| {
                ui.set_min_height(BODY_H + 12.0);
                add(ui);
            });
            let w = ui.min_rect().width();
            ui.allocate_ui_with_layout(vec2(w.max(10.0), 14.0), Layout::top_down(Align::Center), |ui| {
                ui.label(egui::RichText::new(title).size(11.5).color(look::text_dim()));
            });
        })
        .response;
    ui.painter().vline(r.rect.right() + 5.0, r.rect.y_range(), Stroke::new(1.0, look::border()));
    ui.add_space(10.0);
}

fn menu(resp: &Response, width: f32, add: impl FnOnce(&mut Ui)) {
    egui::Popup::menu(resp).width(width).show(add);
}

fn color_button(ui: &mut Ui, enabled: bool, color: Rgb) -> Response {
    icon_btn(ui, "Colore carattere", enabled, false, |p, r, c| {
        p.text(egui::pos2(r.center().x, r.center().y - 2.0), egui::Align2::CENTER_CENTER, "A", FontId::proportional(15.0), c);
        p.rect_filled(ERect::from_min_size(egui::pos2(r.center().x - 8.0, r.bottom() - 6.0), vec2(16.0, 3.0)), CornerRadius::ZERO, rgb(color));
    })
}

impl PubliCraft {
    pub(crate) fn ribbon(&mut self, ui: &mut Ui) {
        self.title_bar(ui);
        let fade = self.tab_bar(ui);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, look::border()));
        ui.add_space(1.0);
        egui::ScrollArea::horizontal().id_salt("ribbon-scroll").auto_shrink([false, true]).show(ui, |ui| {
            ui.scope(|ui| {
                // Dissolvenza quando si cambia scheda.
                ui.set_opacity(fade.clamp(0.0, 1.0));
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    match self.tab {
                        RibbonTab::Home => self.home(ui),
                        RibbonTab::Insert => self.insert_tab(ui),
                        RibbonTab::View => self.view_tab(ui),
                    }
                });
            });
        });
        ui.add_space(3.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, look::border()));
    }

    /// Riga del titolo: accesso rapido a sinistra, nome del documento al centro, ricerca comandi a destra.
    fn title_bar(&mut self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
        let title = format!("{}{}", self.d.title(), if self.d.dirty() { " — Modificato" } else { "" });
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, title, FontId::proportional(13.5), look::text());

        let left = ERect::from_min_size(rect.min + vec2(12.0, 0.0), vec2(190.0, 36.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(left).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            let glyph = |g: &'static str| move |p: &Painter, r: ERect, c: Color32| {
                p.text(r.center(), egui::Align2::CENTER_CENTER, g, FontId::proportional(15.0), c);
            };
            if icon_btn(ui, "Salva (Ctrl+S)", true, false, glyph("💾")).clicked() {
                self.save(false);
            }
            if icon_btn(ui, "Annulla (Ctrl+Z)", !self.d.undo.is_empty(), false, glyph("↶")).clicked() {
                self.undo();
            }
            if icon_btn(ui, "Ripeti (Ctrl+Y)", !self.d.redo.is_empty(), false, glyph("↷")).clicked() {
                self.redo();
            }
            let dark = craft_theme::is_dark();
            if icon_btn(ui, if dark { "Modalità chiara" } else { "Modalità scura" }, true, false, glyph(if dark { "☀" } else { "🌙" })).clicked() {
                self.set_dark(&ui.ctx().clone(), !dark);
            }
        });

        let right = ERect::from_min_max(egui::pos2(rect.max.x - 340.0, rect.min.y + 4.0), egui::pos2(rect.max.x - 14.0, rect.max.y - 4.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(right).layout(Layout::left_to_right(Align::Center)), |ui| self.search_box(ui));
    }

    /// Casella «Cerca comandi e guida» con elenco dei risultati.
    fn search_box(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let resp = ui.add(egui::TextEdit::singleline(&mut self.cmd).hint_text("🔍  Cerca comandi e guida").desired_width(ui.available_width()).margin(egui::Margin::symmetric(10, 5)));
        if resp.has_focus() {
            self.cmd_open = true;
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cmd.clear();
                self.cmd_open = false;
            }
        }
        let q = self.cmd.trim().to_lowercase();
        if !self.cmd_open || q.is_empty() {
            return;
        }
        let hits: Vec<usize> = COMMANDS.iter().enumerate().filter(|(_, c)| c.to_lowercase().contains(&q)).map(|(i, _)| i).collect();
        let mut run = None;
        if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            run = hits.first().copied();
        }
        let area = egui::Area::new(ui.scope_id().with("cmd-pop")).order(egui::Order::Foreground).fixed_pos(resp.rect.left_bottom() + vec2(0.0, 4.0)).show(&ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(resp.rect.width() - 20.0);
                if hits.is_empty() {
                    ui.label(egui::RichText::new("Nessun comando trovato").color(look::text_dim()));
                }
                for i in &hits {
                    if ui.add(egui::Button::new(COMMANDS[*i]).frame(false).min_size(vec2(ui.available_width(), 24.0))).clicked() {
                        run = Some(*i);
                    }
                }
            });
        });
        let outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p) && !resp.rect.contains(p)));
        if outside {
            self.cmd_open = false;
        }
        if let Some(i) = run {
            self.cmd.clear();
            self.cmd_open = false;
            self.run_command(&ctx, i);
        }
    }

    fn run_command(&mut self, ctx: &egui::Context, i: usize) {
        match i {
            0 => self.new_doc(),
            1 => self.open(),
            2 => {
                self.save(false);
            }
            3 => {
                if self.save(true) {
                    self.request_exit(ctx);
                }
            }
            4 => self.undo(),
            5 => self.redo(),
            6 => {
                self.find.open = true;
                self.find.focus = true;
            }
            7 => self.tool = Tool::Text,
            8 => self.tool = Tool::Shape(ShapeTool::Rectangle),
            9 => self.tool = Tool::Shape(ShapeTool::Ellipse),
            10 => self.insert_picture(),
            11 => {
                let (r, c) = self.table_dims;
                self.insert_table(r, c);
            }
            12 => self.view.master = !self.view.master,
            13 => self.view.spread = !self.view.spread,
            _ => self.import_pdf_dialog(),
        }
    }

    /// Schede File / Home / Inserisci / Visualizza; restituisce l'opacità (0..1) della scheda attiva.
    fn tab_bar(&mut self, ui: &mut Ui) -> f32 {
        let ctx = ui.ctx().clone();
        let mut fade = 1.0;
        let mut active_rect: Option<ERect> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            ui.add_space(10.0);
            let (r, resp) = ui.allocate_exact_size(vec2(62.0, 30.0), Sense::click());
            let h = ctx.animate_bool_with_time(resp.id, resp.hovered(), 0.12);
            let fill = ACCENT.lerp_to_gamma(Color32::from_rgb(0xc2, 0x36, 0x24), h);
            ui.painter().rect_filled(r, CornerRadius::ZERO, fill);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "File", FontId::proportional(14.0), Color32::WHITE);
            if resp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                // «File»: schermata con barra laterale (Home, Nuovo, Apri, Salva…).
                self.start.section = crate::start::Section::Home;
                self.start.open = true;
            }
            ui.add_space(6.0);
            for (i, (t, l)) in [(RibbonTab::Home, "Home"), (RibbonTab::Insert, "Inserisci"), (RibbonTab::View, "Visualizza")].into_iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(82.0, 30.0), Sense::click());
                let active = self.tab == t;
                let a = ctx.animate_bool_with_time(ui.scope_id().with(("tab-on", i)), active, 0.18);
                let h = ctx.animate_bool_with_time(resp.id, resp.hovered() && !active, 0.12);
                if h > 0.0 {
                    ui.painter().rect_filled(r, CornerRadius::ZERO, look::panel_raised().gamma_multiply(h));
                }
                let col = look::text().lerp_to_gamma(ACCENT, a);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, l, FontId::proportional(14.0), col);
                if resp.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if active {
                    active_rect = Some(r);
                    fade = a;
                }
                if resp.clicked() {
                    self.tab = t;
                }
            }
            // Sottolineatura che scivola sotto la scheda attiva.
            if let Some(r) = active_rect {
                let l = ctx.animate_value_with_time(ui.scope_id().with("tab-l"), r.left() + 6.0, 0.18);
                let rr = ctx.animate_value_with_time(ui.scope_id().with("tab-r"), r.right() - 6.0, 0.18);
                ui.painter().hline(l..=rr, r.bottom() - 1.0, Stroke::new(3.0, ACCENT));
            }
        });
        fade
    }

    fn home(&mut self, ui: &mut Ui) {
        let has_sel = !self.d.sel.is_empty();
        let text = self.selected_text();
        let has_text = text.is_some();
        let (size, color, style) = text.unwrap_or((14.0, Rgb(0x20, 0x20, 0x20), TextStyle::default()));
        let pg = self.pg();
        let any_grouped = self.d.sel.iter().any(|id| self.d.doc.frame(pg, *id).is_some_and(|f| f.group != 0));
        let first_wrap = self.d.sel.first().and_then(|id| self.d.doc.frame(pg, *id)).map(|f| f.wrap);

        group(ui, "Appunti", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "📋", "Incolla", !self.clipboard.is_empty(), false).on_hover_text("Incolla (Ctrl+V)").clicked() {
                    self.paste();
                }
                ui.vertical(|ui| {
                    if sbtn(ui, "✂  Taglia", "Taglia (Ctrl+X)", has_sel, false, 92.0).clicked() {
                        self.cut();
                    }
                    if sbtn(ui, "⧉  Copia", "Copia (Ctrl+C)", has_sel, false, 92.0).clicked() {
                        self.copy();
                    }
                    let active = self.fmt_painter.is_some();
                    if sbtn(ui, "🖌  Copia formato", "Copia il formato del testo e applicalo a un'altra casella", has_text || active, active, 92.0).clicked() {
                        self.fmt_painter = if active { None } else { Some((style, size, color)) };
                    }
                });
            });
        });

        group(ui, "Carattere", |ui| {
            ui.add_enabled_ui(has_text, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("rb-font").width(140.0).selected_text(style.family.label()).show_ui(ui, |ui| {
                        for f in Family::ALL {
                            let t = egui::RichText::new(f.label()).family(fonts::family(f));
                            if ui.selectable_label(style.family == f, t).clicked() {
                                self.edit_text(move |_, _, _, s| s.family = f);
                            }
                        }
                    });
                    egui::ComboBox::from_id_salt("rb-size").width(46.0).selected_text(size_label(size)).show_ui(ui, |ui| {
                        for s in SIZES {
                            if ui.selectable_label((size - s).abs() < 0.01, size_label(s)).clicked() {
                                self.edit_text(move |_, sz, _, _| *sz = s);
                            }
                        }
                    });
                    if sbtn(ui, "A⁺", "Ingrandisci carattere", has_text, false, 26.0).clicked() {
                        self.edit_text(|_, sz, _, _| *sz = step_size(*sz, true));
                    }
                    if sbtn(ui, "A⁻", "Riduci carattere", has_text, false, 26.0).clicked() {
                        self.edit_text(|_, sz, _, _| *sz = step_size(*sz, false));
                    }
                    if sbtn(ui, "✕", "Cancella formattazione", has_text, false, 26.0).clicked() {
                        self.edit_text(|_, sz, c, s| {
                            *s = TextStyle::default();
                            *sz = 14.0;
                            *c = Rgb(0x20, 0x20, 0x20);
                        });
                    }
                });
                ui.horizontal(|ui| {
                    if sbtn(ui, egui::RichText::new("G").strong(), "Grassetto", has_text, style.bold, 26.0).clicked() {
                        let v = !style.bold;
                        self.edit_text(move |_, _, _, s| s.bold = v);
                    }
                    if sbtn(ui, egui::RichText::new("C").italics(), "Corsivo", has_text, style.italic, 26.0).clicked() {
                        let v = !style.italic;
                        self.edit_text(move |_, _, _, s| s.italic = v);
                    }
                    if sbtn(ui, egui::RichText::new("S").underline(), "Sottolineato", has_text, style.underline, 26.0).clicked() {
                        let v = !style.underline;
                        self.edit_text(move |_, _, _, s| s.underline = v);
                    }
                    if sbtn(ui, "X₂", "Pedice", has_text, style.sub, 26.0).clicked() {
                        let v = !style.sub;
                        self.edit_text(move |_, _, _, s| {
                            s.sub = v;
                            if v {
                                s.sup = false;
                            }
                        });
                    }
                    if sbtn(ui, "X²", "Apice", has_text, style.sup, 26.0).clicked() {
                        let v = !style.sup;
                        self.edit_text(move |_, _, _, s| {
                            s.sup = v;
                            if v {
                                s.sub = false;
                            }
                        });
                    }
                    let case = sbtn(ui, "Aa ▾", "Maiuscole/minuscole", has_text, false, 40.0);
                    menu(&case, 230.0, |ui| {
                        for (label, kind) in [
                            ("Maiuscola a inizio frase", CaseKind::Sentence),
                            ("minuscolo", CaseKind::Lower),
                            ("MAIUSCOLO", CaseKind::Upper),
                            ("Iniziali Maiuscole", CaseKind::Title),
                            ("iNVERTI MAIUSCOLE", CaseKind::Toggle),
                        ] {
                            if ui.add(egui::Button::new(label).frame(false).min_size(vec2(210.0, 22.0))).clicked() {
                                self.edit_text(move |t, _, _, _| *t = change_case(t, kind));
                            }
                        }
                    });
                    let spacing = sbtn(ui, "AV ▾", "Spaziatura tra i caratteri", has_text, false, 40.0);
                    menu(&spacing, 170.0, |ui| {
                        for (label, v) in [("Molto stretta", -1.5f32), ("Stretta", -0.75), ("Normale", 0.0), ("Larga", 1.0), ("Molto larga", 2.5)] {
                            if ui.selectable_label((style.tracking_pt - v).abs() < 0.01, label).clicked() {
                                self.edit_text(move |_, _, _, s| s.tracking_pt = v);
                            }
                        }
                    });
                    let cb = color_button(ui, has_text, color);
                    egui::Popup::menu(&cb).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                        ui.label("Colore del testo");
                        egui::Grid::new("palette").spacing([3.0, 3.0]).show(ui, |ui| {
                            for (i, c) in PALETTE.iter().enumerate() {
                                let r = ui.add(egui::Button::new("").fill(Color32::from_rgb(c[0], c[1], c[2])).min_size(vec2(20.0, 20.0)));
                                if r.clicked() {
                                    let n = Rgb(c[0], c[1], c[2]);
                                    self.edit_text(move |_, _, col, _| *col = n);
                                }
                                if i % 5 == 4 {
                                    ui.end_row();
                                }
                            }
                        });
                        let mut a = [color.0, color.1, color.2];
                        ui.horizontal(|ui| {
                            ui.label("Altro colore");
                            if ui.color_edit_button_srgb(&mut a).changed() {
                                let n = Rgb(a[0], a[1], a[2]);
                                self.edit_text(move |_, _, col, _| *col = n);
                            }
                        });
                    });
                });
            });
        });

        group(ui, "Paragrafo", |ui| {
            ui.add_enabled_ui(has_text, |ui| {
                ui.horizontal(|ui| {
                    if sbtn(ui, "•≡", "Elenco puntato", has_text, style.list == ListKind::Bullet, 30.0).clicked() {
                        let v = if style.list == ListKind::Bullet { ListKind::None } else { ListKind::Bullet };
                        self.edit_text(move |_, _, _, s| s.list = v);
                    }
                    if sbtn(ui, "1≡", "Elenco numerato", has_text, style.list == ListKind::Number, 30.0).clicked() {
                        let v = if style.list == ListKind::Number { ListKind::None } else { ListKind::Number };
                        self.edit_text(move |_, _, _, s| s.list = v);
                    }
                    if sbtn(ui, "⇤", "Riduci rientro", has_text && style.indent > 0, false, 26.0).clicked() {
                        self.edit_text(|_, _, _, s| s.indent = s.indent.saturating_sub(1));
                    }
                    if sbtn(ui, "⇥", "Aumenta rientro", has_text, false, 26.0).clicked() {
                        self.edit_text(|_, _, _, s| s.indent = (s.indent + 1).min(12));
                    }
                    if sbtn(ui, "¶", "Mostra/nascondi segni di paragrafo", true, self.view.marks, 26.0).clicked() {
                        self.view.marks = !self.view.marks;
                    }
                });
                ui.horizontal(|ui| {
                    for (a, tip) in [(DAlign::Left, "Allinea a sinistra"), (DAlign::Center, "Centra"), (DAlign::Right, "Allinea a destra"), (DAlign::Justify, "Giustifica")] {
                        if icon_btn(ui, tip, has_text, style.align == a, |p, r, c| align_lines(p, r, c, a)).clicked() {
                            self.edit_text(move |_, _, _, s| s.align = a);
                        }
                    }
                    let ls = sbtn(ui, "↕ ▾", "Interlinea", has_text, false, 40.0);
                    menu(&ls, 120.0, |ui| {
                        for v in [1.0f32, 1.15, 1.5, 2.0, 2.5, 3.0] {
                            if ui.selectable_label((style.line_spacing - v).abs() < 0.01, format!("{v:.2}")).clicked() {
                                self.edit_text(move |_, _, _, s| s.line_spacing = v);
                            }
                        }
                    });
                });
            });
        });

        group(ui, "Stili", |ui| {
            let r = tile(ui, "Aa", "Stili", has_text, false);
            menu(&r, 190.0, |ui| {
                // (nome, corpo, grassetto, corsivo)
                for (name, pt, b, i) in [
                    ("Normale", 14.0, false, false),
                    ("Titolo", 36.0, true, false),
                    ("Titolo 2", 26.0, true, false),
                    ("Sottotitolo", 18.0, false, true),
                    ("Citazione", 16.0, false, true),
                    ("Didascalia", 10.0, false, true),
                ] {
                    let t = egui::RichText::new(name).size((pt * 0.75f32).clamp(12.0, 24.0));
                    let t = if b { t.strong() } else { t };
                    let t = if i { t.italics() } else { t };
                    if ui.add(egui::Button::new(t).frame(false).min_size(vec2(170.0, 24.0))).clicked() {
                        self.edit_text(move |_, sz, _, s| {
                            *sz = pt;
                            s.bold = b;
                            s.italic = i;
                        });
                    }
                }
            });
        });

        group(ui, "Oggetti", |ui| {
          ui.horizontal(|ui| {
            let t = tile(ui, "▤", "Casella di testo", true, self.tool == Tool::Text);
            if t.clicked() {
                self.tool = if self.tool == Tool::Text { Tool::Select } else { Tool::Text };
            }
            if tile(ui, "🖼", "Immagini", true, false).clicked() {
                self.insert_picture();
            }
            let tb = tile(ui, "▦", "Tabella", true, false);
            egui::Popup::menu(&tb).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).width(170.0).show(|ui| {
                ui.label("Inserisci tabella");
                ui.horizontal(|ui| {
                    ui.label("Righe");
                    ui.add(egui::DragValue::new(&mut self.table_dims.0).range(1..=30));
                });
                ui.horizontal(|ui| {
                    ui.label("Colonne");
                    ui.add(egui::DragValue::new(&mut self.table_dims.1).range(1..=12));
                });
                if ui.button("Inserisci").clicked() {
                    let (r, c) = self.table_dims;
                    self.insert_table(r, c);
                    ui.close();
                }
            });
            let sh = tile(ui, "◇", "Forme", true, matches!(self.tool, Tool::Shape(_)));
            menu(&sh, 150.0, |ui| {
                for (label, s) in [("▭  Rettangolo", ShapeTool::Rectangle), ("◯  Ellisse", ShapeTool::Ellipse), ("△  Triangolo", ShapeTool::Triangle), ("╱  Linea", ShapeTool::Line)] {
                    if ui.add(egui::Button::new(label).frame(false).min_size(vec2(130.0, 24.0))).clicked() {
                        self.tool = Tool::Shape(s);
                    }
                }
            });
          });
        });

        group(ui, "Disponi", |ui| {
            ui.horizontal(|ui| {
                let wr = tile(ui, "⇆", "Testo attorno", has_sel, false);
                menu(&wr, 160.0, |ui| {
                    for w in Wrap::ALL {
                        if ui.selectable_label(first_wrap == Some(w), w.label()).clicked() {
                            self.mutate(move |doc, pg, sel| {
                                for id in sel {
                                    if let Some(f) = doc.frame_mut(pg, *id) {
                                        f.wrap = w;
                                    }
                                }
                            });
                        }
                    }
                });
                ui.vertical(|ui| {
                    let fw = sbtn(ui, "▲  Porta avanti ▾", "", has_sel, false, 128.0);
                    menu(&fw, 190.0, |ui| {
                        if ui.button("Porta avanti").clicked() {
                            self.mutate(|d, pg, sel| d.bring_forward(pg, sel));
                        }
                        if ui.button("Porta in primo piano").clicked() {
                            self.mutate(|d, pg, sel| d.bring_to_front(pg, sel));
                        }
                    });
                    let bw = sbtn(ui, "▼  Porta indietro ▾", "", has_sel, false, 128.0);
                    menu(&bw, 190.0, |ui| {
                        if ui.button("Porta indietro").clicked() {
                            self.mutate(|d, pg, sel| d.send_backward(pg, sel));
                        }
                        if ui.button("Porta in secondo piano").clicked() {
                            self.mutate(|d, pg, sel| d.send_to_back(pg, sel));
                        }
                    });
                    let al = sbtn(ui, "⊞  Allinea ▾", "", has_sel, false, 128.0);
                    menu(&al, 230.0, |ui| {
                        for (label, k) in [
                            ("Allinea a sinistra", AlignKind::Left),
                            ("Centra orizzontalmente", AlignKind::CenterH),
                            ("Allinea a destra", AlignKind::Right),
                            ("Allinea in alto", AlignKind::Top),
                            ("Centra verticalmente", AlignKind::CenterV),
                            ("Allinea in basso", AlignKind::Bottom),
                        ] {
                            if ui.button(label).clicked() {
                                self.mutate(move |d, pg, sel| d.align(pg, sel, k));
                            }
                        }
                        ui.separator();
                        let many = self.d.sel.len() >= 3;
                        if ui.add_enabled(many, egui::Button::new("Distribuisci orizzontalmente")).clicked() {
                            self.mutate(|d, pg, sel| d.distribute(pg, sel, true));
                        }
                        if ui.add_enabled(many, egui::Button::new("Distribuisci verticalmente")).clicked() {
                            self.mutate(|d, pg, sel| d.distribute(pg, sel, false));
                        }
                    });
                });
                ui.vertical(|ui| {
                    if sbtn(ui, "▣  Raggruppa", "Raggruppa gli elementi selezionati", self.d.sel.len() >= 2, false, 100.0).clicked() {
                        self.mutate(|d, pg, sel| {
                            d.group(pg, sel);
                        });
                    }
                    if sbtn(ui, "▢  Separa", "Separa il gruppo", any_grouped, false, 100.0).clicked() {
                        self.mutate(|d, pg, sel| {
                            d.ungroup(pg, sel);
                        });
                    }
                    let ro = sbtn(ui, "⟳  Ruota ▾", "", has_sel, false, 100.0);
                    menu(&ro, 190.0, |ui| {
                        for (label, q) in [("Ruota a destra di 90°", 1), ("Ruota a sinistra di 90°", -1), ("Ruota di 180°", 2)] {
                            if ui.button(label).clicked() {
                                self.mutate(move |d, pg, sel| d.rotate(pg, sel, q));
                            }
                        }
                    });
                });
            });
        });

        group(ui, "Modifica", |ui| {
            if sbtn(ui, "🔍  Trova", "Trova (Ctrl+F)", true, false, 100.0).clicked() {
                self.find.open = true;
                self.find.focus = true;
            }
            if sbtn(ui, "ab  Sostituisci", "Sostituisci (Ctrl+H)", true, false, 100.0).clicked() {
                self.find.open = true;
                self.find.focus = true;
            }
            let se = sbtn(ui, "↖  Seleziona ▾", "", true, false, 100.0);
            menu(&se, 210.0, |ui| {
                if ui.button("Seleziona tutto (Ctrl+A)").clicked() {
                    self.select_all();
                }
                if ui.button("Seleziona tutte le caselle di testo").clicked() {
                    self.select_texts();
                }
                if ui.button("Deseleziona").clicked() {
                    self.d.sel.clear();
                }
                if ui.button("Strumento di selezione").clicked() {
                    self.tool = Tool::Select;
                }
            });
        });
    }

    fn view_tab(&mut self, ui: &mut Ui) {
        let has_sel = !self.d.sel.is_empty();

        group(ui, "Visualizzazioni", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "▭", "Normale", true, !self.view.master).clicked() && self.view.master {
                    self.view.master = false;
                    self.d.sel.clear();
                    self.fit = Some(Fit::Page);
                }
                if tile(ui, "▥", "Pagina master", true, self.view.master).on_hover_text("Elementi comuni a tutte le pagine").clicked() && !self.view.master {
                    self.view.master = true;
                    self.d.sel.clear();
                    self.fit = Some(Fit::Page);
                }
            });
        });

        group(ui, "Layout", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "▯", "Pagina singola", true, !self.view.spread).clicked() {
                    self.view.spread = false;
                    self.fit = Some(Fit::Page);
                }
                if tile(ui, "▯▯", "Doppia pagina", true, self.view.spread).clicked() {
                    self.view.spread = true;
                    self.fit = Some(Fit::Width);
                }
            });
        });

        group(ui, "Mostra", |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.view.boundaries, "Delimitazioni");
                    ui.checkbox(&mut self.view.guides, "Guide");
                    ui.checkbox(&mut self.view.fields, "Campi");
                });
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.view.rulers, "Righelli");
                    ui.checkbox(&mut self.view.nav, "Navigazione pagine");
                    ui.checkbox(&mut self.view.scratch, "Area di lavoro");
                });
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.view.graphics, "Gestione immagini");
                    ui.checkbox(&mut self.view.baselines, "Linee di base");
                });
            });
        });

        group(ui, "Zoom", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "🔍", "100%", true, false).clicked() {
                    self.set_zoom(1.0);
                }
                if tile(ui, "⛶", "Pagina intera", true, false).clicked() {
                    self.fit = Some(Fit::Page);
                }
                ui.vertical(|ui| {
                    let cur = format!("{:.0}%", self.d.zoom * 100.0);
                    egui::ComboBox::from_id_salt("rb-zoom").width(84.0).selected_text(cur).show_ui(ui, |ui| {
                        for z in [500, 300, 200, 150, 100, 75, 50, 25] {
                            if ui.selectable_label(false, format!("{z}%")).clicked() {
                                self.set_zoom(z as f32 / 100.0);
                            }
                        }
                    });
                    if sbtn(ui, "↔  Larghezza pagina", "", true, false, 124.0).clicked() {
                        self.fit = Some(Fit::Width);
                    }
                    if sbtn(ui, "⊡  Oggetti selezionati", "", has_sel, false, 124.0).clicked() {
                        self.fit = Some(Fit::Selection);
                    }
                });
            });
        });

        group(ui, "Finestra", |ui| {
            ui.horizontal(|ui| {
                let tip = "Serve con più finestre affiancate: per ora i documenti si alternano con Cambia finestra";
                tile(ui, "▦", "Disponi tutto", false, false).on_disabled_hover_text(tip);
                tile(ui, "❐", "Cascata", false, false).on_disabled_hover_text(tip);
                let sw = tile(ui, "⧉", "Cambia finestra", true, false);
                menu(&sw, 230.0, |ui| {
                    ui.label(egui::RichText::new(format!("✓  {}", self.d.title())).strong());
                    let mut go = None;
                    for (i, o) in self.others.iter().enumerate() {
                        if ui.add(egui::Button::new(format!("     {}", o.title())).frame(false).min_size(vec2(210.0, 22.0))).clicked() {
                            go = Some(i);
                        }
                    }
                    if let Some(i) = go {
                        self.switch_to(i);
                    }
                    ui.separator();
                    if ui.button("Nuovo documento").clicked() {
                        self.new_doc();
                    }
                });
            });
        });
    }

    fn insert_tab(&mut self, ui: &mut Ui) {
        let n = self.d.doc.pages.len();
        let cur = self.d.page;

        group(ui, "Pagine", |ui| {
            let r = tile(ui, "🗋", "Pagina", true, false);
            menu(&r, 210.0, |ui| {
                if ui.button("Inserisci pagina dopo").clicked() {
                    self.insert_page_at(cur + 1);
                }
                if ui.button("Inserisci pagina prima").clicked() {
                    self.insert_page_at(cur);
                }
                if ui.button("Duplica pagina").clicked() {
                    self.duplicate_page();
                }
                if ui.add_enabled(n > 1, egui::Button::new("Elimina pagina")).clicked() {
                    self.delete_page();
                }
            });
        });

        group(ui, "Tabelle", |ui| {
            let tb = tile(ui, "▦", "Tabella", true, false);
            egui::Popup::menu(&tb).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).width(170.0).show(|ui| {
                ui.label("Inserisci tabella");
                ui.horizontal(|ui| {
                    ui.label("Righe");
                    ui.add(egui::DragValue::new(&mut self.table_dims.0).range(1..=30));
                });
                ui.horizontal(|ui| {
                    ui.label("Colonne");
                    ui.add(egui::DragValue::new(&mut self.table_dims.1).range(1..=12));
                });
                if ui.button("Inserisci").clicked() {
                    let (r, c) = self.table_dims;
                    self.insert_table(r, c);
                    ui.close();
                }
            });
        });

        group(ui, "Illustrazioni", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "🖼", "Immagini", true, false).clicked() {
                    self.insert_picture();
                }
                let sh = tile(ui, "◇", "Forme", true, matches!(self.tool, Tool::Shape(_)));
                menu(&sh, 150.0, |ui| {
                    for (label, s) in [("▭  Rettangolo", ShapeTool::Rectangle), ("◯  Ellisse", ShapeTool::Ellipse), ("△  Triangolo", ShapeTool::Triangle), ("╱  Linea", ShapeTool::Line)] {
                        if ui.add(egui::Button::new(label).frame(false).min_size(vec2(130.0, 24.0))).clicked() {
                            self.tool = Tool::Shape(s);
                        }
                    }
                });
            });
        });

        group(ui, "Testo", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "▤", "Casella di testo", true, self.tool == Tool::Text).clicked() {
                    self.tool = if self.tool == Tool::Text { Tool::Select } else { Tool::Text };
                }
                if tile(ui, "#", "Numero di pagina", true, false).on_hover_text("Inserisce il numero della pagina, che si aggiorna da solo").clicked() {
                    self.insert_into_text(canvas::PAGE_TOKEN);
                }
                if tile(ui, "📅", "Data", true, false).clicked() {
                    self.insert_into_text(&Self::today());
                }
                let sy = tile(ui, "Ω", "Simbolo", true, false);
                menu(&sy, 240.0, |ui| {
                    egui::Grid::new("symbols").spacing([2.0, 2.0]).show(ui, |ui| {
                        for (i, c) in "©®™°±×÷€£¥§¶•…–—«»≤≥≠≈∞√πΩ←→↑↓♥★✓✗".chars().enumerate() {
                            if ui.add(egui::Button::new(c.to_string()).min_size(vec2(26.0, 26.0))).clicked() {
                                self.insert_into_text(&c.to_string());
                            }
                            if i % 8 == 7 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        });

        group(ui, "Intestazione e piè di pagina", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, "⤒", "Intestazione", true, false).on_hover_text("Aggiunge una casella in alto sulla pagina master").clicked() {
                    self.insert_header_footer(true);
                }
                if tile(ui, "⤓", "Piè di pagina", true, false).on_hover_text("Aggiunge una casella in basso sulla pagina master, con il numero di pagina").clicked() {
                    self.insert_header_footer(false);
                }
            });
        });
    }
}
