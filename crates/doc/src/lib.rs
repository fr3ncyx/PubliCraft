//! Modello documento di PubliCraft. Nessuna dipendenza da egui o dal windowing.
//! Le misure sono in millimetri; l'origine è l'angolo in alto a sinistra della pagina.

use serde::{Deserialize, Serialize};

/// Indice speciale di pagina: la pagina master (sfondo comune a tutte le pagine).
pub const MASTER: usize = usize::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn union(&self, o: &Rect) -> Rect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect { x, y, w: self.right().max(o.right()) - x, h: self.bottom().max(o.bottom()) - y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Family {
    #[default]
    Sans,
    Mono,
    Arial,
    Times,
    Calibri,
    Georgia,
    Verdana,
    Tahoma,
    Consolas,
    Courier,
}

impl Family {
    pub const ALL: [Family; 10] = [
        Family::Sans,
        Family::Arial,
        Family::Calibri,
        Family::Georgia,
        Family::Times,
        Family::Verdana,
        Family::Tahoma,
        Family::Mono,
        Family::Consolas,
        Family::Courier,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Family::Sans => "Segoe UI",
            Family::Mono => "Monospazio",
            Family::Arial => "Arial",
            Family::Times => "Times New Roman",
            Family::Calibri => "Calibri",
            Family::Georgia => "Georgia",
            Family::Verdana => "Verdana",
            Family::Tahoma => "Tahoma",
            Family::Consolas => "Consolas",
            Family::Courier => "Courier New",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ListKind {
    #[default]
    None,
    Bullet,
    Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub align: Align,
    pub family: Family,
    pub sub: bool,
    pub sup: bool,
    pub list: ListKind,
    /// Livelli di rientro (ognuno 6 mm).
    pub indent: u8,
    /// Interlinea (1.0 = singola).
    pub line_spacing: f32,
    /// Spaziatura tra i caratteri, in punti (negativa = più stretta).
    pub tracking_pt: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            align: Align::Left,
            family: Family::Sans,
            sub: false,
            sup: false,
            list: ListKind::None,
            indent: 0,
            line_spacing: 1.0,
            tracking_pt: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseKind {
    Sentence,
    Lower,
    Upper,
    Title,
    Toggle,
}

pub fn change_case(s: &str, kind: CaseKind) -> String {
    match kind {
        CaseKind::Lower => s.to_lowercase(),
        CaseKind::Upper => s.to_uppercase(),
        CaseKind::Toggle => s
            .chars()
            .map(|c| if c.is_lowercase() { c.to_uppercase().next().unwrap_or(c) } else { c.to_lowercase().next().unwrap_or(c) })
            .collect(),
        CaseKind::Title => {
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if c.is_alphanumeric() {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    start = !(c == '\'' || c == '’');
                }
            }
            out
        }
        CaseKind::Sentence => {
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if c.is_alphanumeric() {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    if matches!(c, '.' | '!' | '?' | '\n') {
                        start = true;
                    }
                }
            }
            out
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Wrap {
    #[default]
    None,
    Square,
    Tight,
    TopBottom,
}

impl Wrap {
    pub const ALL: [Wrap; 4] = [Wrap::None, Wrap::Square, Wrap::Tight, Wrap::TopBottom];

    pub fn label(self) -> &'static str {
        match self {
            Wrap::None => "Nessuno",
            Wrap::Square => "Quadrato",
            Wrap::Tight => "Aderente",
            Wrap::TopBottom => "Sopra e sotto",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FrameKind {
    Text {
        text: String,
        size_pt: f32,
        color: Rgb,
        #[serde(default)]
        style: TextStyle,
    },
    Rectangle { fill: Rgb, stroke: Rgb, stroke_mm: f32 },
    Ellipse { fill: Rgb, stroke: Rgb, stroke_mm: f32 },
    Triangle { fill: Rgb, stroke: Rgb, stroke_mm: f32 },
    Line { stroke: Rgb, stroke_mm: f32 },
    /// Immagine collegata dal percorso del file.
    Picture { path: String },
    Table {
        rows: usize,
        cols: usize,
        /// Celle per riga, dalla prima all'ultima.
        cells: Vec<String>,
        border: Rgb,
        border_mm: f32,
        size_pt: f32,
    },
}

impl FrameKind {
    pub fn table(rows: usize, cols: usize) -> FrameKind {
        FrameKind::Table {
            rows: rows.max(1),
            cols: cols.max(1),
            cells: vec![String::new(); rows.max(1) * cols.max(1)],
            border: Rgb(0x40, 0x40, 0x40),
            border_mm: 0.3,
            size_pt: 11.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: u64,
    pub rect: Rect,
    pub kind: FrameKind,
    /// Rotazione in quarti di giro orari (0..=3).
    #[serde(default)]
    pub rot: u8,
    /// 0 = nessun gruppo.
    #[serde(default)]
    pub group: u64,
    #[serde(default)]
    pub wrap: Wrap,
}

impl Frame {
    /// Rettangolo occupato sulla pagina, tenendo conto della rotazione a quarti di giro.
    pub fn visual_rect(&self) -> Rect {
        if self.rot % 2 == 0 {
            self.rect
        } else {
            let cx = self.rect.x + self.rect.w / 2.0;
            let cy = self.rect.y + self.rect.h / 2.0;
            Rect { x: cx - self.rect.h / 2.0, y: cy - self.rect.w / 2.0, w: self.rect.h, h: self.rect.w }
        }
    }

    /// Imposta il rettangolo visibile, ricavando quello non ruotato.
    pub fn set_visual_rect(&mut self, v: Rect) {
        if self.rot % 2 == 0 {
            self.rect = v;
        } else {
            let cx = v.x + v.w / 2.0;
            let cy = v.y + v.h / 2.0;
            self.rect = Rect { x: cx - v.h / 2.0, y: cy - v.w / 2.0, w: v.h, h: v.w };
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    pub w_mm: f32,
    pub h_mm: f32,
}

impl PageSize {
    pub const A4: Self = Self { w_mm: 210.0, h_mm: 297.0 };
    pub const A5: Self = Self { w_mm: 148.0, h_mm: 210.0 };
    pub const LETTER: Self = Self { w_mm: 215.9, h_mm: 279.4 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignKind {
    Left,
    CenterH,
    Right,
    Top,
    CenterV,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub page_size: PageSize,
    pub pages: Vec<Page>,
    /// Pagina master: gli elementi compaiono dietro a tutte le pagine.
    #[serde(default)]
    pub master: Page,
    next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(PageSize::A4)
    }
}

impl Document {
    pub fn new(page_size: PageSize) -> Self {
        Self { page_size, pages: vec![Page::default()], master: Page::default(), next_id: 1 }
    }

    pub fn page(&self, idx: usize) -> Option<&Page> {
        if idx == MASTER { Some(&self.master) } else { self.pages.get(idx) }
    }

    pub fn page_mut(&mut self, idx: usize) -> Option<&mut Page> {
        if idx == MASTER { Some(&mut self.master) } else { self.pages.get_mut(idx) }
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Aggiunge un riquadro alla pagina e ne restituisce l'id.
    pub fn add_frame(&mut self, page: usize, rect: Rect, kind: FrameKind) -> Option<u64> {
        self.page(page)?;
        let id = self.take_id();
        self.page_mut(page)?.frames.push(Frame { id, rect, kind, rot: 0, group: 0, wrap: Wrap::None });
        Some(id)
    }

    pub fn frame(&self, page: usize, id: u64) -> Option<&Frame> {
        self.page(page)?.frames.iter().find(|f| f.id == id)
    }

    pub fn frame_mut(&mut self, page: usize, id: u64) -> Option<&mut Frame> {
        self.page_mut(page)?.frames.iter_mut().find(|f| f.id == id)
    }

    pub fn remove_frame(&mut self, page: usize, id: u64) -> bool {
        match self.page_mut(page) {
            Some(p) => {
                let before = p.frames.len();
                p.frames.retain(|f| f.id != id);
                p.frames.len() != before
            }
            None => false,
        }
    }

    pub fn remove_frames(&mut self, page: usize, ids: &[u64]) {
        if let Some(p) = self.page_mut(page) {
            p.frames.retain(|f| !ids.contains(&f.id));
        }
    }

    /// Riquadro più in alto sotto il punto (x, y), in mm.
    pub fn hit_test(&self, page: usize, x: f32, y: f32) -> Option<u64> {
        self.page(page)?.frames.iter().rev().find(|f| f.visual_rect().contains(x, y)).map(|f| f.id)
    }

    /// L'id con tutti i membri del suo gruppo (se ne ha uno).
    pub fn with_group(&self, page: usize, id: u64) -> Vec<u64> {
        let Some(p) = self.page(page) else { return vec![id] };
        match p.frames.iter().find(|f| f.id == id) {
            Some(f) if f.group != 0 => p.frames.iter().filter(|g| g.group == f.group).map(|g| g.id).collect(),
            _ => vec![id],
        }
    }

    /// Elementi che toccano il rettangolo (per la selezione ad area).
    pub fn in_rect(&self, page: usize, r: &Rect) -> Vec<u64> {
        let mut out: Vec<u64> = Vec::new();
        if let Some(p) = self.page(page) {
            for f in &p.frames {
                if f.visual_rect().intersects(r) {
                    for id in self.with_group(page, f.id) {
                        if !out.contains(&id) {
                            out.push(id);
                        }
                    }
                }
            }
        }
        out
    }

    /// Rettangolo che racchiude tutti gli elementi indicati.
    pub fn bounds(&self, page: usize, ids: &[u64]) -> Option<Rect> {
        let p = self.page(page)?;
        p.frames.iter().filter(|f| ids.contains(&f.id)).map(|f| f.visual_rect()).reduce(|a, b| a.union(&b))
    }

    pub fn insert_page(&mut self, at: usize) -> usize {
        let at = at.min(self.pages.len());
        self.pages.insert(at, Page::default());
        at
    }

    /// Duplica la pagina subito dopo l'originale; restituisce l'indice della copia.
    pub fn duplicate_page(&mut self, idx: usize) -> Option<usize> {
        let frames = self.pages.get(idx)?.frames.clone();
        let at = self.insert_page(idx + 1);
        self.paste_frames(at, &frames, 0.0, 0.0);
        Some(at)
    }

    pub fn add_page(&mut self) {
        self.pages.push(Page::default());
    }

    /// Elimina la pagina; l'ultima rimasta non si può eliminare.
    pub fn remove_page(&mut self, idx: usize) -> bool {
        if self.pages.len() <= 1 || idx >= self.pages.len() {
            return false;
        }
        self.pages.remove(idx);
        true
    }

    // ---- Disposizione ----

    pub fn bring_forward(&mut self, page: usize, ids: &[u64]) {
        let Some(p) = self.page_mut(page) else { return };
        let n = p.frames.len();
        for i in (0..n.saturating_sub(1)).rev() {
            if ids.contains(&p.frames[i].id) && !ids.contains(&p.frames[i + 1].id) {
                p.frames.swap(i, i + 1);
            }
        }
    }

    pub fn send_backward(&mut self, page: usize, ids: &[u64]) {
        let Some(p) = self.page_mut(page) else { return };
        for i in 1..p.frames.len() {
            if ids.contains(&p.frames[i].id) && !ids.contains(&p.frames[i - 1].id) {
                p.frames.swap(i, i - 1);
            }
        }
    }

    pub fn bring_to_front(&mut self, page: usize, ids: &[u64]) {
        if let Some(p) = self.page_mut(page) {
            let (mut sel, mut rest): (Vec<Frame>, Vec<Frame>) = p.frames.drain(..).partition(|f| ids.contains(&f.id));
            rest.append(&mut sel);
            p.frames = rest;
        }
    }

    pub fn send_to_back(&mut self, page: usize, ids: &[u64]) {
        if let Some(p) = self.page_mut(page) {
            let (mut sel, mut rest): (Vec<Frame>, Vec<Frame>) = p.frames.drain(..).partition(|f| ids.contains(&f.id));
            sel.append(&mut rest);
            p.frames = sel;
        }
    }

    pub fn group(&mut self, page: usize, ids: &[u64]) -> bool {
        if ids.len() < 2 {
            return false;
        }
        let g = self.take_id();
        if let Some(p) = self.page_mut(page) {
            for f in p.frames.iter_mut().filter(|f| ids.contains(&f.id)) {
                f.group = g;
            }
        }
        true
    }

    pub fn ungroup(&mut self, page: usize, ids: &[u64]) -> bool {
        let Some(p) = self.page_mut(page) else { return false };
        let groups: Vec<u64> = p.frames.iter().filter(|f| ids.contains(&f.id) && f.group != 0).map(|f| f.group).collect();
        if groups.is_empty() {
            return false;
        }
        for f in p.frames.iter_mut().filter(|f| groups.contains(&f.group)) {
            f.group = 0;
        }
        true
    }

    pub fn rotate(&mut self, page: usize, ids: &[u64], quarter_turns: i32) {
        if let Some(p) = self.page_mut(page) {
            // Le tabelle restano dritte.
            for f in p.frames.iter_mut().filter(|f| ids.contains(&f.id) && !matches!(f.kind, FrameKind::Table { .. })) {
                f.rot =((f.rot as i32 + quarter_turns).rem_euclid(4)) as u8;
            }
        }
    }

    pub fn translate(&mut self, page: usize, ids: &[u64], dx: f32, dy: f32) {
        if let Some(p) = self.page_mut(page) {
            for f in p.frames.iter_mut().filter(|f| ids.contains(&f.id)) {
                f.rect.x += dx;
                f.rect.y += dy;
            }
        }
    }

    /// Allinea gli elementi: alla pagina se è uno solo, altrimenti al loro insieme.
    pub fn align(&mut self, page: usize, ids: &[u64], kind: AlignKind) {
        let size = self.page_size;
        let target = if ids.len() == 1 {
            Rect { x: 0.0, y: 0.0, w: size.w_mm, h: size.h_mm }
        } else {
            match self.bounds(page, ids) {
                Some(b) => b,
                None => return,
            }
        };
        let Some(p) = self.page_mut(page) else { return };
        for f in p.frames.iter_mut().filter(|f| ids.contains(&f.id)) {
            let mut v = f.visual_rect();
            match kind {
                AlignKind::Left => v.x = target.x,
                AlignKind::CenterH => v.x = target.x + (target.w - v.w) / 2.0,
                AlignKind::Right => v.x = target.right() - v.w,
                AlignKind::Top => v.y = target.y,
                AlignKind::CenterV => v.y = target.y + (target.h - v.h) / 2.0,
                AlignKind::Bottom => v.y = target.bottom() - v.h,
            }
            f.set_visual_rect(v);
        }
    }

    /// Distribuisce con spazi uguali (servono almeno 3 elementi).
    pub fn distribute(&mut self, page: usize, ids: &[u64], horizontal: bool) {
        if ids.len() < 3 {
            return;
        }
        let Some(p) = self.page_mut(page) else { return };
        let mut items: Vec<(usize, Rect)> =
            p.frames.iter().enumerate().filter(|(_, f)| ids.contains(&f.id)).map(|(i, f)| (i, f.visual_rect())).collect();
        if horizontal {
            items.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
        } else {
            items.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
        }
        let first = items[0].1;
        let last = items[items.len() - 1].1;
        let total: f32 = items.iter().map(|(_, r)| if horizontal { r.w } else { r.h }).sum();
        let span = if horizontal { last.right() - first.x } else { last.bottom() - first.y };
        let gap = (span - total) / (items.len() as f32 - 1.0);
        let mut pos = if horizontal { first.x } else { first.y };
        for (i, r) in items {
            let mut v = r;
            if horizontal {
                v.x = pos;
                pos += v.w + gap;
            } else {
                v.y = pos;
                pos += v.h + gap;
            }
            p.frames[i].set_visual_rect(v);
        }
    }

    // ---- Appunti ----

    pub fn copy_frames(&self, page: usize, ids: &[u64]) -> Vec<Frame> {
        self.page(page).map(|p| p.frames.iter().filter(|f| ids.contains(&f.id)).cloned().collect()).unwrap_or_default()
    }

    /// Inserisce copie dei riquadri (con nuovi id e gruppi) spostate di (dx, dy).
    pub fn paste_frames(&mut self, page: usize, frames: &[Frame], dx: f32, dy: f32) -> Vec<u64> {
        let mut groups: Vec<(u64, u64)> = Vec::new();
        let mut out = Vec::new();
        for f in frames {
            let id = self.take_id();
            let mut n = f.clone();
            n.id = id;
            n.rect.x += dx;
            n.rect.y += dy;
            if n.group != 0 {
                let g = match groups.iter().find(|(old, _)| *old == f.group) {
                    Some((_, g)) => *g,
                    None => {
                        let g = self.take_id();
                        groups.push((f.group, g));
                        g
                    }
                };
                n.group = g;
            }
            if let Some(p) = self.page_mut(page) {
                p.frames.push(n);
                out.push(id);
            }
        }
        out
    }

    // ---- Trova e sostituisci ----

    fn frame_texts(f: &Frame) -> Vec<&String> {
        match &f.kind {
            FrameKind::Text { text, .. } => vec![text],
            FrameKind::Table { cells, .. } => cells.iter().collect(),
            _ => Vec::new(),
        }
    }

    /// Elementi (pagina, id) che contengono il testo cercato, in ordine di lettura.
    pub fn find(&self, needle: &str, match_case: bool) -> Vec<(usize, u64)> {
        if needle.is_empty() {
            return Vec::new();
        }
        let n = if match_case { needle.to_string() } else { needle.to_lowercase() };
        let mut out = Vec::new();
        for (pi, p) in self.pages.iter().enumerate() {
            for f in &p.frames {
                let hit = Self::frame_texts(f).iter().any(|t| if match_case { t.contains(&n) } else { t.to_lowercase().contains(&n) });
                if hit {
                    out.push((pi, f.id));
                }
            }
        }
        out
    }

    /// Sostituisce tutte le occorrenze; restituisce quante.
    pub fn replace_all(&mut self, needle: &str, with: &str, match_case: bool) -> usize {
        if needle.is_empty() {
            return 0;
        }
        let mut count = 0;
        let mut fix = |s: &mut String| {
            let (n, c) = replace_counting(s, needle, with, match_case);
            if c > 0 {
                *s = n;
                count += c;
            }
        };
        for p in self.pages.iter_mut().chain(std::iter::once(&mut self.master)) {
            for f in &mut p.frames {
                match &mut f.kind {
                    FrameKind::Text { text, .. } => fix(text),
                    FrameKind::Table { cells, .. } => cells.iter_mut().for_each(&mut fix),
                    _ => {}
                }
            }
        }
        count
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

fn replace_counting(s: &str, needle: &str, with: &str, match_case: bool) -> (String, usize) {
    if match_case {
        let c = s.matches(needle).count();
        return (s.replace(needle, with), c);
    }
    let hay = s.to_lowercase();
    let nl = needle.to_lowercase();
    // Se la minuscola cambia le lunghezze in byte non posso mappare le posizioni: ripiego sul confronto esatto.
    if hay.len() != s.len() {
        let c = s.matches(needle).count();
        return (s.replace(needle, with), c);
    }
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut count = 0;
    let mut from = 0;
    while let Some(pos) = hay[from..].find(&nl) {
        let at = from + pos;
        out.push_str(&s[last..at]);
        out.push_str(with);
        last = at + nl.len();
        from = last;
        count += 1;
    }
    out.push_str(&s[last..]);
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> FrameKind {
        FrameKind::Text { text: "Ciao".into(), size_pt: 12.0, color: Rgb(0, 0, 0), style: TextStyle::default() }
    }

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn add_hit_remove() {
        let mut d = Document::default();
        let a = d.add_frame(0, r(10.0, 10.0, 50.0, 20.0), text()).unwrap();
        let b = d.add_frame(0, r(20.0, 15.0, 50.0, 20.0), text()).unwrap();
        assert_eq!(d.hit_test(0, 25.0, 18.0), Some(b)); // quello sopra vince
        assert_eq!(d.hit_test(0, 12.0, 12.0), Some(a));
        assert_eq!(d.hit_test(0, 200.0, 200.0), None);
        assert!(d.remove_frame(0, b));
        assert_eq!(d.hit_test(0, 25.0, 18.0), Some(a));
    }

    #[test]
    fn old_files_still_load() {
        let old = r#"{"page_size":{"w_mm":210.0,"h_mm":297.0},"pages":[{"frames":[{"id":1,"rect":{"x":0.0,"y":0.0,"w":10.0,"h":10.0},"kind":{"Text":{"text":"a","size_pt":12.0,"color":[0,0,0]}}}]}],"next_id":2}"#;
        let d = Document::from_json(old).unwrap();
        let f = &d.pages[0].frames[0];
        assert_eq!((f.rot, f.group, f.wrap), (0, 0, Wrap::None));
        match &f.kind {
            FrameKind::Text { style, .. } => assert_eq!(*style, TextStyle::default()),
            _ => panic!("atteso testo"),
        }
        // Stile salvato dalla versione precedente (senza i campi nuovi).
        let s: TextStyle = serde_json::from_str(r#"{"bold":true,"italic":false,"underline":false,"align":"Center","family":"Mono"}"#).unwrap();
        assert!(s.bold);
        assert_eq!(s.line_spacing, 1.0);
    }

    #[test]
    fn invalid_page_is_none() {
        let mut d = Document::default();
        assert!(d.add_frame(5, r(0.0, 0.0, 1.0, 1.0), text()).is_none());
    }

    #[test]
    fn json_roundtrip() {
        let mut d = Document::default();
        d.add_frame(0, r(1.0, 2.0, 3.0, 4.0), text());
        d.add_frame(MASTER, r(1.0, 2.0, 3.0, 4.0), FrameKind::table(2, 3));
        d.add_page();
        let back = Document::from_json(&d.to_json().unwrap()).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn z_order() {
        let mut d = Document::default();
        let ids: Vec<u64> = (0..4).map(|i| d.add_frame(0, r(i as f32, 0.0, 5.0, 5.0), text()).unwrap()).collect();
        let order = |d: &Document| d.pages[0].frames.iter().map(|f| f.id).collect::<Vec<_>>();
        d.bring_forward(0, &[ids[0]]);
        assert_eq!(order(&d), vec![ids[1], ids[0], ids[2], ids[3]]);
        d.send_to_back(0, &[ids[3]]);
        assert_eq!(order(&d), vec![ids[3], ids[1], ids[0], ids[2]]);
        d.bring_to_front(0, &[ids[3], ids[1]]);
        assert_eq!(order(&d), vec![ids[0], ids[2], ids[3], ids[1]]);
        d.send_backward(0, &[ids[1]]);
        assert_eq!(order(&d), vec![ids[0], ids[2], ids[1], ids[3]]);
    }

    #[test]
    fn group_rotate_and_hit() {
        let mut d = Document::default();
        let a = d.add_frame(0, r(0.0, 0.0, 10.0, 10.0), text()).unwrap();
        let b = d.add_frame(0, r(20.0, 0.0, 10.0, 10.0), text()).unwrap();
        assert!(d.group(0, &[a, b]));
        assert_eq!(d.with_group(0, a).len(), 2);
        assert!(d.ungroup(0, &[b]));
        assert_eq!(d.with_group(0, a), vec![a]);

        let c = d.add_frame(0, r(0.0, 50.0, 40.0, 10.0), text()).unwrap();
        d.rotate(0, &[c], 1);
        let v = d.frame(0, c).unwrap().visual_rect();
        assert_eq!((v.w, v.h), (10.0, 40.0));
        assert_eq!(d.hit_test(0, 20.0, 40.0), Some(c));
        d.rotate(0, &[c], 3);
        assert_eq!(d.frame(0, c).unwrap().rot, 0);
    }

    #[test]
    fn align_and_distribute() {
        let mut d = Document::default();
        let a = d.add_frame(0, r(10.0, 5.0, 10.0, 10.0), text()).unwrap();
        let b = d.add_frame(0, r(40.0, 30.0, 20.0, 10.0), text()).unwrap();
        let c = d.add_frame(0, r(100.0, 60.0, 10.0, 10.0), text()).unwrap();
        d.align(0, &[a, b, c], AlignKind::Left);
        assert!(d.pages[0].frames.iter().all(|f| f.rect.x == 10.0));
        d.align(0, &[a], AlignKind::CenterH);
        assert_eq!(d.frame(0, a).unwrap().rect.x, 100.0);
        d.align(0, &[a, b, c], AlignKind::Left);
        d.translate(0, &[c], 90.0, 0.0);
        d.distribute(0, &[a, b, c], true);
        let xs: Vec<f32> = [a, b, c].iter().map(|i| d.frame(0, *i).unwrap().rect.x).collect();
        assert!((xs[1] - xs[0] - 10.0 - (xs[2] - xs[1] - 20.0)).abs() < 1e-3);
    }

    #[test]
    fn paste_gets_new_ids_and_groups() {
        let mut d = Document::default();
        let a = d.add_frame(0, r(0.0, 0.0, 10.0, 10.0), text()).unwrap();
        let b = d.add_frame(0, r(20.0, 0.0, 10.0, 10.0), text()).unwrap();
        d.group(0, &[a, b]);
        let copy = d.copy_frames(0, &[a, b]);
        let ids = d.paste_frames(0, &copy, 5.0, 5.0);
        assert_eq!(ids.len(), 2);
        assert!(!ids.contains(&a) && !ids.contains(&b));
        assert_eq!(d.with_group(0, ids[0]).len(), 2);
        assert_eq!(d.with_group(0, a).len(), 2);
        assert_eq!(d.frame(0, ids[0]).unwrap().rect.x, 5.0);
    }

    #[test]
    fn find_and_replace() {
        let mut d = Document::default();
        d.add_frame(0, r(0.0, 0.0, 10.0, 10.0), text());
        let t = d.add_frame(0, r(0.0, 0.0, 10.0, 10.0), FrameKind::table(1, 2)).unwrap();
        if let Some(Frame { kind: FrameKind::Table { cells, .. }, .. }) = d.frame_mut(0, t) {
            cells[1] = "ciao mondo CIAO".into();
        }
        assert_eq!(d.find("CIAO", false).len(), 2);
        assert_eq!(d.find("CIAO", true).len(), 1);
        assert_eq!(d.replace_all("ciao", "hey", false), 3);
        assert!(d.find("ciao", false).is_empty());
        assert_eq!(d.find("hey", true).len(), 2);
    }

    #[test]
    fn case_changes() {
        assert_eq!(change_case("ciao MONDO", CaseKind::Upper), "CIAO MONDO");
        assert_eq!(change_case("ciao MONDO", CaseKind::Title), "Ciao Mondo");
        assert_eq!(change_case("ciao. altra FRASE", CaseKind::Sentence), "Ciao. Altra frase");
        assert_eq!(change_case("aB", CaseKind::Toggle), "Ab");
    }

    #[test]
    fn insert_and_duplicate_page() {
        let mut d = Document::default();
        let a = d.add_frame(0, r(0.0, 0.0, 5.0, 5.0), text()).unwrap();
        let b = d.add_frame(0, r(9.0, 0.0, 5.0, 5.0), text()).unwrap();
        d.group(0, &[a, b]);
        assert_eq!(d.insert_page(0), 0);
        assert_eq!(d.pages.len(), 2);
        let at = d.duplicate_page(1).unwrap();
        assert_eq!((at, d.pages.len()), (2, 3));
        assert_eq!(d.pages[2].frames.len(), 2);
        assert_ne!(d.pages[2].frames[0].id, a);
        assert_eq!(d.with_group(2, d.pages[2].frames[0].id).len(), 2);
    }

    #[test]
    fn master_and_pages() {
        let mut d = Document::default();
        assert!(d.add_frame(MASTER, r(0.0, 0.0, 5.0, 5.0), text()).is_some());
        assert_eq!(d.master.frames.len(), 1);
        d.add_page();
        assert!(d.remove_page(0));
        assert!(!d.remove_page(0));
    }
}
