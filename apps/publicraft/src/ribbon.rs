//! Barra multifunzione in stile Publisher: schede File, Home e Visualizza.

use super::*;
use egui::{Align, CornerRadius, FontId, Layout, Painter, Response, Sense, Stroke, Ui};
use egui::Rect as ERect;
use publicraft_doc::{AlignKind, CaseKind, Family, ListKind, Wrap, change_case};

const ACCENT: Color32 = craft_theme::ACCENT_PUBLICRAFT;
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
    craft_theme::TEXT_DIM.gamma_multiply(0.6)
}

/// Pulsante grande con icona sopra e nome sotto.
fn tile(ui: &mut Ui, glyph: &str, label: &str, enabled: bool, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(TILE_W, BODY_H), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::ZERO, ACCENT.gamma_multiply(0.3));
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), egui::StrokeKind::Inside);
    } else if enabled && resp.hovered() {
        p.rect_filled(rect, CornerRadius::ZERO, craft_theme::PANEL_RAISED);
    }
    let col = if enabled { craft_theme::TEXT } else { dim() };
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
        p.rect_filled(rect, CornerRadius::ZERO, ACCENT.gamma_multiply(0.35));
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), egui::StrokeKind::Inside);
    } else if enabled && resp.hovered() {
        p.rect_filled(rect, CornerRadius::ZERO, craft_theme::PANEL_RAISED);
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, craft_theme::BORDER), egui::StrokeKind::Inside);
    }
    draw(p, rect, if enabled { craft_theme::TEXT } else { dim() });
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
                ui.label(egui::RichText::new(title).size(11.5).color(craft_theme::TEXT_DIM));
            });
        })
        .response;
    ui.painter().vline(r.rect.right() + 5.0, r.rect.y_range(), Stroke::new(1.0, craft_theme::BORDER));
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
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let file = ui.add(egui::Button::new(egui::RichText::new("File").strong().color(Color32::WHITE)).fill(ACCENT).min_size(vec2(64.0, 26.0)));
            menu(&file, 200.0, |ui| {
                let item = |ui: &mut Ui, t: &str| ui.add(egui::Button::new(t).frame(false).min_size(vec2(180.0, 24.0))).clicked();
                if item(ui, "Nuovo") {
                    self.new_doc();
                }
                if item(ui, "Apri…") {
                    self.open();
                }
                if item(ui, "Salva") {
                    self.save(false);
                }
                if item(ui, "Salva con nome…") && self.save(true) {
                    // Dopo «Salva con nome» l'app si chiude.
                    self.request_exit(ui.ctx());
                }
                ui.separator();
                if item(ui, "Esci") {
                    self.request_exit(ui.ctx());
                }
            });
            ui.add_space(4.0);
            if sbtn(ui, "↶", "Annulla (Ctrl+Z)", !self.d.undo.is_empty(), false, 26.0).clicked() {
                self.undo();
            }
            if sbtn(ui, "↷", "Ripeti (Ctrl+Y)", !self.d.redo.is_empty(), false, 26.0).clicked() {
                self.redo();
            }
            ui.add_space(10.0);
            for (t, l) in [(RibbonTab::Home, "Home"), (RibbonTab::Insert, "Inserisci"), (RibbonTab::View, "Visualizza")] {
                let active = self.tab == t;
                let r = ui.add(egui::Button::new(egui::RichText::new(l).size(14.0).color(if active { ACCENT.gamma_multiply(1.3) } else { craft_theme::TEXT })).frame(false).min_size(vec2(70.0, 26.0)));
                if active {
                    ui.painter().hline(r.rect.x_range(), r.rect.bottom(), Stroke::new(3.0, ACCENT));
                }
                if r.clicked() {
                    self.tab = t;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(egui::RichText::new(self.d.title()).color(craft_theme::TEXT_DIM));
            });
        });
        ui.add_space(2.0);
        ui.separator();
        egui::ScrollArea::horizontal().id_salt("ribbon-scroll").auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                match self.tab {
                    RibbonTab::Home => self.home(ui),
                    RibbonTab::Insert => self.insert_tab(ui),
                    RibbonTab::View => self.view_tab(ui),
                }
            });
        });
        ui.add_space(2.0);
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
