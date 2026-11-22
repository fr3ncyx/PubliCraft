#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assoc;
mod canvas;
mod export;
mod fonts;
mod look;
mod outui;
mod pdfimport;
mod print;
mod ribbon;
mod start;

use egui::{Color32, Pos2, RichText, Vec2, vec2};
use publicraft_doc::{Align as DAlign, Document, Frame, FrameKind, MASTER, PageSize, Rect, Rgb, TextStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) const PX_PER_MM: f32 = 96.0 / 25.4;
pub(crate) const MM_PER_PT: f32 = 25.4 / 72.0;
pub(crate) const MIN_FRAME_MM: f32 = 3.0;
const HISTORY_MAX: usize = 200;
const COALESCE: Duration = Duration::from_millis(900);
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeTool {
    Rectangle,
    Ellipse,
    Triangle,
    Line,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Select,
    Text,
    Shape(ShapeTool),
}

pub(crate) enum Drag {
    Create { page: usize, start: Pos2, tool: Tool },
    Move { page: usize, grab: Pos2, starts: Vec<(u64, f32, f32)> },
    Resize { page: usize, id: u64 },
    Marquee { page: usize, start: Pos2 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fit {
    Page,
    Width,
    Selection,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RibbonTab {
    Home,
    Insert,
    View,
}

/// Tutto ciò che appartiene a un documento aperto (si scambia quando si cambia finestra).
pub(crate) struct DocState {
    pub doc: Document,
    pub page: usize,
    pub sel: Vec<u64>,
    pub file: Option<PathBuf>,
    pub undo: Vec<Document>,
    pub redo: Vec<Document>,
    /// Stato prima di un trascinamento in corso.
    pub pending: Option<Document>,
    /// Ultima modifica dal pannello proprietà (per raggruppare la digitazione).
    pub props_last: Option<(u64, Instant)>,
    pub zoom: f32,
    pub num: usize,
    /// Contenuto all'ultimo salvataggio (per sapere se ci sono modifiche).
    pub saved: Document,
}

impl DocState {
    fn new(num: usize) -> Self {
        Self {
            doc: Document::new(PageSize::A4),
            page: 0,
            sel: Vec::new(),
            file: None,
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            props_last: None,
            zoom: 0.8,
            num,
            saved: Document::new(PageSize::A4),
        }
    }

    pub fn dirty(&self) -> bool {
        self.doc != self.saved
    }

    fn title(&self) -> String {
        match &self.file {
            Some(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            None => format!("Documento {}", self.num),
        }
    }
}

pub(crate) struct ViewOpts {
    pub master: bool,
    pub spread: bool,
    pub boundaries: bool,
    pub guides: bool,
    pub fields: bool,
    pub rulers: bool,
    pub nav: bool,
    pub scratch: bool,
    pub graphics: bool,
    pub baselines: bool,
    pub marks: bool,
}

#[derive(Default)]
pub(crate) struct FindState {
    pub open: bool,
    pub needle: String,
    pub with: String,
    pub match_case: bool,
    pub note: String,
    pub focus: bool,
    pub last: usize,
}

pub(crate) struct Tex {
    pub handle: egui::TextureHandle,
    pub size: [usize; 2],
}

pub(crate) struct PubliCraft {
    pub d: DocState,
    pub others: Vec<DocState>,
    pub next_num: usize,
    pub tool: Tool,
    pub drag: Option<Drag>,
    pub view: ViewOpts,
    pub tab: RibbonTab,
    pub message: Option<(String, bool)>,
    pub clipboard: Vec<Frame>,
    pub fmt_painter: Option<(TextStyle, f32, Rgb)>,
    pub fit: Option<Fit>,
    pub scroll_target: Option<Vec2>,
    pub scroll_off: Vec2,
    pub viewport: Vec2,
    /// Posizione del puntatore in mm sulla pagina (indice, x, y).
    pub cursor: Option<(usize, f32, f32)>,
    pub find: FindState,
    pub textures: HashMap<String, Option<Tex>>,
    pub table_dims: (usize, usize),
    pub last_title: String,
    pub confirm_exit: bool,
    pub closing_ok: bool,
    pub start: start::StartState,
    /// Vero dopo che è stato creato o aperto il primo documento.
    pub started: bool,
    pub recent: Vec<PathBuf>,
    /// Opacità della schermata File (dissolvenza all'apertura).
    pub bs_alpha: f32,
    /// Testo della ricerca comandi e se l'elenco è aperto.
    pub cmd: String,
    pub cmd_open: bool,
    pub out: outui::OutState,
}

pub(crate) fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

impl PubliCraft {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        craft_theme::load_mode("PubliCraft");
        look::apply(&cc.egui_ctx);
        fonts::install(&cc.egui_ctx);
        let mut me = Self {
            d: DocState::new(1),
            others: Vec::new(),
            next_num: 2,
            tool: Tool::Select,
            drag: None,
            view: ViewOpts {
                master: false,
                spread: false,
                boundaries: false,
                guides: false,
                fields: false,
                rulers: true,
                nav: true,
                scratch: true,
                graphics: false,
                baselines: false,
                marks: false,
            },
            tab: RibbonTab::Home,
            message: None,
            clipboard: Vec::new(),
            fmt_painter: None,
            fit: Some(Fit::Page),
            scroll_target: None,
            scroll_off: Vec2::ZERO,
            viewport: vec2(800.0, 600.0),
            cursor: None,
            find: FindState::default(),
            textures: HashMap::new(),
            table_dims: (3, 3),
            last_title: String::new(),
            confirm_exit: false,
            closing_ok: false,
            start: start::StartState::default(),
            started: false,
            recent: start::load_recent(),
            bs_alpha: 1.0,
            cmd: String::new(),
            cmd_open: false,
            out: outui::OutState::new(),
        };
        // Doppio clic su un file .pubcraft: l'app parte e lo apre al posto del documento vuoto.
        if let Some(arg) = std::env::args().nth(1) {
            let path = PathBuf::from(arg);
            if path.is_file() {
                me.open_path(path);
                if me.others.len() == 1 && !me.others[0].dirty() {
                    me.others.clear();
                }
            }
        }
        // Per le prove: PUBLICRAFT_IMPORT=file.pdf importa subito quel PDF.
        if let Ok(pdf) = std::env::var("PUBLICRAFT_IMPORT") {
            if let Ok((doc, msg)) = pdfimport::import_pdf(std::path::Path::new(&pdf)) {
                me.add_imported(doc, msg);
            }
        }
        // Per le prove: PUBLICRAFT_SECTION=print|export|options|info apre quella pagina di «File».
        if let Ok(sec) = std::env::var("PUBLICRAFT_SECTION") {
            use start::Section;
            let s = match sec.as_str() {
                "print" => Some(Section::Print),
                "export" => Some(Section::Export),
                "options" => Some(Section::Options),
                "info" => Some(Section::Info),
                _ => None,
            };
            if let Some(s) = s {
                me.start.section = s;
                me.start.open = true;
            }
        }
        me
    }


    /// Pagina su cui si lavora: la master nella vista master.
    pub fn pg(&self) -> usize {
        if self.view.master { MASTER } else { self.d.page }
    }

    pub fn scale(&self) -> f32 {
        PX_PER_MM * self.d.zoom
    }

    pub fn set_zoom(&mut self, z: f32) {
        self.d.zoom = z.clamp(0.1, 5.0);
    }

    // ---- Cronologia ----

    pub fn checkpoint(&mut self) {
        let snap = self.d.doc.clone();
        self.push_undo(snap);
    }

    pub fn push_undo(&mut self, snap: Document) {
        self.d.undo.push(snap);
        if self.d.undo.len() > HISTORY_MAX {
            self.d.undo.remove(0);
        }
        self.d.redo.clear();
    }

    fn restore(&mut self, doc: Document) {
        self.d.doc = doc;
        self.d.page = self.d.page.min(self.d.doc.pages.len() - 1);
        let pg = self.pg();
        let doc = &self.d.doc;
        self.d.sel.retain(|id| doc.frame(pg, *id).is_some());
        self.drag = None;
        self.d.pending = None;
        self.d.props_last = None;
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.d.undo.pop() {
            let cur = self.d.doc.clone();
            self.d.redo.push(cur);
            self.restore(prev);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.d.redo.pop() {
            let cur = self.d.doc.clone();
            self.d.undo.push(cur);
            self.restore(next);
        }
    }

    /// Esegue una modifica al documento registrandola nella cronologia se ha cambiato qualcosa.
    pub fn mutate(&mut self, f: impl FnOnce(&mut Document, usize, &[u64])) {
        let before = self.d.doc.clone();
        let pg = self.pg();
        let sel = self.d.sel.clone();
        f(&mut self.d.doc, pg, &sel);
        if self.d.doc != before {
            self.push_undo(before);
        }
    }

    /// Applica una modifica a testo, corpo, colore e stile di tutte le caselle di testo selezionate.
    pub fn edit_text(&mut self, f: impl Fn(&mut String, &mut f32, &mut Rgb, &mut TextStyle)) {
        self.mutate(|doc, pg, sel| {
            for id in sel {
                if let Some(Frame { kind: FrameKind::Text { text, size_pt, color, style }, .. }) = doc.frame_mut(pg, *id) {
                    f(text, size_pt, color, style);
                }
            }
        });
    }

    /// Corpo, colore e stile della prima casella di testo selezionata.
    pub fn selected_text(&self) -> Option<(f32, Rgb, TextStyle)> {
        let pg = self.pg();
        self.d.sel.iter().find_map(|id| match self.d.doc.frame(pg, *id).map(|f| &f.kind) {
            Some(FrameKind::Text { size_pt, color, style, .. }) => Some((*size_pt, *color, *style)),
            _ => None,
        })
    }

    /// Passa dalla modalità chiara a quella scura (e viceversa) e la ricorda.
    pub fn set_dark(&mut self, ctx: &egui::Context, dark: bool) {
        craft_theme::save_mode("PubliCraft", dark);
        look::apply(ctx);
    }

    // ---- File e documenti ----

    /// «Nuovo»: mostra la schermata iniziale con formati e modelli.
    pub fn new_doc(&mut self) {
        self.start.section = start::Section::New;
        self.start.open = true;
    }

    fn push_tab(&mut self, st: DocState) {
        let old = std::mem::replace(&mut self.d, st);
        self.others.push(old);
        self.drag = None;
        self.view.master = false;
        self.fit = Some(Fit::Page);
        self.scroll_target = Some(Vec2::ZERO);
    }

    pub fn switch_to(&mut self, i: usize) {
        if i < self.others.len() {
            std::mem::swap(&mut self.d, &mut self.others[i]);
            self.drag = None;
            self.view.master = false;
            self.scroll_target = Some(Vec2::ZERO);
        }
    }

    pub fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Documento PubliCraft", &["pubcraft", "pcraft"]).pick_file() else { return };
        self.open_path(path);
    }

    pub fn open_path(&mut self, path: PathBuf) {
        let loaded = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| Document::from_json(s.trim_start_matches('\u{feff}')).map_err(|e| e.to_string()));
        match loaded {
            Ok(d) if !d.pages.is_empty() => {
                let mut st = DocState::new(self.next_num);
                self.next_num += 1;
                st.saved = d.clone();
                st.doc = d;
                st.file = Some(path.clone());
                self.push_tab(st);
                if !self.started {
                    // Il documento vuoto di partenza non serve più.
                    self.others.clear();
                    self.started = true;
                }
                self.start.open = false;
                self.add_recent(&path);
                self.message = Some((format!("Aperto {}", path.display()), false));
            }
            Ok(_) => self.message = Some(("Il documento non contiene pagine".into(), true)),
            Err(e) => {
                self.forget_recent(&path);
                self.message = Some((format!("Impossibile aprire il file: {e}"), true));
            }
        }
    }

    /// Salva il documento; restituisce true se il file è stato scritto.
    pub fn save(&mut self, as_new: bool) -> bool {
        let path = match (&self.d.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Documento PubliCraft", &["pubcraft"]).set_file_name("Senza titolo.pubcraft").save_file() {
                Some(p) => p,
                None => return false,
            },
        };
        let path = if path.extension().is_none() { path.with_extension("pubcraft") } else { path };
        let result = self.d.doc.to_json().map_err(|e| e.to_string()).and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()));
        let ok = result.is_ok();
        self.message = Some(match result {
            Ok(()) => {
                let m = format!("Salvato {}", path.display());
                self.add_recent(&path);
                self.d.file = Some(path);
                self.d.saved = self.d.doc.clone();
                (m, false)
            }
            Err(e) => (format!("Salvataggio non riuscito: {e}"), true),
        });
        ok
    }

    pub fn any_dirty(&self) -> bool {
        self.d.dirty() || self.others.iter().any(|o| o.dirty())
    }

    /// Salva tutti i documenti con modifiche; false se uno non è stato salvato.
    fn save_all(&mut self) -> bool {
        if self.d.dirty() && !self.save(false) {
            return false;
        }
        for i in 0..self.others.len() {
            if self.others[i].dirty() {
                self.switch_to(i);
                let ok = self.save(false);
                self.switch_to(i);
                if !ok {
                    return false;
                }
            }
        }
        true
    }

    /// Chiude l'app; con modifiche non salvate chiede prima conferma.
    pub fn request_exit(&mut self, ctx: &egui::Context) {
        if self.any_dirty() {
            self.confirm_exit = true;
        } else {
            self.closing_ok = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    // ---- Inserimento ----

    pub fn create(&mut self, tool: Tool, rect: Rect, page: usize) {
        let accent = Rgb(0xf6, 0xcd, 0xc6);
        let line = Rgb(0xe3, 0x45, 0x30);
        let kind = match tool {
            Tool::Text => FrameKind::Text { text: "Testo".into(), size_pt: 14.0, color: Rgb(0x20, 0x20, 0x20), style: TextStyle::default() },
            Tool::Shape(ShapeTool::Rectangle) => FrameKind::Rectangle { fill: accent, stroke: line, stroke_mm: 0.5 },
            Tool::Shape(ShapeTool::Ellipse) => FrameKind::Ellipse { fill: accent, stroke: line, stroke_mm: 0.5 },
            Tool::Shape(ShapeTool::Triangle) => FrameKind::Triangle { fill: accent, stroke: line, stroke_mm: 0.5 },
            Tool::Shape(ShapeTool::Line) => FrameKind::Line { stroke: line, stroke_mm: 0.8 },
            Tool::Select => return,
        };
        if self.d.pending.is_none() {
            self.checkpoint();
        }
        if let Some(id) = self.d.doc.add_frame(page, rect, kind) {
            self.d.sel = vec![id];
        }
        self.tool = Tool::Select;
    }

    /// Posizione di partenza per un nuovo elemento: al centro della parte visibile della pagina.
    fn insert_rect(&self, w: f32, h: f32) -> Rect {
        let s = self.d.doc.page_size;
        Rect { x: ((s.w_mm - w) / 2.0).max(0.0), y: ((s.h_mm - h) / 2.0).max(0.0), w, h }
    }

    pub fn insert_picture(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Immagini", &["png", "jpg", "jpeg", "bmp", "gif"]).pick_file() else { return };
        match image::image_dimensions(&path) {
            Ok((w, h)) if w > 0 && h > 0 => {
                // Dimensioni a 96 dpi, ridotte se sforano la pagina.
                let mut wm = w as f32 / PX_PER_MM;
                let mut hm = h as f32 / PX_PER_MM;
                let max_w = self.d.doc.page_size.w_mm * 0.8;
                if wm > max_w {
                    hm *= max_w / wm;
                    wm = max_w;
                }
                let rect = self.insert_rect(wm, hm);
                self.checkpoint();
                let pg = self.pg();
                if let Some(id) = self.d.doc.add_frame(pg, rect, FrameKind::Picture { path: path.to_string_lossy().into_owned() }) {
                    self.d.sel = vec![id];
                }
            }
            _ => self.message = Some(("Immagine non valida o formato non supportato".into(), true)),
        }
    }

    pub fn insert_table(&mut self, rows: usize, cols: usize) {
        let rect = self.insert_rect(cols as f32 * 30.0, rows as f32 * 9.0);
        self.checkpoint();
        let pg = self.pg();
        if let Some(id) = self.d.doc.add_frame(pg, rect, FrameKind::table(rows, cols)) {
            self.d.sel = vec![id];
        }
    }

    pub fn insert_page_at(&mut self, at: usize) {
        self.checkpoint();
        let i = self.d.doc.insert_page(at);
        self.go_to_page(i);
    }

    pub fn duplicate_page(&mut self) {
        self.checkpoint();
        if let Some(i) = self.d.doc.duplicate_page(self.d.page) {
            self.go_to_page(i);
        }
    }

    /// Nuova casella di testo al centro della pagina.
    pub fn insert_text(&mut self, text: &str, size_pt: f32, w: f32, h: f32) {
        let rect = self.insert_rect(w, h);
        self.checkpoint();
        let pg = self.pg();
        let kind = FrameKind::Text { text: text.into(), size_pt, color: Rgb(0x20, 0x20, 0x20), style: TextStyle::default() };
        if let Some(id) = self.d.doc.add_frame(pg, rect, kind) {
            self.d.sel = vec![id];
        }
    }

    /// Aggiunge il testo alla prima casella selezionata, oppure ne crea una nuova.
    pub fn insert_into_text(&mut self, add: &str) {
        if self.selected_text().is_some() {
            let add = add.to_string();
            self.edit_text(move |t, _, _, _| t.push_str(&add));
        } else {
            self.insert_text(add, 14.0, 60.0, 15.0);
        }
    }

    /// Intestazione o piè di pagina: casella sulla pagina master (con il numero di pagina nel piè).
    pub fn insert_header_footer(&mut self, header: bool) {
        let s = self.d.doc.page_size;
        let h = 10.0;
        let rect = Rect { x: 12.7, y: if header { 8.0 } else { s.h_mm - 8.0 - h }, w: s.w_mm - 25.4, h };
        self.checkpoint();
        let style = TextStyle { align: if header { DAlign::Left } else { DAlign::Center }, ..Default::default() };
        let text = if header { "Intestazione" } else { "Pagina {pagina}" };
        let kind = FrameKind::Text { text: text.into(), size_pt: 10.0, color: Rgb(0x50, 0x50, 0x50), style };
        self.d.doc.add_frame(MASTER, rect, kind);
        let what = if header { "Intestazione" } else { "Piè di pagina" };
        self.message = Some((format!("{what} aggiunto alla pagina master"), false));
    }

    pub fn today() -> String {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let days = secs.div_euclid(86400);
        // Da giorni dal 1970 a data civile (algoritmo di Howard Hinnant).
        let z = days + 719468;
        let era = z.div_euclid(146097);
        let doe = z.rem_euclid(146097);
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + (m <= 2) as i64;
        format!("{d:02}/{m:02}/{y}")
    }

    // ---- Appunti e selezione ----

    pub fn copy(&mut self) {
        let pg = self.pg();
        let sel = self.d.sel.clone();
        self.clipboard = self.d.doc.copy_frames(pg, &sel);
    }

    pub fn delete_selected(&mut self) {
        if self.d.sel.is_empty() {
            return;
        }
        self.checkpoint();
        let pg = self.pg();
        let sel = std::mem::take(&mut self.d.sel);
        self.d.doc.remove_frames(pg, &sel);
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selected();
    }

    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.checkpoint();
        let pg = self.pg();
        let frames = self.clipboard.clone();
        let ids = self.d.doc.paste_frames(pg, &frames, 5.0, 5.0);
        // Il prossimo incolla si sposta ancora, come in Publisher.
        for f in &mut self.clipboard {
            f.rect.x += 5.0;
            f.rect.y += 5.0;
        }
        self.d.sel = ids;
    }

    pub fn select_all(&mut self) {
        let pg = self.pg();
        self.d.sel = self.d.doc.page(pg).map(|p| p.frames.iter().map(|f| f.id).collect()).unwrap_or_default();
    }

    pub fn select_texts(&mut self) {
        let pg = self.pg();
        self.d.sel = self
            .d
            .doc
            .page(pg)
            .map(|p| p.frames.iter().filter(|f| matches!(f.kind, FrameKind::Text { .. })).map(|f| f.id).collect())
            .unwrap_or_default();
    }

    pub fn nudge(&mut self, dx: f32, dy: f32) {
        if self.d.sel.is_empty() {
            return;
        }
        self.checkpoint();
        let pg = self.pg();
        let sel = self.d.sel.clone();
        self.d.doc.translate(pg, &sel, dx, dy);
    }

    // ---- Pagine e vista ----

    pub fn add_page(&mut self) {
        self.checkpoint();
        self.d.doc.add_page();
        self.go_to_page(self.d.doc.pages.len() - 1);
    }

    pub fn delete_page(&mut self) {
        let i = self.d.page;
        if self.d.doc.pages.len() <= 1 {
            self.message = Some(("Non si può eliminare l'unica pagina".into(), true));
            return;
        }
        self.checkpoint();
        self.d.doc.remove_page(i);
        self.d.page = i.min(self.d.doc.pages.len() - 1);
        self.d.sel.clear();
    }

    pub fn go_to_page(&mut self, i: usize) {
        if i >= self.d.doc.pages.len() {
            return;
        }
        if self.d.page != i {
            self.d.sel.clear();
        }
        self.d.page = i;
        self.view.master = false;
        let y = self.page_top_px(i);
        self.scroll_target = Some(vec2(self.scroll_off.x, (y - 20.0).max(0.0)));
    }

    /// Cerca la prossima occorrenza, la seleziona e porta la vista su di essa.
    pub fn find_next(&mut self) {
        let hits = self.d.doc.find(&self.find.needle, self.find.match_case);
        if hits.is_empty() {
            self.find.note = "Nessun risultato".into();
            return;
        }
        let i = if self.find.last < hits.len() { self.find.last } else { 0 };
        let (page, id) = hits[i];
        self.find.last = i + 1;
        self.find.note = format!("Risultato {} di {}", i + 1, hits.len());
        self.go_to_page(page);
        self.d.sel = vec![id];
    }

    pub fn replace_all(&mut self) {
        if self.find.needle.is_empty() {
            return;
        }
        let before = self.d.doc.clone();
        let n = self.d.doc.replace_all(&self.find.needle, &self.find.with, self.find.match_case);
        if n > 0 {
            self.push_undo(before);
        }
        self.find.note = format!("Sostituzioni: {n}");
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        // Scorciatoie di cronologia: prima dei widget, cosi i campi di testo non le intercettano.
        let (undo, redo, find, repl, all) = ctx.input_mut(|i| {
            let redo = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)
                | i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
            let undo = i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
            let find = i.consume_key(egui::Modifiers::COMMAND, egui::Key::F);
            let repl = i.consume_key(egui::Modifiers::COMMAND, egui::Key::H);
            (undo, redo, find, repl, false)
        });
        let _ = all;
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        let (save, save_as) = ctx.input_mut(|i| {
            let sa = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S);
            (i.consume_key(egui::Modifiers::COMMAND, egui::Key::S), sa)
        });
        if save_as {
            if self.save(true) {
                self.request_exit(ctx);
            }
        } else if save {
            self.save(false);
        }
        if find || repl {
            self.find.open = true;
            self.find.focus = true;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Copy => self.copy(),
                egui::Event::Cut => self.cut(),
                egui::Event::Paste(text) => {
                    if !self.clipboard.is_empty() {
                        self.paste();
                    } else if !text.trim().is_empty() {
                        let rect = self.insert_rect(80.0, 30.0);
                        let pg = self.pg();
                        self.checkpoint();
                        let kind = FrameKind::Text { text, size_pt: 14.0, color: Rgb(0x20, 0x20, 0x20), style: TextStyle::default() };
                        if let Some(id) = self.d.doc.add_frame(pg, rect, kind) {
                            self.d.sel = vec![id];
                        }
                    }
                }
                _ => {}
            }
        }
        let (del, sel_all, esc, dx, dy) = ctx.input(|i| {
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let dx = (i.key_pressed(egui::Key::ArrowRight) as i32 - i.key_pressed(egui::Key::ArrowLeft) as i32) as f32 * step;
            let dy = (i.key_pressed(egui::Key::ArrowDown) as i32 - i.key_pressed(egui::Key::ArrowUp) as i32) as f32 * step;
            (
                i.key_pressed(egui::Key::Delete),
                i.modifiers.command && i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::Escape),
                dx,
                dy,
            )
        });
        if del {
            self.delete_selected();
        }
        if sel_all {
            self.select_all();
        }
        if esc {
            self.d.sel.clear();
            self.fmt_painter = None;
            self.tool = Tool::Select;
        }
        if dx != 0.0 || dy != 0.0 {
            self.nudge(dx, dy);
        }
    }

    // ---- Pannelli ----

    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("Proprietà").strong());
        let pg = self.pg();
        if self.d.sel.is_empty() {
            ui.label(RichText::new("Nessun elemento selezionato").color(look::text_dim()));
            return;
        }
        if self.d.sel.len() > 1 {
            ui.label(RichText::new(format!("{} elementi selezionati", self.d.sel.len())).color(look::text_dim()));
            ui.add_space(6.0);
            if ui.button("Elimina").clicked() {
                self.delete_selected();
            }
            return;
        }
        let id = self.d.sel[0];
        let mut delete = false;
        let before = self.d.doc.clone();
        if let Some(f) = self.d.doc.frame_mut(pg, id) {
            egui::Grid::new("geom").num_columns(2).show(ui, |ui| {
                for (label, v) in [("X (mm)", &mut f.rect.x), ("Y (mm)", &mut f.rect.y), ("Larg. (mm)", &mut f.rect.w), ("Alt. (mm)", &mut f.rect.h)] {
                    ui.label(label);
                    ui.add(egui::DragValue::new(v).speed(0.5).fixed_decimals(1));
                    ui.end_row();
                }
                ui.label("Rotazione");
                let mut deg = f.rot as i32 * 90;
                if ui.add(egui::DragValue::new(&mut deg).range(0..=270).speed(1.0).suffix("°")).changed() {
                    f.rot = ((deg / 90).rem_euclid(4)) as u8;
                }
                ui.end_row();
            });
            f.rect.w = f.rect.w.max(MIN_FRAME_MM);
            f.rect.h = f.rect.h.max(MIN_FRAME_MM);
            ui.separator();
            match &mut f.kind {
                FrameKind::Text { text, size_pt, color, .. } => {
                    ui.add(egui::TextEdit::multiline(text).desired_rows(6).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        ui.label("Corpo (pt)");
                        ui.add(egui::DragValue::new(size_pt).range(4.0..=400.0).speed(0.25));
                    });
                    color_row(ui, "Colore", color);
                    ui.label(RichText::new("Grassetto, rientri, elenchi e allineamento: scheda Home.").small().color(look::text_dim()));
                }
                FrameKind::Rectangle { fill, stroke, stroke_mm } | FrameKind::Ellipse { fill, stroke, stroke_mm } | FrameKind::Triangle { fill, stroke, stroke_mm } => {
                    color_row(ui, "Riempimento", fill);
                    color_row(ui, "Contorno", stroke);
                    ui.horizontal(|ui| {
                        ui.label("Spessore (mm)");
                        ui.add(egui::DragValue::new(stroke_mm).range(0.0..=20.0).speed(0.05));
                    });
                }
                FrameKind::Line { stroke, stroke_mm } => {
                    color_row(ui, "Colore", stroke);
                    ui.horizontal(|ui| {
                        ui.label("Spessore (mm)");
                        ui.add(egui::DragValue::new(stroke_mm).range(0.1..=20.0).speed(0.05));
                    });
                }
                FrameKind::Picture { path } => {
                    ui.label(RichText::new(path.as_str()).small().color(look::text_dim()));
                }
                FrameKind::Table { rows, cols, cells, border, border_mm, size_pt } => {
                    let (mut r, mut c) = (*rows, *cols);
                    ui.horizontal(|ui| {
                        ui.label("Righe");
                        ui.add(egui::DragValue::new(&mut r).range(1..=30));
                        ui.label("Colonne");
                        ui.add(egui::DragValue::new(&mut c).range(1..=12));
                    });
                    if (r, c) != (*rows, *cols) {
                        let mut n = vec![String::new(); r * c];
                        for y in 0..r.min(*rows) {
                            for x in 0..c.min(*cols) {
                                n[y * c + x] = std::mem::take(&mut cells[y * *cols + x]);
                            }
                        }
                        *rows = r;
                        *cols = c;
                        *cells = n;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Corpo (pt)");
                        ui.add(egui::DragValue::new(size_pt).range(4.0..=72.0).speed(0.25));
                    });
                    color_row(ui, "Bordo", border);
                    ui.horizontal(|ui| {
                        ui.label("Spessore (mm)");
                        ui.add(egui::DragValue::new(border_mm).range(0.0..=5.0).speed(0.05));
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        egui::Grid::new("cells").show(ui, |ui| {
                            for y in 0..*rows {
                                for x in 0..*cols {
                                    ui.add(egui::TextEdit::singleline(&mut cells[y * *cols + x]).desired_width(60.0));
                                }
                                ui.end_row();
                            }
                        });
                    });
                }
            }
            ui.separator();
            delete = ui.button("Elimina").clicked();
        }
        if delete {
            self.push_undo(before);
            self.d.doc.remove_frame(pg, id);
            self.d.sel.clear();
        } else if self.d.doc != before {
            // Raggruppa le modifiche ravvicinate allo stesso elemento in un solo passo.
            let now = Instant::now();
            let merge = matches!(self.d.props_last, Some((i, t)) if i == id && now.duration_since(t) < COALESCE);
            if !merge {
                self.push_undo(before);
            }
            self.d.props_last = Some((id, now));
        }
    }

    fn pages_pane(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Pagine").size(18.0).color(craft_theme::ACCENT_PUBLICRAFT));
        });
        ui.add_space(4.0);
        let mut goto = None;
        egui::ScrollArea::vertical().auto_shrink(false).max_height(ui.available_height() - 34.0).show(ui, |ui| {
            for i in 0..self.d.doc.pages.len() {
                if let Some(true) = self.thumbnail(ui, i) {
                    goto = Some(i);
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("＋ Pagina").clicked() {
                self.add_page();
            }
            if ui.add_enabled(self.d.doc.pages.len() > 1, egui::Button::new("Elimina")).clicked() {
                self.delete_page();
            }
        });
        if let Some(i) = goto {
            self.go_to_page(i);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Pagina: {} di {}", self.d.page + 1, self.d.doc.pages.len()));
            ui.separator();
            if let Some((_, x, y)) = self.cursor {
                ui.label(format!("{:.2}, {:.2} cm", x / 10.0, y / 10.0));
            } else {
                ui.label(RichText::new("—").color(look::text_dim()));
            }
            if self.d.sel.len() == 1 {
                if let Some(f) = self.d.doc.frame(self.pg(), self.d.sel[0]) {
                    let v = f.visual_rect();
                    ui.separator();
                    ui.label(format!("{:.2}, {:.2} cm", v.x / 10.0, v.y / 10.0));
                    ui.label(format!("{:.2} × {:.2} cm", v.w / 10.0, v.h / 10.0));
                }
            } else if self.d.sel.len() > 1 {
                ui.separator();
                ui.label(format!("{} elementi", self.d.sel.len()));
            }
            if let Some((m, err)) = &self.message {
                ui.separator();
                let c = if *err { Color32::from_rgb(0xe0, 0x5a, 0x47) } else { look::text_dim() };
                ui.label(RichText::new(m).color(c));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⛶").on_hover_text("Adatta la pagina alla finestra").clicked() {
                    self.fit = Some(Fit::Page);
                }
                if ui.button("＋").on_hover_text("Ingrandisci").clicked() {
                    self.set_zoom(self.d.zoom * 1.1);
                }
                let mut pct = self.d.zoom * 100.0;
                ui.spacing_mut().slider_width = 140.0;
                if ui.add(egui::Slider::new(&mut pct, 10.0..=400.0).logarithmic(true).show_value(false)).changed() {
                    self.set_zoom(pct / 100.0);
                }
                if ui.button("－").on_hover_text("Riduci").clicked() {
                    self.set_zoom(self.d.zoom / 1.1);
                }
                ui.label(format!("{:.0}%", self.d.zoom * 100.0));
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.confirm_exit {
            let mut action = 0; // 1 salva e chiudi, 2 chiudi senza salvare, 3 annulla
            let n = self.others.iter().filter(|o| o.dirty()).count() + self.d.dirty() as usize;
            #[allow(deprecated)]
            let confirm_id = egui::Id::new("confirm-exit");
            egui::Modal::new(confirm_id).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label(RichText::new("Modifiche non salvate").size(18.0).strong().color(craft_theme::ACCENT_PUBLICRAFT));
                ui.add_space(6.0);
                let msg = if n > 1 { format!("{n} documenti hanno modifiche non salvate. Vuoi salvarle prima di chiudere?") } else { "Il documento ha modifiche non salvate. Vuoi salvarle prima di chiudere?".to_string() };
                ui.label(msg);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let save = egui::Button::new(RichText::new("Salva").strong().color(Color32::WHITE)).fill(craft_theme::ACCENT_PUBLICRAFT).min_size(vec2(90.0, 28.0));
                    if ui.add(save).clicked() {
                        action = 1;
                    }
                    if ui.add(egui::Button::new("Non salvare").min_size(vec2(100.0, 28.0))).clicked() {
                        action = 2;
                    }
                    if ui.add(egui::Button::new("Annulla").min_size(vec2(80.0, 28.0))).clicked() {
                        action = 3;
                    }
                });
            });
            match action {
                1 => {
                    if self.save_all() {
                        self.closing_ok = true;
                        self.confirm_exit = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                2 => {
                    self.closing_ok = true;
                    self.confirm_exit = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                3 => self.confirm_exit = false,
                _ => {}
            }
        }

        if self.find.open {
            let mut open = true;
            egui::Window::new("Trova e sostituisci").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                egui::Grid::new("find").num_columns(2).show(ui, |ui| {
                    ui.label("Trova");
                    let r = ui.add(egui::TextEdit::singleline(&mut self.find.needle).desired_width(220.0));
                    if self.find.focus {
                        r.request_focus();
                        self.find.focus = false;
                    }
                    if r.changed() {
                        self.find.last = 0;
                        self.find.note.clear();
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.find_next();
                    }
                    ui.end_row();
                    ui.label("Sostituisci con");
                    ui.add(egui::TextEdit::singleline(&mut self.find.with).desired_width(220.0));
                    ui.end_row();
                });
                ui.checkbox(&mut self.find.match_case, "Maiuscole/minuscole");
                ui.horizontal(|ui| {
                    if ui.button("Trova successivo").clicked() {
                        self.find_next();
                    }
                    if ui.button("Sostituisci tutto").clicked() {
                        self.replace_all();
                    }
                });
                if !self.find.note.is_empty() {
                    ui.label(RichText::new(&self.find.note).color(look::text_dim()));
                }
            });
            self.find.open = open;
        }

        if self.view.graphics {
            let mut open = true;
            let mut goto = None;
            egui::Window::new("Gestione immagini").open(&mut open).default_width(380.0).show(ctx, |ui| {
                let mut any = false;
                for (pi, p) in self.d.doc.pages.iter().enumerate() {
                    for f in &p.frames {
                        if let FrameKind::Picture { path } = &f.kind {
                            any = true;
                            let dims = self.textures.get(path).and_then(|t| t.as_ref()).map(|t| format!("{}×{} px", t.size[0], t.size[1])).unwrap_or_else(|| "non trovata".into());
                            ui.horizontal(|ui| {
                                if ui.button("Vai").clicked() {
                                    goto = Some((pi, f.id));
                                }
                                let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                ui.label(format!("Pagina {} · {} · {}", pi + 1, name, dims)).on_hover_text(path);
                            });
                        }
                    }
                }
                if !any {
                    ui.label(RichText::new("Nessuna immagine nel documento").color(look::text_dim()));
                }
            });
            self.view.graphics = open;
            if let Some((pi, id)) = goto {
                self.go_to_page(pi);
                self.d.sel = vec![id];
            }
        }
    }
}

