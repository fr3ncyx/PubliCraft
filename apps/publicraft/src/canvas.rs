//! Tela: pagine impilate (o affiancate), righelli, miniature, disegno degli elementi e interazione.

use super::*;
use egui::epaint::{Mesh, TextShape, Vertex};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, CornerRadius, FontId, Painter, Sense, Shape, Stroke, StrokeKind};
use egui::Rect as ERect;
use publicraft_doc::ListKind;

pub(crate) const PAGE_TOKEN: &str = "{pagina}";
const MARGIN: f32 = 40.0;
const GAP_MM: f32 = 12.0;
const SPREAD_GAP_MM: f32 = 2.0;
const RULER: f32 = 18.0;
const HANDLE_PX: f32 = 9.0;
const MARGIN_GUIDE_MM: f32 = 12.7;
const BASELINE_MM: f32 = 5.08;

struct Look<'a> {
    scale: f32,
    textures: &'a HashMap<String, Option<Tex>>,
    marks: bool,
    fields: bool,
    /// Numero della pagina disegnata (0 = master).
    page_no: usize,
}

fn rot_point(p: Pos2, c: Pos2, angle: f32) -> Pos2 {
    let (s, co) = angle.sin_cos();
    let d = p - c;
    c + vec2(d.x * co - d.y * s, d.x * s + d.y * co)
}

fn screen_rect(origin: Pos2, r: &Rect, scale: f32) -> ERect {
    ERect::from_min_size(origin + vec2(r.x, r.y) * scale, vec2(r.w, r.h) * scale)
}

fn draw_text(p: &Painter, r0: ERect, angle: f32, text: &str, size_pt: f32, color: Color32, style: &TextStyle, look: &Look) {
    let scale = look.scale;
    let px = size_pt * MM_PER_PT * scale;
    if px < 1.0 {
        return;
    }
    let c = r0.center();
    let run_px = if style.sub || style.sup { px * 0.65 } else { px };
    let fmt = TextFormat {
        font_id: FontId::new(run_px, fonts::family(style.family)),
        color,
        italics: style.italic,
        underline: if style.underline { Stroke::new(1.0, color) } else { Stroke::NONE },
        extra_letter_spacing: style.tracking_pt * MM_PER_PT * scale,
        line_height: Some(px * 1.22 * style.line_spacing.max(0.5)),
        valign: if style.sup {
            Align::TOP
        } else if style.sub {
            Align::BOTTOM
        } else {
            Align::Center
        },
        ..Default::default()
    };
    let put = |pos: Pos2, galley: std::sync::Arc<egui::Galley>| {
        let shape = TextShape::new(rot_point(pos, c, angle), galley.clone(), color).with_angle(angle);
        p.add(shape);
        if style.bold {
            // Il font di base non ha il grassetto: lo simulo sovrapponendo il testo.
            let off = (px * 0.035).max(0.6);
            let shape = TextShape::new(rot_point(pos + vec2(off, 0.0), c, angle), galley, color).with_angle(angle);
            p.add(shape);
        }
    };
    let indent = style.indent as f32 * 6.0 * scale;
    let list_w = if style.list == ListKind::None { 0.0 } else { px * 1.8 };
    let avail = (r0.width() - indent - list_w).max(px);
    let mut y = 0.0;
    for (n, para) in text.split('\n').enumerate() {
        let mut job = LayoutJob::default();
        job.wrap.max_width = avail;
        job.halign = match style.align {
            DAlign::Left | DAlign::Justify => Align::Min,
            DAlign::Center => Align::Center,
            DAlign::Right => Align::Max,
        };
        job.justify = style.align == DAlign::Justify;
        job.append(if para.is_empty() { " " } else { para }, 0.0, fmt.clone());
        if look.marks {
            job.append("¶", 0.0, TextFormat { color: Color32::from_rgb(0x5a, 0x7f, 0xd0), ..fmt.clone() });
        }
        let galley = p.layout_job(job);
        let h = galley.size().y;
        // L'allineamento è relativo all'ancora: lo riporto dentro la larghezza disponibile.
        let dx = match style.align {
            DAlign::Center => avail / 2.0 - galley.rect.center().x,
            DAlign::Right => avail - galley.rect.max.x,
            _ => -galley.rect.min.x,
        };
        put(r0.min + vec2(indent + list_w + dx, y), galley);
        if style.list != ListKind::None {
            let marker = if style.list == ListKind::Bullet { "•".to_string() } else { format!("{}.", n + 1) };
            let mut j = LayoutJob::default();
            j.append(&marker, 0.0, fmt.clone());
            let g = p.layout_job(j);
            put(r0.min + vec2(indent, y), g);
        }
        y += h;
    }
}

