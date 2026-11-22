//! Pagine «Esporta» e «Stampa» della schermata File, con i lavori in secondo piano.

use super::*;
use crate::export;
use crate::print::{self, Duplex, Orient};
use egui::{Sense, Stroke};
use std::sync::mpsc::{Receiver, TryRecvError};

const DPIS: [(f32, &str); 3] = [(96.0, "Bassa · 96 dpi (file leggero)"), (150.0, "Standard · 150 dpi"), (300.0, "Alta · 300 dpi")];
const PRINT_DPI: f32 = 300.0;

pub(crate) struct OutState {
    pub fmt: usize,
    pub dpi: usize,
    pub range: usize,
    pub custom: String,
    pub print: print::Settings,
    pub printers: Vec<String>,
    pub loaded: bool,
    pub papers: Vec<(i16, String)>,
    pub papers_for: String,
    pub preview: usize,
    pub busy: Option<Receiver<Result<String, String>>>,
    pub busy_what: &'static str,
    pub import: Option<Receiver<Result<(Document, String), String>>>,
}

impl OutState {
    pub fn new() -> Self {
        Self {
            fmt: 0,
            dpi: 1,
            range: 0,
            custom: String::new(),
            print: print::Settings { printer: String::new(), copies: 1, color: true, orient: Orient::Auto, duplex: Duplex::Off, paper: 0, fit: true },
            printers: Vec::new(),
            loaded: false,
            papers: Vec::new(),
            papers_for: String::new(),
            preview: 0,
            busy: None,
            busy_what: "",
            import: None,
        }
    }
}

fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, value: &mut T, options: &[(T, &str)]) {
    let cur = options.iter().find(|(v, _)| v == value).map(|(_, l)| *l).unwrap_or("");
    egui::ComboBox::from_id_salt(id).width(300.0).selected_text(cur).show_ui(ui, |ui| {
        for (v, l) in options {
            ui.selectable_value(value, *v, *l);
        }
    });
}

fn heading(ui: &mut egui::Ui, t: &str) {
    ui.add_space(12.0);
    ui.label(egui::RichText::new(t).size(13.0).color(look::text_dim()));
    ui.add_space(2.0);
}

