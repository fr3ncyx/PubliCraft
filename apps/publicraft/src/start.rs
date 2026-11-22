//! Schermata iniziale in stile Publisher: recenti a sinistra, formati di pagina e modelli a destra.

use super::*;
use egui::{CornerRadius, FontId, Frame as EFrame, Margin, Painter, Sense, Shape, Stroke, StrokeKind};
use egui::Rect as ERect;
use publicraft_doc::Align;

const RED: Rgb = Rgb(0xe3, 0x45, 0x30);
const RED_LIGHT: Rgb = Rgb(0xf6, 0xcd, 0xc6);
const TILE_W: f32 = 206.0;
const MAX_RECENT: usize = 12;

pub(crate) struct StartState {
    pub open: bool,
    pub search: String,
    pub tab: usize,
    pub section: Section,
    pub custom: bool,
    pub custom_mm: (f32, f32),
}

impl Default for StartState {
    fn default() -> Self {
        Self { open: true, search: String::new(), tab: 0, section: Section::Home, custom: false, custom_mm: (210.0, 297.0) }
    }
}

/// Voci della barra laterale (vista «File»).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Section {
    Home,
    New,
    Open,
    Info,
    Restore,
    Save,
    SaveAs,
    Print,
    Export,
    Close,
    Options,
}

const SIDE: [(Section, &str, &str); 11] = [
    (Section::Home, "⌂", "Home"),
    (Section::New, "＋", "Nuovo"),
    (Section::Open, "📂", "Apri"),
    (Section::Info, "ℹ", "Informazioni"),
    (Section::Restore, "↺", "Ripristina"),
    (Section::Save, "💾", "Salva"),
    (Section::SaveAs, "📝", "Salva con nome"),
    (Section::Print, "🖨", "Stampa"),
    (Section::Export, "📤", "Esporta"),
    (Section::Close, "✕", "Chiudi"),
    (Section::Options, "⚙", "Opzioni"),
];

#[derive(Clone, Copy)]
enum Item {
    Blank(PageSize),
    Custom,
    Template(usize),
}

const A4_L: PageSize = PageSize { w_mm: 297.0, h_mm: 210.0 };
const LETTER_L: PageSize = PageSize { w_mm: 279.4, h_mm: 215.9 };
const A3: PageSize = PageSize { w_mm: 297.0, h_mm: 420.0 };

/// Formati di pagina vuoti: A4 è il primo, come l'originale che partiva da solo.
const FORMATS: [(&str, PageSize); 6] = [
    ("Vuoto A4 (210 × 297 mm)", PageSize::A4),
    ("Vuoto A4 orizzontale", A4_L),
    ("Vuoto Lettera (8,5 × 11\")", PageSize::LETTER),
    ("Vuoto Lettera orizzontale", LETTER_L),
    ("Vuoto A5 (148 × 210 mm)", PageSize::A5),
    ("Vuoto A3 (297 × 420 mm)", A3),
];

const TEMPLATES: [(&str, PageSize); 4] = [
    ("Volantino", PageSize::A4),
    ("Invito di compleanno", PageSize { w_mm: 210.0, h_mm: 148.0 }),
    ("Biglietto da visita", PageSize { w_mm: 90.0, h_mm: 55.0 }),
    ("Lettera", PageSize::A4),
];

const SUGGESTED: [&str; 4] = ["Volantino", "Invito", "Biglietto", "Lettera"];

fn text(s: &str, pt: f32, color: Rgb, style: TextStyle) -> FrameKind {
    FrameKind::Text { text: s.into(), size_pt: pt, color, style }
}

fn plain(align: Align) -> TextStyle {
    TextStyle { align, ..Default::default() }
}

fn bold(align: Align) -> TextStyle {
    TextStyle { align, bold: true, ..Default::default() }
}