fn draw_frame(p: &Painter, f: &Frame, origin: Pos2, look: &Look) {
    let s = look.scale;
    let r0 = screen_rect(origin, &f.rect, s);
    let rv = screen_rect(origin, &f.visual_rect(), s);
    let c = r0.center();
    let angle = f.rot as f32 * std::f32::consts::FRAC_PI_2;
    let line = |w_mm: f32, col: Rgb| Stroke::new((w_mm * s).max(0.5), rgb(col));
    match &f.kind {
        FrameKind::Rectangle { fill, stroke, stroke_mm } => {
            p.rect_filled(rv, CornerRadius::ZERO, rgb(*fill));
            if *stroke_mm > 0.0 {
                p.rect_stroke(rv, CornerRadius::ZERO, line(*stroke_mm, *stroke), StrokeKind::Inside);
            }
        }
        FrameKind::Ellipse { fill, stroke, stroke_mm } => {
            let mut e = egui::epaint::EllipseShape::filled(rv.center(), rv.size() / 2.0, rgb(*fill));
            if *stroke_mm > 0.0 {
                e.stroke = line(*stroke_mm, *stroke);
            }
            p.add(e);
        }
        FrameKind::Triangle { fill, stroke, stroke_mm } => {
            let pts = [r0.center_top(), r0.right_bottom(), r0.left_bottom()].map(|q| rot_point(q, c, angle)).to_vec();
            let st = if *stroke_mm > 0.0 { line(*stroke_mm, *stroke) } else { Stroke::NONE };
            p.add(Shape::convex_polygon(pts, rgb(*fill), st));
        }
        FrameKind::Line { stroke, stroke_mm } => {
            p.line_segment([rot_point(r0.left_top(), c, angle), rot_point(r0.right_bottom(), c, angle)], line(*stroke_mm, *stroke));
        }
        FrameKind::Picture { path } => match look.textures.get(path).and_then(|t| t.as_ref()) {
            Some(t) => {
                let corners = [rv.left_top(), rv.right_top(), rv.right_bottom(), rv.left_bottom()];
                let uvs = [egui::pos2(0.0, 0.0), egui::pos2(1.0, 0.0), egui::pos2(1.0, 1.0), egui::pos2(0.0, 1.0)];
                let mut mesh = Mesh::with_texture(t.handle.id());
                for (i, pos) in corners.iter().enumerate() {
                    mesh.vertices.push(Vertex { pos: *pos, uv: uvs[(i + 4 - f.rot as usize % 4) % 4], color: Color32::WHITE });
                }
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                p.add(Shape::mesh(mesh));
            }
            None => {
                p.rect_filled(rv, CornerRadius::ZERO, Color32::from_gray(0xe4));
                p.rect_stroke(rv, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_gray(0x90)), StrokeKind::Inside);
                p.text(rv.center(), egui::Align2::CENTER_CENTER, "Immagine non trovata", FontId::proportional(12.0), Color32::from_gray(0x50));
            }
        },
        FrameKind::Table { rows, cols, cells, border, border_mm, size_pt } => {
            let (cw, ch) = (r0.width() / *cols as f32, r0.height() / *rows as f32);
            let st = if *border_mm > 0.0 { line(*border_mm, *border) } else { Stroke::NONE };
            let px = size_pt * MM_PER_PT * s;
            for y in 0..*rows {
                for x in 0..*cols {
                    let cell = ERect::from_min_size(r0.min + vec2(x as f32 * cw, y as f32 * ch), vec2(cw, ch));
                    p.rect_stroke(cell, CornerRadius::ZERO, st, StrokeKind::Inside);
                    let t = &cells[y * *cols + x];
                    if !t.is_empty() && px >= 1.0 {
                        p.with_clip_rect(cell).text(cell.left_center() + vec2(px * 0.4, 0.0), egui::Align2::LEFT_CENTER, t, FontId::proportional(px), Color32::from_rgb(0x20, 0x20, 0x20));
                    }
                }
            }
        }
        FrameKind::Text { text, size_pt, color, style } => {
            let shown;
            let text = if text.contains(PAGE_TOKEN) {
                let n = if look.page_no == 0 { "#".to_string() } else { look.page_no.to_string() };
                shown = text.replace(PAGE_TOKEN, &n);
                &shown
            } else {
                text
            };
            if look.fields && text.contains("<<") {
                p.rect_filled(rv, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(0xff, 0xe0, 0x66, 90));
            }
            draw_text(&p.with_clip_rect(rv), r0, angle, text, *size_pt, rgb(*color), style, look);
        }
    }
}