impl PubliCraft {
    /// Controlla se un lavoro in secondo piano è finito.
    /// Sceglie un PDF e lo importa come documento modificabile (in secondo piano).
    pub(crate) fn import_pdf_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Documento PDF", &["pdf"]).pick_file() else { return };
        let (tx, rx) = std::sync::mpsc::channel();
        self.out.import = Some(rx);
        self.message = Some((format!("Importazione di {} in corso…", path.display()), false));
        std::thread::spawn(move || {
            let _ = tx.send(crate::pdfimport::import_pdf(&path));
        });
    }

    pub(crate) fn add_imported(&mut self, doc: Document, msg: String) {
        let mut st = DocState::new(self.next_num);
        self.next_num += 1;
        st.saved = Document::new(doc.page_size);
        st.doc = doc;
        self.push_tab(st);
        if !self.started {
            // Il documento vuoto di partenza non serve più.
            self.others.clear();
            self.started = true;
        }
        self.start.open = false;
        self.message = Some((msg, false));
    }

    pub(crate) fn poll_jobs(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.out.import {
            match rx.try_recv() {
                Ok(r) => {
                    self.out.import = None;
                    match r {
                        Ok((doc, msg)) => self.add_imported(doc, msg),
                        Err(e) => self.message = Some((format!("Importazione non riuscita: {e}"), true)),
                    }
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                Err(TryRecvError::Disconnected) => self.out.import = None,
            }
        }
        let Some(rx) = &self.out.busy else { return };
        match rx.try_recv() {
            Ok(r) => {
                self.out.busy = None;
                self.message = Some(match r {
                    Ok(m) => (m, false),
                    Err(e) => (format!("{} non riuscita: {e}", self.out.busy_what), true),
                });
            }
            Err(TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            Err(TryRecvError::Disconnected) => self.out.busy = None,
        }
    }

    fn pick_pages(&self) -> Result<Vec<usize>, String> {
        let n = self.d.doc.pages.len();
        match self.out.range {
            0 => Ok((0..n).collect()),
            1 => Ok(vec![self.d.page.min(n.saturating_sub(1))]),
            _ => export::parse_range(&self.out.custom, n).ok_or_else(|| format!("intervallo non valido: usa ad esempio 1-3, 5 (il documento ha {n} pagine)")),
        }
    }

    fn range_ui(&mut self, ui: &mut egui::Ui) {
        let mut r = self.out.range;
        combo(ui, "out-range", &mut r, &[(0, "Tutte le pagine"), (1, "Pagina corrente"), (2, "Intervallo di pagine")]);
        self.out.range = r;
        if r == 2 {
            ui.add_space(4.0);
            ui.add(egui::TextEdit::singleline(&mut self.out.custom).hint_text("Ad esempio 1-3, 5").desired_width(300.0));
        }
    }

    fn start_export(&mut self) {
        let pages = match self.pick_pages() {
            Ok(p) => p,
            Err(e) => {
                self.message = Some((e, true));
                return;
            }
        };
        let png = self.out.fmt == 1;
        let stem = self.d.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| self.d.title());
        let dialog = if png { rfd::FileDialog::new().add_filter("Immagine PNG", &["png"]) } else { rfd::FileDialog::new().add_filter("Documento PDF", &["pdf"]) };
        let Some(path) = dialog.set_file_name(format!("{stem}.{}", if png { "png" } else { "pdf" })).save_file() else { return };
        let doc = self.d.doc.clone();
        let dpi = DPIS[self.out.dpi].0;
        let (tx, rx) = std::sync::mpsc::channel();
        self.out.busy = Some(rx);
        self.out.busy_what = "Esportazione";
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let imgs = export::render_pages(&doc, &pages, dpi);
                if png {
                    if imgs.len() == 1 {
                        export::write_png(&path, &imgs[0])?;
                    } else {
                        for (img, idx) in imgs.iter().zip(&pages) {
                            let name = format!("{}-{}.png", path.with_extension("").display(), idx + 1);
                            export::write_png(std::path::Path::new(&name), img)?;
                        }
                    }
                } else {
                    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    export::write_pdf(&path, &title, &imgs)?;
                }
                Ok(format!("Esportato in {}", path.display()))
            })();
            let _ = tx.send(result);
        });
    }

    fn start_print(&mut self) {
        let pages = match self.pick_pages() {
            Ok(p) => p,
            Err(e) => {
                self.message = Some((e, true));
                return;
            }
        };
        if self.out.print.printer.is_empty() {
            self.message = Some(("Nessuna stampante selezionata".into(), true));
            return;
        }
        let doc = self.d.doc.clone();
        let settings = self.out.print.clone();
        let name = self.d.title();
        let (tx, rx) = std::sync::mpsc::channel();
        self.out.busy = Some(rx);
        self.out.busy_what = "Stampa";
        std::thread::spawn(move || {
            let mut imgs = export::render_pages(&doc, &pages, PRINT_DPI);
            if !settings.color {
                imgs.iter_mut().for_each(|i| i.to_gray());
            }
            let n = imgs.len();
            let r = print::print(&name, &settings, &imgs).map(|_| format!("Inviate {n} pagine a «{}»", settings.printer));
            let _ = tx.send(r);
        });
    }

    pub(crate) fn export_page(&mut self, ui: &mut egui::Ui) {
        let accent = craft_theme::ACCENT_PUBLICRAFT;
        ui.label(egui::RichText::new("Esporta").size(30.0));
        ui.add_space(14.0);
        ui.label("Crea un file da condividere o da stampare altrove. Il PDF mantiene le dimensioni reali delle pagine.");
        heading(ui, "Formato");
        ui.horizontal(|ui| {
            for (i, l) in ["PDF", "Immagine PNG"].iter().enumerate() {
                if ui.add(egui::Button::new(*l).selected(self.out.fmt == i).min_size(vec2(130.0, 30.0))).clicked() {
                    self.out.fmt = i;
                }
            }
        });
        heading(ui, "Qualità");
        let mut dpi = self.out.dpi;
        egui::ComboBox::from_id_salt("exp-dpi").width(300.0).selected_text(DPIS[dpi].1).show_ui(ui, |ui| {
            for (i, (_, l)) in DPIS.iter().enumerate() {
                ui.selectable_value(&mut dpi, i, *l);
            }
        });
        self.out.dpi = dpi;
        heading(ui, "Pagine");
        self.range_ui(ui);
        ui.add_space(20.0);
        let busy = self.out.busy.is_some();
        ui.horizontal(|ui| {
            let btn = egui::Button::new(egui::RichText::new("Esporta…").size(15.0).color(egui::Color32::WHITE)).fill(accent).min_size(vec2(160.0, 34.0));
            if ui.add_enabled(!busy, btn).clicked() {
                self.start_export();
            }
            if busy {
                ui.spinner();
                ui.label(format!("{} in corso…", self.out.busy_what));
            }
        });
    }

    pub(crate) fn print_page(&mut self, ui: &mut egui::Ui) {
        let accent = craft_theme::ACCENT_PUBLICRAFT;
        if !self.out.loaded {
            self.out.loaded = true;
            self.out.printers = print::list_printers();
            self.out.print.printer = print::default_printer().filter(|d| self.out.printers.contains(d)).or_else(|| self.out.printers.first().cloned()).unwrap_or_default();
        }
        if self.out.papers_for != self.out.print.printer {
            self.out.papers_for = self.out.print.printer.clone();
            self.out.papers = if self.out.print.printer.is_empty() { Vec::new() } else { print::paper_sizes(&self.out.print.printer) };
            self.out.print.paper = 0;
        }
        ui.label(egui::RichText::new("Stampa").size(30.0));
        ui.add_space(12.0);
        let n_pages = self.d.doc.pages.len();
        let busy = self.out.busy.is_some();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(380.0, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(egui::RichText::new("🖨  Stampa").size(16.0).color(egui::Color32::WHITE)).fill(accent).min_size(vec2(130.0, 52.0));
                        if ui.add_enabled(!busy && !self.out.printers.is_empty(), btn).clicked() {
                            self.start_print();
                        }
                        ui.vertical(|ui| {
                            ui.label("Copie");
                            let mut c = self.out.print.copies as u32;
                            ui.add(egui::DragValue::new(&mut c).range(1..=99).speed(0.2));
                            self.out.print.copies = c as u16;
                        });
                        if busy {
                            ui.spinner();
                        }
                    });
                    heading(ui, "Stampante");
                    if self.out.printers.is_empty() {
                        ui.label("Nessuna stampante installata.");
                    } else {
                        let cur = self.out.print.printer.clone();
                        egui::ComboBox::from_id_salt("prn").width(300.0).selected_text(cur.clone()).show_ui(ui, |ui| {
                            for p in &self.out.printers {
                                ui.selectable_value(&mut self.out.print.printer, p.clone(), p);
                            }
                        });
                    }
                    heading(ui, "Pagine");
                    self.range_ui(ui);
                    heading(ui, "Fronte-retro");
                    combo(ui, "prn-duplex", &mut self.out.print.duplex, &[(Duplex::Off, "Solo fronte"), (Duplex::LongEdge, "Fronte-retro (lato lungo)"), (Duplex::ShortEdge, "Fronte-retro (lato corto)")]);
                    heading(ui, "Orientamento");
                    combo(ui, "prn-orient", &mut self.out.print.orient, &[(Orient::Auto, "Come la pagina"), (Orient::Portrait, "Verticale"), (Orient::Landscape, "Orizzontale")]);
                    heading(ui, "Carta");
                    let cur = self.out.papers.iter().find(|(c, _)| *c == self.out.print.paper).map(|(_, n)| n.clone()).unwrap_or_else(|| "Predefinita della stampante".into());
                    egui::ComboBox::from_id_salt("prn-paper").width(300.0).selected_text(cur).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.out.print.paper, 0, "Predefinita della stampante");
                        for (c, n) in &self.out.papers {
                            ui.selectable_value(&mut self.out.print.paper, *c, n);
                        }
                    });
                    heading(ui, "Colore");
                    combo(ui, "prn-color", &mut self.out.print.color, &[(true, "A colori"), (false, "Scala di grigi")]);
                    heading(ui, "Ridimensiona");
                    combo(ui, "prn-fit", &mut self.out.print.fit, &[(true, "Adatta alla pagina"), (false, "Dimensione reale (100%)")]);
                });
            });
            ui.add_space(24.0);
            self.print_preview(ui, n_pages);
        });
    }

    fn print_preview(&mut self, ui: &mut egui::Ui, n_pages: usize) {
        ui.vertical(|ui| {
            let idx = self.out.preview.min(n_pages.saturating_sub(1));
            let avail = ui.available_size();
            let s = self.d.doc.page_size;
            let room = vec2(avail.x.max(100.0), (avail.y - 44.0).max(100.0));
            let k = (room.x / s.w_mm).min(room.y / s.h_mm).min(2.5);
            let size = vec2(s.w_mm, s.h_mm) * k;
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let painter = ui.painter().with_clip_rect(rect.expand(6.0));
            painter.rect_filled(rect.translate(vec2(3.0, 3.0)), egui::CornerRadius::ZERO, egui::Color32::from_black_alpha(50));
            canvas::paint_page_to(&painter, &self.d.doc, idx, rect.min, k, &self.textures);
            painter.rect_stroke(rect, egui::CornerRadius::ZERO, Stroke::new(1.0, look::border()), egui::StrokeKind::Outside);
            ui.horizontal(|ui| {
                if ui.add_enabled(idx > 0, egui::Button::new("◀")).clicked() {
                    self.out.preview = idx - 1;
                }
                ui.label(format!("{} di {}", idx + 1, n_pages));
                if ui.add_enabled(idx + 1 < n_pages, egui::Button::new("▶")).clicked() {
                    self.out.preview = idx + 1;
                }
            });
        });
    }
}