pub(crate) fn color_row(ui: &mut egui::Ui, label: &str, c: &mut Rgb) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut a = [c.0, c.1, c.2];
        if ui.color_edit_button_srgb(&mut a).changed() {
            *c = Rgb(a[0], a[1], a[2]);
        }
    });
}

impl eframe::App for PubliCraft {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keys(&ctx);
        self.ensure_textures(&ctx);

        let title = format!("{}{} — PubliCraft", if self.d.dirty() { "● " } else { "" }, self.d.title());
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.closing_ok && self.any_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_exit = true;
        }

        self.poll_jobs(&ctx);
        self.bs_alpha = ctx.animate_bool_with_time(ui.scope_id().with("backstage"), self.start.open, 0.18);
        if self.start.open {
            self.start_screen(ui);
            self.dialogs(&ctx);
            return;
        }

        egui::Panel::top("ribbon").frame(egui::Frame::new().fill(look::ribbon())).show(ui, |ui| self.ribbon(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if self.view.nav {
            egui::Panel::left("pages").resizable(false).exact_size(150.0).show(ui, |ui| self.pages_pane(ui));
        }
        egui::Panel::right("props").default_size(240.0).show(ui, |ui| self.properties(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(look::work())).show(ui, |ui| self.canvas(ui));
        self.dialogs(&ctx);
    }
}

fn main() -> eframe::Result {
    // Registra i file .pubcraft per questo utente (anche da riga di comando: `publicraft --register`).
    assoc::register();
    // Esportazione da riga di comando: `publicraft --export documento.pubcraft uscita.pdf [dpi]`.
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--export") {
        let (Some(inp), Some(out)) = (args.get(i + 1), args.get(i + 2)) else {
            eprintln!("uso: publicraft --export <documento.pubcraft> <uscita.pdf|png> [dpi]");
            std::process::exit(2);
        };
        let dpi: f32 = args.get(i + 3).and_then(|d| d.parse().ok()).unwrap_or(150.0);
        let doc = if inp.to_lowercase().ends_with(".pdf") {
            pdfimport::import_pdf(std::path::Path::new(inp)).map(|(d, m)| {
                eprintln!("{m}");
                d
            })
        } else if let Some(n) = inp.strip_prefix("template:") {
            Ok(start::build_template(n.parse().unwrap_or(0)))
        } else {
            std::fs::read_to_string(inp).map_err(|e| e.to_string()).and_then(|t| Document::from_json(t.trim_start_matches('\u{feff}')).map_err(|e| e.to_string()))
        };
        let doc = doc.unwrap_or_else(|e| {
            eprintln!("impossibile aprire {inp}: {e}");
            std::process::exit(1);
        });
        let all: Vec<usize> = (0..doc.pages.len()).collect();
        let imgs = export::render_pages(&doc, &all, dpi);
        let path = std::path::Path::new(out);
        let r = if out.to_lowercase().ends_with(".png") { export::write_png(path, &imgs[0]) } else { export::write_pdf(path, "PubliCraft", &imgs) };
        if let Err(e) = r {
            eprintln!("errore: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--register") {
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("PubliCraft")
        .with_inner_size([1500.0, 860.0])
        .with_min_inner_size([900.0, 560.0]);
    if let Ok(img) = image::load_from_memory(ICON_PNG) {
        let img = img.to_rgba8();
        let (width, height) = img.dimensions();
        viewport = viewport.with_icon(egui::IconData { rgba: img.into_raw(), width, height });
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("PubliCraft", options, Box::new(|cc| Ok(Box::new(PubliCraft::new(cc)))))
}
