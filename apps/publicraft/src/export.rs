//! Disegno delle pagine fuori schermo con lo stesso motore della tela (egui + rasterizzatore software),
//! usato per l'esportazione in PDF/PNG e per la stampa.

use super::*;
use egui::epaint::{ClippedPrimitive, Primitive};
use egui::{Color32, TextureId};
use std::io::Write;

/// Immagine RGB di una pagina renderizzata.
pub(crate) struct PageImage {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
    pub w_mm: f32,
    pub h_mm: f32,
}

impl PageImage {
    /// Converte in scala di grigi (per la stampa in bianco e nero).
    pub fn to_gray(&mut self) {
        for px in self.rgb.chunks_exact_mut(3) {
            let g = (px[0] as f32 * 0.299 + px[1] as f32 * 0.587 + px[2] as f32 * 0.114).round() as u8;
            px.fill(g);
        }
    }
}

struct SoftTex {
    w: usize,
    h: usize,
    px: Vec<Color32>,
}

fn apply_deltas(texs: &mut HashMap<TextureId, SoftTex>, out: &egui::FullOutput) {
    for (id, deltas) in &out.textures_delta.set {
        for delta in deltas {
            let egui::ImageData::Color(img) = &delta.image;
            let [w, h] = img.size;
            match delta.pos {
                None => {
                    texs.insert(*id, SoftTex { w, h, px: img.pixels.clone() });
                }
                Some([x0, y0]) => {
                    if let Some(t) = texs.get_mut(id) {
                        for y in 0..h {
                            let dst = (y0 + y) * t.w + x0;
                            if dst + w <= t.px.len() {
                                t.px[dst..dst + w].copy_from_slice(&img.pixels[y * w..(y + 1) * w]);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn sample(t: &SoftTex, u: f32, v: f32) -> [f32; 4] {
    // Campionamento bilineare, come fa la scheda grafica.
    let x = (u * t.w as f32 - 0.5).clamp(0.0, t.w as f32 - 1.0);
    let y = (v * t.h as f32 - 0.5).clamp(0.0, t.h as f32 - 1.0);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(t.w - 1), (y0 + 1).min(t.h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |xx: usize, yy: usize| {
        let c = t.px[yy * t.w + xx];
        [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
    };
    let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
    let mut o = [0.0; 4];
    for i in 0..4 {
        let top = a[i] + (b[i] - a[i]) * fx;
        let bot = c[i] + (d[i] - c[i]) * fx;
        o[i] = top + (bot - top) * fy;
    }
    o
}

fn raster(buf: &mut [[f32; 3]], w: usize, h: usize, prims: &[ClippedPrimitive], texs: &HashMap<TextureId, SoftTex>) {
    for prim in prims {
        let Primitive::Mesh(mesh) = &prim.primitive else { continue };
        let Some(tex) = texs.get(&mesh.texture_id) else { continue };
        let clip = prim.clip_rect;
        for tri in mesh.indices.chunks_exact(3) {
            let (a, b, c) = (&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]);
            let area = (b.pos.x - a.pos.x) * (c.pos.y - a.pos.y) - (b.pos.y - a.pos.y) * (c.pos.x - a.pos.x);
            if area.abs() < 1e-6 {
                continue;
            }
            let min_x = a.pos.x.min(b.pos.x).min(c.pos.x).max(clip.min.x).max(0.0).floor() as usize;
            let min_y = a.pos.y.min(b.pos.y).min(c.pos.y).max(clip.min.y).max(0.0).floor() as usize;
            let max_x = (a.pos.x.max(b.pos.x).max(c.pos.x).min(clip.max.x).min(w as f32)).ceil() as usize;
            let max_y = (a.pos.y.max(b.pos.y).max(c.pos.y).min(clip.max.y).min(h as f32)).ceil() as usize;
            let inv = 1.0 / area;
            for y in min_y..max_y {
                let py = y as f32 + 0.5;
                for x in min_x..max_x {
                    let px = x as f32 + 0.5;
                    let w0 = ((b.pos.x - px) * (c.pos.y - py) - (b.pos.y - py) * (c.pos.x - px)) * inv;
                    let w1 = ((c.pos.x - px) * (a.pos.y - py) - (c.pos.y - py) * (a.pos.x - px)) * inv;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                        continue;
                    }
                    let u = a.uv.x * w0 + b.uv.x * w1 + c.uv.x * w2;
                    let v = a.uv.y * w0 + b.uv.y * w1 + c.uv.y * w2;
                    let t = sample(tex, u, v);
                    let ch = |f: fn(&egui::epaint::Vertex) -> u8| (f(a) as f32 * w0 + f(b) as f32 * w1 + f(c) as f32 * w2) / 255.0;
                    let (vr, vg, vb, va) = (ch(|q| q.color.r()), ch(|q| q.color.g()), ch(|q| q.color.b()), ch(|q| q.color.a()));
                    // Colori premoltiplicati: sorgente = texel × colore del vertice.
                    let (sr, sg, sb, sa) = (t[0] * vr, t[1] * vg, t[2] * vb, t[3] * va);
                    let keep = 1.0 - sa / 255.0;
                    let d = &mut buf[y * w + x];
                    d[0] = sr + d[0] * keep;
                    d[1] = sg + d[1] * keep;
                    d[2] = sb + d[2] * keep;
                }
            }
        }
    }
}

fn load_textures(ctx: &egui::Context, doc: &Document) -> HashMap<String, Option<Tex>> {
    let mut map: HashMap<String, Option<Tex>> = HashMap::new();
    for p in doc.pages.iter().chain(std::iter::once(&doc.master)) {
        for f in &p.frames {
            if let FrameKind::Picture { path } = &f.kind {
                map.entry(path.clone()).or_insert_with(|| {
                    image::open(path).ok().map(|img| {
                        let img = img.to_rgba8();
                        let size = [img.width() as usize, img.height() as usize];
                        let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                        Tex { handle: ctx.load_texture(path, ci, egui::TextureOptions::LINEAR), size }
                    })
                });
            }
        }
    }
    map
}

/// Disegna le pagine indicate (indici da 0) alla risoluzione `dpi`.
pub(crate) fn render_pages(doc: &Document, pages: &[usize], dpi: f32) -> Vec<PageImage> {
    let ctx = egui::Context::default();
    fonts::install(&ctx);
    let textures = load_textures(&ctx, doc);
    let scale = dpi / 25.4;
    let (w_mm, h_mm) = (doc.page_size.w_mm, doc.page_size.h_mm);
    let (w, h) = ((w_mm * scale).round().max(1.0) as usize, (h_mm * scale).round().max(1.0) as usize);
    let mut texs: HashMap<TextureId, SoftTex> = HashMap::new();
    let mut out = Vec::new();
    for &idx in pages {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(w as f32, h as f32))), ..Default::default() };
        // Due passaggi: il primo prepara l'atlante dei caratteri, il secondo disegna con tutti i glifi pronti.
        let mut full = None;
        for _ in 0..2 {
            let o = ctx.run_ui(input.clone(), |ui| {
                canvas::paint_page_to(ui.painter(), doc, idx, egui::Pos2::ZERO, scale, &textures);
            });
            apply_deltas(&mut texs, &o);
            full = Some(o);
        }
        let full = full.expect("almeno un passaggio");
        let prims = ctx.tessellate(full.shapes, 1.0);
        let mut buf = vec![[255.0f32; 3]; w * h];
        raster(&mut buf, w, h, &prims, &texs);
        let mut rgb = Vec::with_capacity(w * h * 3);
        for p in &buf {
            rgb.extend(p.iter().map(|v| v.clamp(0.0, 255.0).round() as u8));
        }
        out.push(PageImage { w, h, rgb, w_mm, h_mm });
    }
    out
}

/// Elenco di pagine da testo come «1-3, 5»; indici da 0, fuori intervallo scartati.
pub(crate) fn parse_range(text: &str, count: usize) -> Option<Vec<usize>> {
    let mut v = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<usize>().ok()?, b.trim().parse::<usize>().ok()?),
            None => {
                let n = part.parse::<usize>().ok()?;
                (n, n)
            }
        };
        if a == 0 || b < a {
            return None;
        }
        for n in a..=b.min(count) {
            if !v.contains(&(n - 1)) {
                v.push(n - 1);
            }
        }
    }
    if v.is_empty() { None } else { Some(v) }
}

fn pdf_hex_string(s: &str) -> String {
    let mut out = String::from("<FEFF");
    for u in s.encode_utf16() {
        out.push_str(&format!("{u:04X}"));
    }
    out.push('>');
    out
}

/// Scrive un PDF con una pagina per immagine (dimensioni reali della pagina).
pub(crate) fn write_pdf(path: &std::path::Path, title: &str, pages: &[PageImage]) -> Result<(), String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let n = pages.len();
    let obj = |buf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(buf.len());
        let id = offsets.len();
        buf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    };
    // 1: catalogo, 2: elenco pagine, 3: informazioni; poi per ogni pagina: pagina, contenuto, immagine.
    obj(&mut buf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 3 * i)).collect();
    obj(&mut buf, &mut offsets, format!("<< /Type /Pages /Count {n} /Kids [{}] >>", kids.join(" ")).as_bytes());
    obj(&mut buf, &mut offsets, format!("<< /Title {} /Producer (PubliCraft {}) >>", pdf_hex_string(title), env!("CARGO_PKG_VERSION")).as_bytes());
    for (i, pg) in pages.iter().enumerate() {
        let (wpt, hpt) = (pg.w_mm * 72.0 / 25.4, pg.h_mm * 72.0 / 25.4);
        let page_id = 4 + 3 * i;
        obj(
            &mut buf,
            &mut offsets,
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {wpt:.2} {hpt:.2}] /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>", page_id + 2, page_id + 1).as_bytes(),
        );
        let content = format!("q {wpt:.2} 0 0 {hpt:.2} 0 0 cm /Im0 Do Q");
        obj(&mut buf, &mut offsets, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).as_bytes());
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
        z.write_all(&pg.rgb).map_err(|e| e.to_string())?;
        let data = z.finish().map_err(|e| e.to_string())?;
        let mut body = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
            pg.w,
            pg.h,
            data.len()
        )
        .into_bytes();
        body.extend_from_slice(&data);
        body.extend_from_slice(b"\nendstream");
        obj(&mut buf, &mut offsets, &body);
    }
    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
    std::fs::write(path, buf).map_err(|e| e.to_string())
}

pub(crate) fn write_png(path: &std::path::Path, pg: &PageImage) -> Result<(), String> {
    let img = image::RgbImage::from_raw(pg.w as u32, pg.h as u32, pg.rgb.clone()).ok_or("immagine non valida")?;
    img.save(path).map_err(|e| e.to_string())
}