fn rect(fill: Rgb) -> FrameKind {
    FrameKind::Rectangle { fill, stroke: fill, stroke_mm: 0.0 }
}

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// Crea il documento di un modello.
pub(crate) fn build_template(i: usize) -> Document {
    let mut d = Document::new(TEMPLATES[i].1);
    let ink = Rgb(0x30, 0x30, 0x30);
    let white = Rgb(255, 255, 255);
    match i {
        0 => {
            d.add_frame(0, r(0.0, 0.0, 210.0, 70.0), rect(RED));
            d.add_frame(0, r(15.0, 18.0, 180.0, 36.0), text("Titolo dell'evento", 44.0, white, bold(Align::Center)));
            d.add_frame(0, r(15.0, 85.0, 180.0, 22.0), text("Sottotitolo o slogan", 24.0, RED, TextStyle { align: Align::Center, italic: true, ..Default::default() }));
            d.add_frame(0, r(20.0, 125.0, 170.0, 100.0), text("Descrivi qui il tuo evento: quando, dove e perché partecipare.\nAggiungi tutte le informazioni utili.", 16.0, ink, plain(Align::Left)));
            d.add_frame(0, r(0.0, 247.0, 210.0, 50.0), rect(RED_LIGHT));
            d.add_frame(0, r(15.0, 264.0, 180.0, 14.0), text("Telefono  ·  Email  ·  Indirizzo", 14.0, ink, plain(Align::Center)));
        }
        1 => {
            d.add_frame(0, r(0.0, 0.0, 210.0, 148.0), rect(RED_LIGHT));
            d.add_frame(0, r(12.0, 12.0, 186.0, 124.0), rect(white));
            d.add_frame(0, r(12.0, 30.0, 186.0, 22.0), text("Sei invitato!", 40.0, RED, bold(Align::Center)));
            d.add_frame(0, r(12.0, 62.0, 186.0, 14.0), text("Festa di compleanno di Nome", 22.0, ink, plain(Align::Center)));
            d.add_frame(0, r(12.0, 92.0, 186.0, 24.0), text("Sabato 20 giugno · ore 18:00\nVia Esempio 1, Città", 16.0, ink, plain(Align::Center)));
        }
        2 => {
            d.add_frame(0, r(0.0, 0.0, 7.0, 55.0), rect(RED));
            d.add_frame(0, r(14.0, 8.0, 70.0, 9.0), text("Nome Cognome", 15.0, ink, bold(Align::Left)));
            d.add_frame(0, r(14.0, 18.0, 70.0, 6.0), text("Ruolo o attività", 9.0, RED, plain(Align::Left)));
            d.add_frame(0, r(14.0, 32.0, 70.0, 16.0), text("+39 000 000 0000\nnome@esempio.it\nVia Esempio 1, Città", 8.0, ink, plain(Align::Left)));
        }
        _ => {
            d.add_frame(0, r(20.0, 20.0, 80.0, 22.0), text("Il tuo nome\nVia Esempio 1\n00000 Città", 11.0, ink, plain(Align::Left)));
            d.add_frame(0, r(140.0, 20.0, 50.0, 7.0), text("Città, 1 gennaio", 11.0, ink, plain(Align::Right)));
            d.add_frame(0, r(20.0, 60.0, 80.0, 22.0), text("Destinatario\nVia Esempio 2\n00000 Città", 11.0, ink, plain(Align::Left)));
            d.add_frame(0, r(20.0, 100.0, 170.0, 8.0), text("Oggetto: ", 12.0, ink, bold(Align::Left)));
            d.add_frame(0, r(20.0, 115.0, 170.0, 130.0), text("Gentile Destinatario,\n\nscrivi qui il testo della lettera.\n\nCordiali saluti", 12.0, ink, plain(Align::Left)));
            d.add_frame(0, r(120.0, 262.0, 70.0, 8.0), text("Firma", 12.0, ink, plain(Align::Right)));
        }
    }
    d
}

fn recent_file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("PubliCraft").join("recent.txt"))
}

pub(crate) fn load_recent() -> Vec<PathBuf> {
    let Some(p) = recent_file() else { return Vec::new() };
    std::fs::read_to_string(p)
        .map(|s| s.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).take(MAX_RECENT).collect())
        .unwrap_or_default()
}