fn dashed_rect(p: &Painter, r: ERect, stroke: Stroke) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&pts, stroke, 4.0, 3.0));
}

fn handle_rect(doc: &Document, pg: usize, sel: &[u64], origin: Pos2, scale: f32) -> Option<ERect> {
    if sel.len() != 1 {
        return None;
    }
    let f = doc.frame(pg, sel[0])?;
    Some(ERect::from_center_size(screen_rect(origin, &f.visual_rect(), scale).right_bottom(), Vec2::splat(HANDLE_PX)))
}

fn paint_page(p: &Painter, doc: &Document, idx: usize, origin: Pos2, look: &Look) {
    if idx != MASTER {
        for f in &doc.master.frames {
            draw_frame(p, f, origin, look);
        }
    }
    if let Some(page) = doc.page(idx) {
        for f in &page.frames {
            draw_frame(p, f, origin, look);
        }
    }
}

impl PubliCraft {
    /// Posizione (mm) di ogni pagina visibile nello spazio della tela, e dimensione totale.
    pub(crate) fn slots(&self) -> (Vec<(usize, Vec2)>, Vec2) {
        let s = self.d.doc.page_size;
        let (w, h) = (s.w_mm, s.h_mm);
        if self.view.master {
            return (vec![(MASTER, Vec2::ZERO)], vec2(w, h));
        }
        let n = self.d.doc.pages.len();
        let mut out = Vec::new();
        if self.view.spread {
            for i in 0..n {
                let row = (i / 2) as f32;
                let col = (i % 2) as f32;
                out.push((i, vec2(col * (w + SPREAD_GAP_MM), row * (h + GAP_MM))));
            }
            let rows = n.div_ceil(2) as f32;
            let width = if n > 1 { 2.0 * w + SPREAD_GAP_MM } else { w };
            (out, vec2(width, rows * h + (rows - 1.0) * GAP_MM))
        } else {
            for i in 0..n {
                out.push((i, vec2(0.0, i as f32 * (h + GAP_MM))));
            }
            (out, vec2(w, n as f32 * h + (n as f32 - 1.0) * GAP_MM))
        }
    }

    /// Posizione (px, nello spazio del contenuto scorrevole) dell'origine di una pagina.
    fn content_origin(&self, off: Vec2, total_mm: Vec2) -> Vec2 {
        let scale = self.scale();
        let total_px = total_mm * scale;
        let alloc_w = (total_px.x + MARGIN * 2.0).max(self.viewport.x);
        vec2((alloc_w - total_px.x) / 2.0 + off.x * scale, MARGIN + off.y * scale)
    }

    pub(crate) fn page_top_px(&self, i: usize) -> f32 {
        let (slots, total) = self.slots();
        slots.iter().find(|(idx, _)| *idx == i).map(|(_, o)| self.content_origin(*o, total).y - MARGIN).unwrap_or(0.0)
    }