fn aspect_fit(area: ERect, w: f32, h: f32) -> ERect {
    let s = (area.width() / w).min(area.height() / h);
    ERect::from_center_size(area.center(), vec2(w * s, h * s))
}

fn blank_page(p: &Painter, area: ERect, size: PageSize) {
    let pr = aspect_fit(area, size.w_mm, size.h_mm);
    let stroke = Stroke::new(1.0, Color32::from_gray(0x6a));
    let fold = (pr.width().min(pr.height()) * 0.18).max(8.0);
    let pts = vec![pr.left_top(), pr.right_top() - vec2(fold, 0.0), pr.right_top() + vec2(0.0, fold), pr.right_bottom(), pr.left_bottom()];
    p.add(Shape::convex_polygon(pts, Color32::WHITE, stroke));
    let tri = vec![pr.right_top() - vec2(fold, 0.0), pr.right_top() - vec2(fold, 0.0) + vec2(0.0, fold), pr.right_top() + vec2(0.0, fold)];
    p.add(Shape::convex_polygon(tri, Color32::from_gray(0xee), stroke));
}

fn name_of(item: Item) -> &'static str {
    match item {
        Item::Blank(s) => FORMATS.iter().find(|(_, f)| *f == s).map(|(n, _)| *n).unwrap_or("Vuoto"),
        Item::Custom => "Altri formati di pagina",
        Item::Template(i) => TEMPLATES[i].0,
    }
}

impl PubliCraft {
    pub(crate) fn add_recent(&mut self, path: &std::path::Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(MAX_RECENT);
        self.save_recent();
    }

    pub(crate) fn forget_recent(&mut self, path: &std::path::Path) {
        self.recent.retain(|p| p != path);
        self.save_recent();
    }

    fn save_recent(&self) {
        if let Some(f) = recent_file() {
            if let Some(dir) = f.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let text: Vec<String> = self.recent.iter().map(|p| p.display().to_string()).collect();
            let _ = std::fs::write(f, text.join("\n"));
        }
    }

    /// Crea un documento dalla schermata iniziale: il primo prende il posto di quello vuoto di partenza.
    fn start_doc(&mut self, doc: Document) {
        let mut st = DocState::new(self.next_num);
        st.saved = Document::new(doc.page_size);
        st.doc = doc;
        if self.started {
            self.next_num += 1;
            self.push_tab(st);
        } else {
            st.num = self.d.num;
            self.d = st;
            self.started = true;
            self.drag = None;
            self.view.master = false;
        }
        self.start.open = false;
        self.fit = Some(Fit::Page);
        self.scroll_target = Some(Vec2::ZERO);
    }

    fn matches(&self, item: Item) -> bool {
        let q = self.start.search.trim().to_lowercase();
        q.is_empty() || name_of(item).to_lowercase().contains(&q)
    }

    /// Un riquadro con anteprima e nome; true se è stato cliccato.
    fn tile(ui: &mut egui::Ui, item: Item, w: f32, ph: f32) -> bool {
        let (rect, resp) = ui.allocate_exact_size(vec2(w, ph + 62.0), Sense::click());
        let p = ui.painter();
        let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
        if t > 0.0 {
            p.rect_filled(rect, CornerRadius::ZERO, rgb(RED_LIGHT).gamma_multiply(t));
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let area = ERect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(w - 16.0, ph));
        match item {
            Item::Blank(s) => blank_page(p, area, s),
            Item::Custom => {
                let g = p.layout_no_wrap("···".into(), FontId::proportional(34.0), Color32::from_gray(0x60));
                blank_page(p, area, PageSize { w_mm: 210.0, h_mm: 297.0 });
                p.galley(area.center() - g.size() / 2.0, g, look::text());
            }
            Item::Template(i) => {
                let doc = build_template(i);
                let pr = aspect_fit(area, doc.page_size.w_mm, doc.page_size.h_mm);
                p.rect_filled(pr, CornerRadius::ZERO, Color32::WHITE);
                canvas::paint_doc_preview(p, &doc, pr);
                p.rect_stroke(pr, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_gray(0x8a)), StrokeKind::Outside);
            }
        }
        let g = p.layout(name_of(item).to_string(), FontId::proportional(13.0), look::text(), w - 16.0);
        p.galley(egui::pos2(rect.min.x + 10.0, rect.min.y + ph + 16.0), g, look::text());
        resp.clicked()
    }

    /// Una riga della barra laterale; true se è stata cliccata (e abilitata).
    fn side_row(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, enabled: bool) -> bool {
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), sense);
        let p = ui.painter();
        if active {
            p.rect_filled(rect, CornerRadius::ZERO, Color32::from_white_alpha(48));
        } else if enabled {
            let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
            if t > 0.0 {
                p.rect_filled(rect, CornerRadius::ZERO, Color32::from_white_alpha((24.0 * t) as u8));
            }
        }
        let col = if enabled { Color32::WHITE } else { Color32::from_white_alpha(110) };
        p.text(egui::pos2(rect.min.x + 24.0, rect.center().y), egui::Align2::LEFT_CENTER, icon, FontId::proportional(18.0), col);
        p.text(egui::pos2(rect.min.x + 62.0, rect.center().y), egui::Align2::LEFT_CENTER, label, FontId::proportional(16.0), col);
        enabled && resp.clicked()
    }

    fn recent_list(&self, ui: &mut egui::Ui, open: &mut Option<PathBuf>) {
        if self.recent.is_empty() {
            ui.label("I documenti che apri compariranno qui.");
            return;
        }
        for p in &self.recent {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let dir = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
            let r = ui.add(egui::Button::new(RichText::new(name).size(15.0).color(look::text())).frame(false));
            ui.label(RichText::new(dir).size(11.0).color(look::text_dim()));
            if r.on_hover_text(p.display().to_string()).clicked() {
                *open = Some(p.clone());
            }
            ui.add_space(4.0);
        }
    }

    fn page_title(ui: &mut egui::Ui, t: &str) {
        ui.label(RichText::new(t).size(30.0).color(look::text()));
        ui.add_space(16.0);
    }

    pub(crate) fn start_screen(&mut self, ui: &mut egui::Ui) {
        ui.set_opacity(self.bs_alpha);
        let ctx = ui.ctx().clone();
        let mut chosen: Option<Item> = None;
        let mut open_recent: Option<PathBuf> = None;
        let mut open_other = false;
        let mut import_pdf = false;
        let mut back = false;
        let mut act: Option<Section> = None;
        let started = self.started;
        let accent = craft_theme::ACCENT_PUBLICRAFT;

        egui::Panel::left("start-side").resizable(false).exact_size(260.0).frame(EFrame::new().fill(accent).inner_margin(Margin::ZERO)).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.add_space(10.0);
            if Self::side_row(ui, "←", "", false, started) {
                back = true;
            }
            ui.add_space(6.0);
            for (sec, icon, label) in SIDE {
                let needs_doc = !matches!(sec, Section::Home | Section::New | Section::Open | Section::Close | Section::Options);
                let active = self.start.section == sec && !matches!(sec, Section::Save | Section::SaveAs | Section::Close);
                if Self::side_row(ui, icon, label, active, started || !needs_doc) {
                    match sec {
                        Section::Save | Section::SaveAs | Section::Close => act = Some(sec),
                        _ => self.start.section = sec,
                    }
                }
            }
        });

        egui::CentralPanel::default().frame(EFrame::new().fill(look::panel()).inner_margin(Margin::symmetric(40, 30))).show(ui, |ui| {
            match self.start.section {
                Section::New => self.new_page(ui, &mut chosen),
                Section::Open => {
                    Self::page_title(ui, "Apri");
                    if ui.add(egui::Button::new(RichText::new("📂  Sfoglia…").size(15.0).color(Color32::WHITE)).fill(accent).min_size(vec2(150.0, 30.0))).clicked() {
                        open_other = true;
                    }
                    ui.add_space(6.0);
                    if ui.add(egui::Button::new(RichText::new("📄  Importa PDF…").size(15.0)).min_size(vec2(150.0, 30.0))).on_hover_text("Trasforma un PDF in un documento modificabile: testi, immagini e campi diventano riquadri").clicked() {
                        import_pdf = true;
                    }
                    ui.add_space(18.0);
                    ui.label(RichText::new("Recenti").size(20.0));
                    ui.add_space(8.0);
                    self.recent_list(ui, &mut open_recent);
                }
                Section::Info => {
                    Self::page_title(ui, "Informazioni");
                    let d = &self.d.doc;
                    let objects: usize = d.pages.iter().map(|p| p.frames.len()).sum();
                    let path = self.d.file.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "Non ancora salvato".into());
                    egui::Grid::new("info-grid").num_columns(2).spacing(vec2(24.0, 8.0)).show(ui, |ui| {
                        let row = |ui: &mut egui::Ui, k: &str, v: String| {
                            ui.label(RichText::new(k).color(look::text_dim()));
                            ui.label(v);
                            ui.end_row();
                        };
                        row(ui, "Nome", self.d.title());
                        row(ui, "Percorso", path);
                        row(ui, "Formato pagina", format!("{:.0} × {:.0} mm", d.page_size.w_mm, d.page_size.h_mm));
                        row(ui, "Pagine", d.pages.len().to_string());
                        row(ui, "Oggetti", objects.to_string());
                        row(ui, "Stato", if self.d.dirty() { "Con modifiche non salvate".into() } else { "Salvato".into() });
                    });
                }
                Section::Restore => {
                    Self::page_title(ui, "Ripristina");
                    ui.label("Torna all'ultima versione salvata del documento. Puoi annullare il ripristino con Annulla.");
                    ui.add_space(14.0);
                    let enabled = self.d.dirty();
                    let btn = egui::Button::new(RichText::new("Ripristina ultima versione salvata").color(Color32::WHITE)).fill(accent).min_size(vec2(260.0, 30.0));
                    if ui.add_enabled(enabled, btn).clicked() {
                        self.checkpoint();
                        self.d.doc = self.d.saved.clone();
                        self.d.sel.clear();
                        self.start.open = false;
                        self.message = Some(("Ripristinata l'ultima versione salvata".into(), false));
                    }
                    if !enabled {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Non ci sono modifiche da annullare.").color(look::text_dim()));
                    }
                }
                Section::Print => self.print_page(ui),
                Section::Export => self.export_page(ui),
                Section::Options => {
                    Self::page_title(ui, "Opzioni");
                    ui.label(format!("PubliCraft {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(6.0);
                    ui.add_space(16.0);
                    ui.label(RichText::new("Aspetto").size(20.0));
                    ui.add_space(8.0);
                    let dark = craft_theme::is_dark();
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("☀  Chiaro").selected(!dark).min_size(vec2(120.0, 30.0))).clicked() && dark {
                            self.set_dark(&ctx, false);
                        }
                        if ui.add(egui::Button::new("🌙  Scuro").selected(dark).min_size(vec2(120.0, 30.0))).clicked() && !dark {
                            self.set_dark(&ctx, true);
                        }
                    });
                }
                _ => {
                    Self::page_title(ui, "Benvenuto");
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                        for item in [Item::Blank(PageSize::A4), Item::Template(0), Item::Template(1), Item::Template(3)] {
                            if Self::tile(ui, item, 176.0, 190.0) {
                                chosen = Some(item);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("Altri modelli →").size(14.0).color(accent)).frame(false)).clicked() {
                            self.start.section = Section::New;
                        }
                        ui.add_space(16.0);
                        if ui.add(egui::Button::new(RichText::new("📄 Importa un PDF come modello").size(14.0).color(accent)).frame(false)).clicked() {
                            import_pdf = true;
                        }
                    });
                    ui.add_space(22.0);
                    ui.label(RichText::new("Recenti").size(20.0));
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| self.recent_list(ui, &mut open_recent));
                }
            }
        });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.started {
            back = true;
        }
        if back {
            self.start.open = false;
        }
        match act {
            Some(Section::Save) => {
                if self.save(false) {
                    self.start.open = false;
                }
            }
            Some(Section::SaveAs) => {
                if self.save(true) {
                    // Dopo «Salva con nome» l'app si chiude.
                    self.request_exit(&ctx);
                }
            }
            Some(Section::Close) => self.request_exit(&ctx),
            _ => {}
        }
        if open_other {
            self.open();
        }
        if import_pdf {
            self.import_pdf_dialog();
        }
        if let Some(p) = open_recent {
            self.open_path(p);
        }
        match chosen {
            Some(Item::Blank(s)) => self.start_doc(Document::new(s)),
            Some(Item::Template(i)) => self.start_doc(build_template(i)),
            Some(Item::Custom) => self.start.custom = true,
            None => {}
        }

        if self.start.custom {
            let mut open = true;
            let mut create = false;
            egui::Window::new("Altri formati di pagina").open(&mut open).collapsible(false).resizable(false).show(&ctx, |ui| {
                egui::Grid::new("custom-size").num_columns(2).show(ui, |ui| {
                    ui.label("Larghezza (mm)");
                    ui.add(egui::DragValue::new(&mut self.start.custom_mm.0).range(20.0..=2000.0).speed(1.0));
                    ui.end_row();
                    ui.label("Altezza (mm)");
                    ui.add(egui::DragValue::new(&mut self.start.custom_mm.1).range(20.0..=2000.0).speed(1.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                let btn = egui::Button::new(RichText::new("Crea").strong().color(Color32::WHITE)).fill(accent).min_size(vec2(90.0, 26.0));
                create = ui.add(btn).clicked();
            });
            self.start.custom = open;
            if create {
                let (w, h) = self.start.custom_mm;
                self.start.custom = false;
                self.start_doc(Document::new(PageSize { w_mm: w, h_mm: h }));
            }
        }
    }

    /// «Nuovo»: formati di pagina e modelli, con ricerca.
    fn new_page(&mut self, ui: &mut egui::Ui, chosen: &mut Option<Item>) {
        let accent = craft_theme::ACCENT_PUBLICRAFT;
        ui.add(egui::TextEdit::singleline(&mut self.start.search).hint_text("Cerca modelli").desired_width(480.0).margin(Margin::symmetric(8, 7)));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ricerche suggerite:").size(12.0));
            for s in SUGGESTED {
                if ui.add(egui::Button::new(RichText::new(s).size(12.0).color(look::text())).frame(false)).clicked() {
                    self.start.search = s.to_string();
                }
            }
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            for (i, l) in ["IN EVIDENZA", "INTEGRATI"].iter().enumerate() {
                let active = self.start.tab == i;
                let col = if active { accent } else { look::text() };
                let resp = ui.add(egui::Button::new(RichText::new(*l).size(13.0).color(col)).frame(false));
                if active {
                    ui.painter().hline(resp.rect.x_range(), resp.rect.bottom(), Stroke::new(2.0, col));
                }
                if resp.clicked() {
                    self.start.tab = i;
                }
            }
        });
        ui.add_space(10.0);

        let mut items: Vec<Item> = Vec::new();
        if self.start.tab == 0 {
            items.extend(FORMATS.iter().map(|(_, s)| Item::Blank(*s)));
            items.push(Item::Custom);
        }
        items.extend((0..TEMPLATES.len()).map(Item::Template));
        let items: Vec<Item> = items.into_iter().filter(|it| self.matches(*it)).collect();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if items.is_empty() {
                ui.label("Nessun modello corrisponde alla ricerca.");
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                for item in &items {
                    if Self::tile(ui, *item, TILE_W, 196.0) {
                        *chosen = Some(*item);
                    }
                }
            });
        });
    }
}