    fn apply_fit(&mut self, fit: Fit) {
        let (slots, total) = self.slots();
        let s = self.d.doc.page_size;
        let vw = (self.viewport.x - 2.0 * MARGIN - 16.0).max(50.0);
        let vh = (self.viewport.y - 2.0 * MARGIN).max(50.0);
        match fit {
            Fit::Page => {
                self.set_zoom((vw / (s.w_mm * PX_PER_MM)).min(vh / (s.h_mm * PX_PER_MM)));
                let y = self.page_top_px(self.pg());
                self.scroll_target = Some(vec2(0.0, y));
            }
            Fit::Width => {
                self.set_zoom(vw / (total.x * PX_PER_MM));
                let y = self.page_top_px(self.pg());
                self.scroll_target = Some(vec2(0.0, y));
            }
            Fit::Selection => {
                let pg = self.pg();
                let Some(b) = self.d.doc.bounds(pg, &self.d.sel) else { return };
                let z = (self.viewport.x * 0.8 / (b.w.max(1.0) * PX_PER_MM)).min(self.viewport.y * 0.8 / (b.h.max(1.0) * PX_PER_MM));
                self.set_zoom(z);
                let Some((_, off)) = slots.iter().find(|(i, _)| *i == pg) else { return };
                let o = self.content_origin(*off, total);
                let sc = self.scale();
                let center = o + vec2(b.x + b.w / 2.0, b.y + b.h / 2.0) * sc;
                self.scroll_target = Some((center - self.viewport / 2.0).max(Vec2::ZERO));
            }
        }
    }

    pub(crate) fn ensure_textures(&mut self, ctx: &egui::Context) {
        let mut missing: Vec<String> = Vec::new();
        for p in self.d.doc.pages.iter().chain(std::iter::once(&self.d.doc.master)) {
            for f in &p.frames {
                if let FrameKind::Picture { path } = &f.kind {
                    if !self.textures.contains_key(path) && !missing.contains(path) {
                        missing.push(path.clone());
                    }
                }
            }
        }
        for path in missing {
            let tex = image::open(&path).ok().map(|img| {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                Tex { handle: ctx.load_texture(&path, ci, egui::TextureOptions::LINEAR), size }
            });
            self.textures.insert(path, tex);
        }
    }

    /// Miniatura nel pannello Pagine. Restituisce `Some(true)` se è stata cliccata.
    pub(crate) fn thumbnail(&mut self, ui: &mut egui::Ui, i: usize) -> Option<bool> {
        let w = 108.0;
        let s = self.d.doc.page_size;
        let scale = w / s.w_mm;
        let h = s.h_mm * scale;
        let (resp, painter) = ui.allocate_painter(vec2(w + 12.0, h + 26.0), Sense::click());
        let page_rect = ERect::from_min_size(resp.rect.min + vec2(6.0, 6.0), vec2(w, h));
        painter.rect_filled(page_rect, CornerRadius::ZERO, Color32::WHITE);
        let look = Look { scale, textures: &self.textures, marks: false, fields: false, page_no: i + 1 };
        paint_page(&painter.with_clip_rect(page_rect), &self.d.doc, i, page_rect.min, &look);
        let cur = i == self.d.page && !self.view.master;
        let col = if cur {
            craft_theme::ACCENT_PUBLICRAFT
        } else if resp.hovered() {
            Color32::from_gray(0x90)
        } else {
            Color32::from_gray(0x50)
        };
        painter.rect_stroke(page_rect, CornerRadius::ZERO, Stroke::new(if cur { 2.5 } else { 1.0 }, col), StrokeKind::Outside);
        painter.text(egui::pos2(page_rect.center().x, page_rect.max.y + 12.0), egui::Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(13.0), craft_theme::TEXT);
        Some(resp.clicked())
    }

    pub(crate) fn canvas(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap();
        let rul = if self.view.rulers { RULER } else { 0.0 };
        let body = ERect::from_min_max(outer.min + vec2(rul, rul), outer.max);
        self.viewport = body.size();

        if ui.rect_contains_pointer(body) {
            let z = ui.input(|i| i.zoom_delta());
            if z != 1.0 {
                self.set_zoom(self.d.zoom * z);
            }
        }
        if let Some(fit) = self.fit.take() {
            self.apply_fit(fit);
        }

        let scale = self.scale();
        let (slots, total_mm) = self.slots();
        let total_px = total_mm * scale;
        let alloc = (total_px + Vec2::splat(MARGIN * 2.0)).max(body.size());
        let pad_x = (alloc.x - total_px.x) / 2.0;
        let target = self.scroll_target.take();
        let size = self.d.doc.page_size;
        let page_px = vec2(size.w_mm, size.h_mm) * scale;
        let mut base = body.min;

        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            let mut sa = egui::ScrollArea::both().auto_shrink(false).id_salt("canvas");
            if let Some(t) = target {
                sa = sa.scroll_offset(t);
            }
            let out = sa.show(ui, |ui| {
                let (resp, painter) = ui.allocate_painter(alloc, Sense::click_and_drag());
                let b = resp.rect.min + vec2(pad_x, MARGIN);
                self.canvas_content(ui, &resp, &painter, b, &slots, scale, page_px);
                b
            });
            self.scroll_off = out.state.clamped_offset();
            base = out.inner;
        });

        if self.view.rulers {
            self.draw_rulers(ui, outer, body, base, scale);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn canvas_content(&mut self, ui: &egui::Ui, resp: &egui::Response, painter: &Painter, base: Pos2, slots: &[(usize, Vec2)], scale: f32, page_px: Vec2) {
        let cur = self.pg();
        let origin_of = |idx: usize| -> Pos2 {
            let off = slots.iter().find(|(i, _)| *i == idx).map(|(_, o)| *o).unwrap_or(Vec2::ZERO);
            base + off * scale
        };

        // ---- Disegno ----
        {
            for (idx, _) in slots {
                let look = Look { scale, textures: &self.textures, marks: self.view.marks, fields: self.view.fields, page_no: if *idx == MASTER { 0 } else { *idx + 1 } };
                let origin = origin_of(*idx);
                let page_rect = ERect::from_min_size(origin, page_px);
                painter.rect_filled(page_rect.translate(vec2(3.0, 4.0)), CornerRadius::ZERO, Color32::from_black_alpha(90));
                painter.rect_filled(page_rect, CornerRadius::ZERO, Color32::WHITE);
                let clip = if self.view.scratch { painter.clip_rect() } else { page_rect.intersect(painter.clip_rect()) };
                let fp = painter.with_clip_rect(clip);
                if *idx == MASTER {
                    // Nella vista master si mostra solo la master.
                    for f in &self.d.doc.master.frames {
                        draw_frame(&fp, f, origin, &look);
                    }
                } else {
                    paint_page(&fp, &self.d.doc, *idx, origin, &look);
                }

                let pr = painter.with_clip_rect(page_rect.expand(2.0));
                if self.view.baselines {
                    let mut y = MARGIN_GUIDE_MM;
                    while y < self.d.doc.page_size.h_mm - MARGIN_GUIDE_MM {
                        let yy = origin.y + y * scale;
                        pr.hline(page_rect.x_range(), yy, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0x5a, 0x9b, 0xe0, 70)));
                        y += BASELINE_MM;
                    }
                }
                if self.view.guides {
                    let m = MARGIN_GUIDE_MM * scale;
                    let g = ERect::from_min_max(page_rect.min + vec2(m, m), page_rect.max - vec2(m, m));
                    pr.rect_stroke(g, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_rgb(0x4a, 0x90, 0xe2)), StrokeKind::Inside);
                    let c = page_rect.center();
                    let pink = Stroke::new(1.0, Color32::from_rgb(0xe0, 0x4a, 0x9a));
                    pr.vline(c.x, page_rect.y_range(), pink);
                    pr.hline(page_rect.x_range(), c.y, pink);
                }
                if self.view.boundaries {
                    let st = Stroke::new(1.0, Color32::from_gray(0x8a));
                    let m = MARGIN_GUIDE_MM * scale;
                    dashed_rect(&pr, ERect::from_min_max(page_rect.min + vec2(m, m), page_rect.max - vec2(m, m)), Stroke::new(1.0, Color32::from_rgb(0x8a, 0x6c, 0xd0)));
                    if let Some(page) = self.d.doc.page(*idx) {
                        for f in &page.frames {
                            dashed_rect(painter, screen_rect(origin, &f.visual_rect(), scale), st);
                        }
                    }
                }

                if *idx == cur {
                    let acc = craft_theme::ACCENT_PUBLICRAFT;
                    for id in &self.d.sel {
                        if let Some(f) = self.d.doc.frame(cur, *id) {
                            painter.rect_stroke(screen_rect(origin, &f.visual_rect(), scale), CornerRadius::ZERO, Stroke::new(1.5, acc), StrokeKind::Outside);
                        }
                    }
                    if self.d.sel.len() > 1 {
                        if let Some(b) = self.d.doc.bounds(cur, &self.d.sel) {
                            dashed_rect(painter, screen_rect(origin, &b, scale).expand(3.0), Stroke::new(1.0, acc));
                        }
                    }
                    if let Some(h) = handle_rect(&self.d.doc, cur, &self.d.sel, origin, scale) {
                        painter.rect_filled(h, CornerRadius::same(2), acc);
                    }
                }
            }
        }

        // ---- Interazione ----
        let locate = |p: Pos2| -> (usize, Pos2, bool) {
            let mut best = (slots[0].0, f32::MAX, false);
            for (idx, _) in slots {
                let r = ERect::from_min_size(origin_of(*idx), page_px);
                let dx = (r.min.x - p.x).max(0.0).max(p.x - r.max.x);
                let dy = (r.min.y - p.y).max(0.0).max(p.y - r.max.y);
                let d = dx.hypot(dy);
                if d < best.1 {
                    best = (*idx, d, d == 0.0);
                }
            }
            let o = origin_of(best.0);
            (best.0, ((p - o) / scale).to_pos2(), best.2)
        };
        let to_mm = |page: usize, p: Pos2| -> Pos2 { ((p - origin_of(page)) / scale).to_pos2() };

        if let Some(h) = resp.hover_pos() {
            let (idx, m, inside) = locate(h);
            self.cursor = inside.then_some((idx, m.x, m.y));
            if self.tool != Tool::Select || self.fmt_painter.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        } else {
            self.cursor = None;
        }

        let shift = ui.input(|i| i.modifiers.shift);
        let ptr = resp.interact_pointer_pos();

        if resp.drag_started() {
            if let Some(p) = ptr {
                let (idx, m, _) = locate(p);
                if idx != MASTER && self.d.page != idx {
                    self.d.sel.clear();
                    self.d.page = idx;
                }
                self.d.pending = Some(self.d.doc.clone());
                if self.tool == Tool::Select {
                    let on_handle = handle_rect(&self.d.doc, idx, &self.d.sel, origin_of(idx), scale).is_some_and(|h| h.expand(3.0).contains(p));
                    if on_handle {
                        self.drag = Some(Drag::Resize { page: idx, id: self.d.sel[0] });
                    } else if let Some(id) = self.d.doc.hit_test(idx, m.x, m.y) {
                        if !self.d.sel.contains(&id) {
                            let members = self.d.doc.with_group(idx, id);
                            if shift {
                                for mid in members {
                                    if !self.d.sel.contains(&mid) {
                                        self.d.sel.push(mid);
                                    }
                                }
                            } else {
                                self.d.sel = members;
                            }
                        }
                        let starts = self.d.sel.iter().filter_map(|i| self.d.doc.frame(idx, *i).map(|f| (*i, f.rect.x, f.rect.y))).collect();
                        self.drag = Some(Drag::Move { page: idx, grab: m, starts });
                    } else {
                        if !shift {
                            self.d.sel.clear();
                        }
                        self.drag = Some(Drag::Marquee { page: idx, start: m });
                    }
                } else {
                    self.drag = Some(Drag::Create { page: idx, start: m, tool: self.tool });
                }
            }
        }

        if let (Some(p), true) = (ptr, resp.dragged()) {
            match &self.drag {
                Some(Drag::Move { page, grab, starts }) => {
                    let m = to_mm(*page, p);
                    let (dx, dy) = (m.x - grab.x, m.y - grab.y);
                    for (id, x, y) in starts {
                        if let Some(f) = self.d.doc.frame_mut(*page, *id) {
                            f.rect.x = x + dx;
                            f.rect.y = y + dy;
                        }
                    }
                }
                Some(Drag::Resize { page, id }) => {
                    let m = to_mm(*page, p);
                    if let Some(f) = self.d.doc.frame_mut(*page, *id) {
                        let mut v = f.visual_rect();
                        v.w = (m.x - v.x).max(MIN_FRAME_MM);
                        v.h = (m.y - v.y).max(MIN_FRAME_MM);
                        f.set_visual_rect(v);
                    }
                }
                Some(Drag::Create { page, start, .. }) => {
                    let o = origin_of(*page);
                    let r = ERect::from_two_pos(o + start.to_vec2() * scale, p);
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, craft_theme::ACCENT_PUBLICRAFT), StrokeKind::Inside);
                }
                Some(Drag::Marquee { page, start }) => {
                    let o = origin_of(*page);
                    let r = ERect::from_two_pos(o + start.to_vec2() * scale, p);
                    painter.rect_filled(r, CornerRadius::ZERO, craft_theme::ACCENT_PUBLICRAFT.gamma_multiply(0.15));
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, craft_theme::ACCENT_PUBLICRAFT), StrokeKind::Inside);
                }
                None => {}
            }
        }

        if resp.drag_stopped() {
            match (self.drag.take(), ptr) {
                (Some(Drag::Create { page, start, tool }), Some(p)) => {
                    let end = to_mm(page, p);
                    let r = Rect { x: start.x.min(end.x), y: start.y.min(end.y), w: (start.x - end.x).abs(), h: (start.y - end.y).abs() };
                    if r.w >= MIN_FRAME_MM && r.h >= MIN_FRAME_MM {
                        self.create(tool, r, page);
                    }
                }
                (Some(Drag::Marquee { page, start }), Some(p)) => {
                    let end = to_mm(page, p);
                    let r = Rect { x: start.x.min(end.x), y: start.y.min(end.y), w: (start.x - end.x).abs(), h: (start.y - end.y).abs() };
                    let ids = self.d.doc.in_rect(page, &r);
                    for id in ids {
                        if !self.d.sel.contains(&id) {
                            self.d.sel.push(id);
                        }
                    }
                }
                _ => {}
            }
            self.drag = None;
            if let Some(before) = self.d.pending.take() {
                if before != self.d.doc {
                    self.push_undo(before);
                }
            }
        }

        if resp.clicked() {
            if let Some(p) = ptr {
                let (idx, m, _) = locate(p);
                if idx != MASTER && self.d.page != idx {
                    self.d.sel.clear();
                    self.d.page = idx;
                }
                let hit = self.d.doc.hit_test(idx, m.x, m.y);
                if let Some((style, size, color)) = self.fmt_painter {
                    // Copia formato: applica a una casella di testo e termina.
                    if let Some(id) = hit {
                        self.mutate(|doc, pg, _| {
                            if let Some(Frame { kind: FrameKind::Text { style: s, size_pt, color: c, .. }, .. }) = doc.frame_mut(pg, id) {
                                *s = style;
                                *size_pt = size;
                                *c = color;
                            }
                        });
                        self.fmt_painter = None;
                    }
                } else if self.tool == Tool::Select {
                    match hit {
                        Some(id) => {
                            let members = self.d.doc.with_group(idx, id);
                            if shift {
                                if self.d.sel.contains(&id) {
                                    self.d.sel.retain(|s| !members.contains(s));
                                } else {
                                    self.d.sel.extend(members);
                                }
                            } else {
                                self.d.sel = members;
                            }
                        }
                        None if !shift => self.d.sel.clear(),
                        None => {}
                    }
                } else {
                    let (w, h) = match self.tool {
                        Tool::Text => (60.0, 20.0),
                        _ => (40.0, 30.0),
                    };
                    self.create(self.tool, Rect { x: m.x, y: m.y, w, h }, idx);
                }
            }
        }
    }

    fn draw_rulers(&self, ui: &egui::Ui, outer: ERect, body: ERect, base: Pos2, scale: f32) {
        let bg = craft_theme::PANEL_RAISED;
        let fg = craft_theme::TEXT_DIM;
        let line = Stroke::new(1.0, craft_theme::BORDER);
        let p = ui.painter_at(outer);
        let hr = ERect::from_min_max(egui::pos2(body.min.x, outer.min.y), egui::pos2(body.max.x, body.min.y));
        let vr = ERect::from_min_max(egui::pos2(outer.min.x, body.min.y), egui::pos2(body.min.x, body.max.y));
        p.rect_filled(ERect::from_min_max(outer.min, body.min), CornerRadius::ZERO, bg);
        p.rect_filled(hr, CornerRadius::ZERO, bg);
        p.rect_filled(vr, CornerRadius::ZERO, bg);
        p.hline(hr.x_range(), hr.max.y, line);
        p.vline(vr.max.x, vr.y_range(), line);

        let label_every: i32 = [10, 20, 50, 100, 200, 500].into_iter().find(|m| *m as f32 * scale >= 44.0).unwrap_or(1000);
        let tick_step: i32 = if scale >= 3.0 { 1 } else if scale >= 1.2 { 5 } else { label_every };
        let font = FontId::proportional(10.0);

        let ticks = |lo: f32, hi: f32, zero: f32, mut draw: Box<dyn FnMut(f32, f32, Option<String>) + '_>| {
            let a = ((lo - zero) / scale).floor() as i32;
            let b = ((hi - zero) / scale).ceil() as i32;
            for mm in a..=b {
                if mm % tick_step != 0 {
                    continue;
                }
                let len = if mm % 10 == 0 { 1.0 } else if mm % 5 == 0 { 0.6 } else { 0.35 };
                let label = (mm % label_every == 0).then(|| format!("{}", mm / 10));
                draw(zero + mm as f32 * scale, len, label);
            }
        };
        let ph = p.with_clip_rect(hr);
        ticks(
            body.min.x,
            body.max.x,
            base.x,
            Box::new(|x, len, label| {
                ph.vline(x, (hr.max.y - RULER * 0.6 * len)..=hr.max.y, Stroke::new(1.0, fg));
                if let Some(t) = label {
                    ph.text(egui::pos2(x + 3.0, hr.min.y + 2.0), egui::Align2::LEFT_TOP, t, font.clone(), fg);
                }
            }),
        );
        let pv = p.with_clip_rect(vr);
        ticks(
            body.min.y,
            body.max.y,
            base.y,
            Box::new(|y, len, label| {
                pv.hline((vr.max.x - RULER * 0.6 * len)..=vr.max.x, y, Stroke::new(1.0, fg));
                if let Some(t) = label {
                    pv.text(egui::pos2(vr.min.x + 2.0, y + 2.0), egui::Align2::LEFT_TOP, t, font.clone(), fg);
                }
            }),
        );
        // Indicatore della posizione del puntatore.
        if let Some(h) = ui.input(|i| i.pointer.hover_pos()) {
            if body.contains(h) {
                ph.vline(h.x, hr.y_range(), Stroke::new(1.0, craft_theme::ACCENT_PUBLICRAFT));
                pv.hline(vr.x_range(), h.y, Stroke::new(1.0, craft_theme::ACCENT_PUBLICRAFT));
            }
        }
    }
}
