//! Importazione di un PDF come documento modificabile: il testo diventa riquadri di testo,
//! le immagini diventano immagini, i rettangoli pieni diventano rettangoli, la grafica vettoriale
//! (loghi, linee) viene disegnata in immagini e i campi dei moduli diventano riquadri di testo.

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document as Pdf, Object, Stream};
use publicraft_doc::{Align, Document, Family, FrameKind, PageSize, Rect, Rgb, TextStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const PT_MM: f64 = 25.4 / 72.0;
const RASTER_DPI: f64 = 200.0;

// ---------------------------------------------------------------- matrici

type M = [f64; 6];
const ID: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mul(a: &M, b: &M) -> M {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn app(m: &M, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn num(o: &Object) -> Option<f64> {
    o.as_float().ok().map(|v| v as f64)
}

fn nums(o: &[Object]) -> Vec<f64> {
    o.iter().filter_map(num).collect()
}

fn matrix_of(o: &[Object]) -> Option<M> {
    let v = nums(o);
    if v.len() == 6 { Some([v[0], v[1], v[2], v[3], v[4], v[5]]) } else { None }
}

// ---------------------------------------------------------------- caratteri e font

const WIN_80: [char; 32] = [
    '€', ' ', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', ' ', 'Ž', ' ', ' ', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', ' ', 'ž', 'Ÿ',
];

const GLYPHS: &[(&str, char)] = &[
    ("space", ' '), ("exclam", '!'), ("quotedbl", '"'), ("numbersign", '#'), ("dollar", '$'), ("percent", '%'), ("ampersand", '&'), ("quotesingle", '\''),
    ("parenleft", '('), ("parenright", ')'), ("asterisk", '*'), ("plus", '+'), ("comma", ','), ("hyphen", '-'), ("period", '.'), ("slash", '/'),
    ("zero", '0'), ("one", '1'), ("two", '2'), ("three", '3'), ("four", '4'), ("five", '5'), ("six", '6'), ("seven", '7'), ("eight", '8'), ("nine", '9'),
    ("colon", ':'), ("semicolon", ';'), ("less", '<'), ("equal", '='), ("greater", '>'), ("question", '?'), ("at", '@'), ("bracketleft", '['),
    ("backslash", '\\'), ("bracketright", ']'), ("asciicircum", '^'), ("underscore", '_'), ("grave", '`'), ("braceleft", '{'), ("bar", '|'),
    ("braceright", '}'), ("asciitilde", '~'), ("endash", '–'), ("emdash", '—'), ("quoteleft", '‘'), ("quoteright", '’'), ("quotedblleft", '“'),
    ("quotedblright", '”'), ("bullet", '•'), ("ellipsis", '…'), ("fi", 'ﬁ'), ("fl", 'ﬂ'), ("Euro", '€'), ("degree", '°'), ("eacute", 'é'),
    ("egrave", 'è'), ("agrave", 'à'), ("ugrave", 'ù'), ("ograve", 'ò'), ("igrave", 'ì'), ("aacute", 'á'), ("iacute", 'í'), ("oacute", 'ó'),
    ("uacute", 'ú'), ("ccedilla", 'ç'), ("ntilde", 'ñ'), ("udieresis", 'ü'), ("odieresis", 'ö'), ("adieresis", 'ä'), ("germandbls", 'ß'),
    ("Eacute", 'É'), ("Agrave", 'À'), ("copyright", '©'), ("registered", '®'), ("trademark", '™'), ("section", '§'), ("sterling", '£'),
    ("yen", '¥'), ("cent", '¢'), ("multiply", '×'), ("divide", '÷'), ("plusminus", '±'), ("guillemotleft", '«'), ("guillemotright", '»'),
    ("periodcentered", '·'), ("minus", '−'),
];

fn glyph_char(name: &str) -> Option<char> {
    if let Some(h) = name.strip_prefix("uni") {
        return u32::from_str_radix(h.get(..4)?, 16).ok().and_then(char::from_u32);
    }
    if name.chars().count() == 1 {
        return name.chars().next();
    }
    GLYPHS.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

struct Font {
    two_byte: bool,
    cmap: HashMap<u32, String>,
    table: Vec<Option<char>>,
    widths: HashMap<u32, f64>,
    dw: f64,
    bold: bool,
    italic: bool,
    family: Family,
}

impl Font {
    fn decode(&self, code: u32) -> String {
        if let Some(s) = self.cmap.get(&code) {
            return s.clone();
        }
        if self.two_byte {
            return String::new();
        }
        self.table.get(code as usize).copied().flatten().map(|c| c.to_string()).unwrap_or_default()
    }

    fn width(&self, code: u32) -> f64 {
        self.widths.get(&code).copied().unwrap_or(self.dw) / 1000.0
    }
}

fn hex_tokens(data: &[u8]) -> Vec<String> {
    // Parole, stringhe esadecimali `<…>` e parentesi quadre.
    let s = String::from_utf8_lossy(data);
    let ch: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < ch.len() {
        let c = ch[i];
        if c == '<' {
            if ch.get(i + 1) == Some(&'<') {
                i += 2;
                continue;
            }
            let mut j = i + 1;
            let mut h = String::from("<");
            while j < ch.len() && ch[j] != '>' {
                if ch[j].is_ascii_hexdigit() {
                    h.push(ch[j]);
                }
                j += 1;
            }
            out.push(h);
            i = j + 1;
        } else if c == '[' || c == ']' {
            out.push(c.to_string());
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '%' {
            while i < ch.len() && ch[i] != '\n' {
                i += 1;
            }
        } else {
            let mut j = i;
            while j < ch.len() && !ch[j].is_whitespace() && !matches!(ch[j], '<' | '[' | ']') {
                j += 1;
            }
            out.push(ch[i..j].iter().collect());
            i = j.max(i + 1);
        }
    }
    out
}

fn hex_val(t: &str) -> Option<u32> {
    u32::from_str_radix(t.strip_prefix('<')?, 16).ok()
}

fn hex_utf16(t: &str) -> String {
    let h = t.trim_start_matches('<');
    let units: Vec<u16> = (0..h.len() / 4).filter_map(|i| u16::from_str_radix(&h[i * 4..i * 4 + 4], 16).ok()).collect();
    if units.is_empty() && h.len() == 2 {
        // Destinazione a un byte.
        return u8::from_str_radix(h, 16).ok().map(|b| (b as char).to_string()).unwrap_or_default();
    }
    String::from_utf16_lossy(&units)
}

fn parse_cmap(data: &[u8]) -> (HashMap<u32, String>, Option<usize>) {
    let t = hex_tokens(data);
    let mut map = HashMap::new();
    let mut code_len = None;
    let mut i = 0;
    while i < t.len() {
        match t[i].as_str() {
            "begincodespacerange" => {
                if let Some(a) = t.get(i + 1) {
                    code_len = Some((a.len() - 1).div_ceil(2));
                }
                i += 1;
            }
            "beginbfchar" => {
                i += 1;
                while i + 1 < t.len() && t[i] != "endbfchar" {
                    if let Some(src) = hex_val(&t[i]) {
                        map.insert(src, hex_utf16(&t[i + 1]));
                    }
                    i += 2;
                }
            }
            "beginbfrange" => {
                i += 1;
                while i + 2 < t.len() && t[i] != "endbfrange" {
                    let (Some(lo), Some(hi)) = (hex_val(&t[i]), hex_val(&t[i + 1])) else {
                        i += 1;
                        continue;
                    };
                    if t[i + 2] == "[" {
                        let mut j = i + 3;
                        let mut c = lo;
                        while j < t.len() && t[j] != "]" {
                            map.insert(c, hex_utf16(&t[j]));
                            c += 1;
                            j += 1;
                        }
                        i = j + 1;
                    } else {
                        let base: Vec<u16> = {
                            let h = t[i + 2].trim_start_matches('<');
                            (0..h.len() / 4).filter_map(|k| u16::from_str_radix(&h[k * 4..k * 4 + 4], 16).ok()).collect()
                        };
                        for (n, c) in (lo..=hi.min(lo + 65535)).enumerate() {
                            let mut u = base.clone();
                            if let Some(last) = u.last_mut() {
                                *last = last.wrapping_add(n as u16);
                            }
                            map.insert(c, String::from_utf16_lossy(&u));
                        }
                        i += 3;
                    }
                }
            }
            _ => i += 1,
        }
    }
    (map, code_len)
}

fn deref<'a>(pdf: &'a Pdf, o: &'a Object) -> &'a Object {
    pdf.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

fn dict_of<'a>(pdf: &'a Pdf, o: &'a Object) -> Option<&'a Dictionary> {
    match deref(pdf, o) {
        Object::Dictionary(d) => Some(d),
        Object::Stream(s) => Some(&s.dict),
        _ => None,
    }
}

fn family_for(name: &str, serif_flag: bool) -> Family {
    let n = name.to_lowercase();
    if n.contains("courier") {
        Family::Courier
    } else if n.contains("consolas") || n.contains("mono") {
        Family::Consolas
    } else if n.contains("georgia") {
        Family::Georgia
    } else if n.contains("times") || n.contains("garamond") || n.contains("minion") || n.contains("cambria") || n.contains("palatino") || n.contains("serif") && !n.contains("sans") {
        Family::Times
    } else if n.contains("calibri") || n.contains("carlito") {
        Family::Calibri
    } else if n.contains("verdana") {
        Family::Verdana
    } else if n.contains("tahoma") {
        Family::Tahoma
    } else if n.contains("arial") || n.contains("helvetica") || n.contains("liberation") {
        Family::Arial
    } else if serif_flag {
        Family::Times
    } else {
        Family::Sans
    }
}

fn load_font(pdf: &Pdf, d: &Dictionary) -> Font {
    let subtype = d.get(b"Subtype").ok().and_then(|o| o.as_name().ok()).unwrap_or(b"");
    let two_byte = subtype == b"Type0";
    let desc_font: Option<&Dictionary> = if two_byte {
        d.get_deref(b"DescendantFonts", pdf).ok().and_then(|o| o.as_array().ok()).and_then(|a| a.first()).and_then(|o| dict_of(pdf, o))
    } else {
        None
    };
    let base = String::from_utf8_lossy(d.get(b"BaseFont").ok().and_then(|o| o.as_name().ok()).unwrap_or(b"")).to_string();
    let base = base.split('+').next_back().unwrap_or("").to_string();
    let descriptor = desc_font.unwrap_or(d).get_deref(b"FontDescriptor", pdf).ok().and_then(|o| dict_of(pdf, o));
    let flags = descriptor.and_then(|f| f.get(b"Flags").ok()).and_then(|o| o.as_i64().ok()).unwrap_or(0);
    let lname = base.to_lowercase();
    let weight = descriptor.and_then(|f| f.get(b"FontWeight").ok()).and_then(num).unwrap_or(400.0);
    let bold = ["bold", "black", "heavy", "semibold", "demi", "extrabold"].iter().any(|k| lname.contains(k)) || flags & 0x40000 != 0 || weight >= 600.0;
    let italic = lname.contains("italic") || lname.contains("oblique") || flags & 0x40 != 0 || descriptor.and_then(|f| f.get(b"ItalicAngle").ok()).and_then(num).unwrap_or(0.0).abs() > 1.0;
    let family = family_for(&base, flags & 0x2 != 0);

    // Tabella dei codici → caratteri per i font a un byte.
    let mut table: Vec<Option<char>> = (0..256u32).map(|c| if (32..127).contains(&c) || c >= 160 { char::from_u32(c) } else { None }).collect();
    let enc = d.get_deref(b"Encoding", pdf).ok();
    let enc_name: Option<String> = match enc {
        Some(Object::Name(n)) => Some(String::from_utf8_lossy(n).to_string()),
        Some(Object::Dictionary(ed)) => ed.get(b"BaseEncoding").ok().and_then(|o| o.as_name().ok()).map(|n| String::from_utf8_lossy(n).to_string()),
        _ => None,
    };
    match enc_name.as_deref() {
        Some("WinAnsiEncoding") => {
            for (i, c) in WIN_80.iter().enumerate() {
                table[0x80 + i] = if *c == ' ' { None } else { Some(*c) };
            }
        }
        Some("MacRomanEncoding") => {
            for c in 128..256usize {
                let bytes = [c as u8];
                let (s, _) = encoding_rs::MACINTOSH.decode_without_bom_handling(&bytes);
                table[c] = s.chars().next();
            }
        }
        _ => {}
    }
    if let Some(Object::Dictionary(ed)) = enc {
        if let Ok(Object::Array(diff)) = ed.get_deref(b"Differences", pdf) {
            let mut code = 0usize;
            for o in diff {
                match deref(pdf, o) {
                    Object::Integer(n) => code = *n as usize,
                    Object::Name(n) => {
                        if code < 256 {
                            table[code] = glyph_char(&String::from_utf8_lossy(n));
                        }
                        code += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    // ToUnicode ha la precedenza quando c'è.
    let mut cmap = HashMap::new();
    let mut two_byte_final = two_byte;
    if let Ok(Object::Stream(s)) = d.get_deref(b"ToUnicode", pdf) {
        if let Ok(data) = s.decompressed_content() {
            let (m, len) = parse_cmap(&data);
            cmap = m;
            if two_byte && len == Some(1) {
                two_byte_final = false;
            }
        }
    }

    // Larghezze dei glifi.
    let mut widths = HashMap::new();
    let dw;
    if two_byte {
        let df = desc_font;
        dw = df.and_then(|f| f.get(b"DW").ok()).and_then(num).unwrap_or(1000.0);
        if let Some(Ok(Object::Array(w))) = df.map(|f| f.get_deref(b"W", pdf)) {
            let mut i = 0;
            while i < w.len() {
                let Some(c) = num(deref(pdf, &w[i])) else { break };
                match w.get(i + 1).map(|o| deref(pdf, o)) {
                    Some(Object::Array(list)) => {
                        for (k, v) in list.iter().enumerate() {
                            if let Some(x) = num(deref(pdf, v)) {
                                widths.insert(c as u32 + k as u32, x);
                            }
                        }
                        i += 2;
                    }
                    Some(o2) => {
                        let (Some(c2), Some(x)) = (num(o2), w.get(i + 2).and_then(|o| num(deref(pdf, o)))) else { break };
                        for cc in c as u32..=(c2 as u32).min(c as u32 + 65535) {
                            widths.insert(cc, x);
                        }
                        i += 3;
                    }
                    None => break,
                }
            }
        }
    } else {
        let first = d.get(b"FirstChar").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0) as u32;
        if let Ok(Object::Array(w)) = d.get_deref(b"Widths", pdf) {
            for (k, v) in w.iter().enumerate() {
                if let Some(x) = num(deref(pdf, v)) {
                    widths.insert(first + k as u32, x);
                }
            }
        }
        dw = descriptor.and_then(|f| f.get(b"MissingWidth").ok()).and_then(num).unwrap_or(500.0);
    }
    Font { two_byte: two_byte_final, cmap, table, widths, dw, bold, italic, family }
}

// ---------------------------------------------------------------- elementi raccolti

#[derive(Clone)]
struct Glyph {
    ch: String,
    x: f64,
    y: f64,
    size: f64,
    w: f64,
    angle: f64,
    bold: bool,
    italic: bool,
    family: Family,
    color: [u8; 3],
    order: usize,
}

struct RectItem {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    fill: [u8; 3],
    stroke: Option<([u8; 3], f64)>,
    order: usize,
}

struct PathItem {
    polys: Vec<Vec<(f64, f64)>>,
    fill: Option<([u8; 3], bool)>,
    stroke: Option<([u8; 3], f64)>,
    order: usize,
}

struct ImgItem {
    path: PathBuf,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    order: usize,
}

#[derive(Default)]
struct Out {
    glyphs: Vec<Glyph>,
    rects: Vec<RectItem>,
    paths: Vec<PathItem>,
    images: Vec<ImgItem>,
    order: usize,
    rotated: usize,
    hidden: usize,
    img_n: usize,
}

#[derive(Clone)]
struct Gs {
    ctm: M,
    fill: [u8; 3],
    stroke: [u8; 3],
    width: f64,
}

#[derive(Clone)]
struct Ts {
    tm: M,
    tlm: M,
    font: Option<Rc<Font>>,
    size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    tl: f64,
    rise: f64,
    mode: i64,
}

fn cmyk(c: f64, m: f64, y: f64, k: f64) -> [u8; 3] {
    let f = |v: f64| (255.0 * (1.0 - v.clamp(0.0, 1.0)) * (1.0 - k.clamp(0.0, 1.0))).round() as u8;
    [f(c), f(m), f(y)]
}

fn gray(g: f64) -> [u8; 3] {
    let v = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
    [v, v, v]
}

fn color_from(v: &[f64]) -> Option<[u8; 3]> {
    match v.len() {
        1 => Some(gray(v[0])),
        3 => Some([(v[0].clamp(0.0, 1.0) * 255.0).round() as u8, (v[1].clamp(0.0, 1.0) * 255.0).round() as u8, (v[2].clamp(0.0, 1.0) * 255.0).round() as u8]),
        4 => Some(cmyk(v[0], v[1], v[2], v[3])),
        _ => None,
    }
}

fn flatten_cubic(out: &mut Vec<(f64, f64)>, p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
    const N: usize = 14;
    for i in 1..=N {
        let t = i as f64 / N as f64;
        let u = 1.0 - t;
        let x = u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0;
        let y = u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1;
        out.push((x, y));
    }
}

/// Se i punti formano un rettangolo con i lati paralleli agli assi: (x0, y0, x1, y1).
fn axis_rect(sp: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    let mut p = sp.to_vec();
    if p.len() == 5 && (p[0].0 - p[4].0).abs() < 1e-6 && (p[0].1 - p[4].1).abs() < 1e-6 {
        p.pop();
    }
    if p.len() != 4 {
        return None;
    }
    let e = 1e-4;
    let ok = (0..4).all(|i| {
        let (a, b) = (p[i], p[(i + 1) % 4]);
        (a.0 - b.0).abs() < e || (a.1 - b.1).abs() < e
    });
    if !ok {
        return None;
    }
    let xs = [p[0].0, p[1].0, p[2].0, p[3].0];
    let ys = [p[0].1, p[1].1, p[2].1, p[3].1];
    let (x0, x1) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
    let (y0, y1) = (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max));
    Some((x0, y0, x1, y1))
}

// ---------------------------------------------------------------- immagini

enum Cs {
    Gray,
    Rgb,
    Cmyk,
    Indexed { base: Box<Cs>, palette: Vec<u8> },
}

impl Cs {
    fn comps(&self) -> usize {
        match self {
            Cs::Gray | Cs::Indexed { .. } => 1,
            Cs::Rgb => 3,
            Cs::Cmyk => 4,
        }
    }

    fn rgb(&self, s: &[u8]) -> [u8; 3] {
        match self {
            Cs::Gray => [s[0], s[0], s[0]],
            Cs::Rgb => [s[0], s[1], s[2]],
            Cs::Cmyk => cmyk(s[0] as f64 / 255.0, s[1] as f64 / 255.0, s[2] as f64 / 255.0, s[3] as f64 / 255.0),
            Cs::Indexed { base, palette } => {
                let n = base.comps();
                let i = s[0] as usize * n;
                if i + n <= palette.len() { base.rgb(&palette[i..i + n]) } else { [0, 0, 0] }
            }
        }
    }
}

fn parse_cs(pdf: &Pdf, o: &Object, depth: u8) -> Cs {
    if depth > 3 {
        return Cs::Gray;
    }
    match deref(pdf, o) {
        Object::Name(n) => match n.as_slice() {
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Cs::Rgb,
            b"DeviceCMYK" | b"CMYK" => Cs::Cmyk,
            _ => Cs::Gray,
        },
        Object::Array(a) if !a.is_empty() => {
            let kind = deref(pdf, &a[0]).as_name().unwrap_or(b"");
            match kind {
                b"ICCBased" => {
                    let n = a.get(1).and_then(|o| dict_of(pdf, o)).and_then(|d| d.get(b"N").ok()).and_then(|o| o.as_i64().ok()).unwrap_or(3);
                    match n {
                        1 => Cs::Gray,
                        4 => Cs::Cmyk,
                        _ => Cs::Rgb,
                    }
                }
                b"Indexed" | b"I" => {
                    let base = a.get(1).map(|o| parse_cs(pdf, o, depth + 1)).unwrap_or(Cs::Rgb);
                    let palette = match a.get(3).map(|o| deref(pdf, o)) {
                        Some(Object::String(s, _)) => s.clone(),
                        Some(Object::Stream(s)) => s.decompressed_content().unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    Cs::Indexed { base: Box::new(base), palette }
                }
                b"CalRGB" => Cs::Rgb,
                _ => Cs::Gray,
            }
        }
        _ => Cs::Gray,
    }
}

fn filter_names(pdf: &Pdf, d: &Dictionary) -> Vec<String> {
    match d.get_deref(b"Filter", pdf) {
        Ok(Object::Name(n)) => vec![String::from_utf8_lossy(n).to_string()],
        Ok(Object::Array(a)) => a.iter().filter_map(|o| deref(pdf, o).as_name().ok()).map(|n| String::from_utf8_lossy(n).to_string()).collect(),
        _ => Vec::new(),
    }
}

fn decode_image(pdf: &Pdf, st: &Stream, fill: [u8; 3], depth: u8) -> Option<image::RgbaImage> {
    let d = &st.dict;
    let w = d.get_deref(b"Width", pdf).ok()?.as_i64().ok()? as u32;
    let h = d.get_deref(b"Height", pdf).ok()?.as_i64().ok()? as u32;
    if w == 0 || h == 0 || (w as u64) * (h as u64) > 120_000_000 {
        return None;
    }
    let filters = filter_names(pdf, d);
    let is_mask = matches!(d.get_deref(b"ImageMask", pdf), Ok(Object::Boolean(true)));
    let mut img = if filters.last().map(String::as_str) == Some("DCTDecode") {
        image::load_from_memory_with_format(&st.content, image::ImageFormat::Jpeg).ok()?.to_rgba8()
    } else if filters.iter().any(|f| matches!(f.as_str(), "JPXDecode" | "CCITTFaxDecode" | "JBIG2Decode")) {
        return None;
    } else {
        let raw = st.decompressed_content().ok()?;
        let bpc = if is_mask { 1 } else { d.get_deref(b"BitsPerComponent", pdf).ok().and_then(|o| o.as_i64().ok()).unwrap_or(8) as usize };
        let mut out = image::RgbaImage::new(w, h);
        if is_mask {
            let invert = matches!(d.get_deref(b"Decode", pdf), Ok(Object::Array(a)) if nums(a).first() == Some(&1.0));
            let row = (w as usize).div_ceil(8);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let bit = raw.get(y * row + x / 8).map(|b| (b >> (7 - x % 8)) & 1).unwrap_or(1);
                    let paint = if invert { bit == 1 } else { bit == 0 };
                    out.put_pixel(x as u32, y as u32, image::Rgba([fill[0], fill[1], fill[2], if paint { 255 } else { 0 }]));
                }
            }
        } else {
            let cs = d.get(b"ColorSpace").map(|o| parse_cs(pdf, o, 0)).unwrap_or(Cs::Gray);
            let n = cs.comps();
            if !matches!(bpc, 1 | 2 | 4 | 8 | 16) {
                return None;
            }
            let row = (w as usize * n * bpc).div_ceil(8);
            let maxv = ((1u32 << bpc.min(8)) - 1) as f64;
            let idx = matches!(cs, Cs::Indexed { .. });
            let mut comp = vec![0u8; n];
            for y in 0..h as usize {
                for x in 0..w as usize {
                    for (c, slot) in comp.iter_mut().enumerate() {
                        let v = match bpc {
                            8 => raw.get(y * row + x * n + c).copied().unwrap_or(0) as u32,
                            16 => raw.get(y * row + (x * n + c) * 2).copied().unwrap_or(0) as u32,
                            _ => {
                                let bit = (x * n + c) * bpc;
                                let byte = raw.get(y * row + bit / 8).copied().unwrap_or(0) as u32;
                                (byte >> (8 - bpc - bit % 8)) & ((1 << bpc) - 1)
                            }
                        };
                        *slot = if bpc == 16 || idx { v as u8 } else { (v as f64 * 255.0 / maxv).round() as u8 };
                    }
                    let c = cs.rgb(&comp);
                    out.put_pixel(x as u32, y as u32, image::Rgba([c[0], c[1], c[2], 255]));
                }
            }
        }
        out
    };
    if img.width() != w || img.height() != h {
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    }
    if depth == 0 {
        if let Ok(Object::Stream(sm)) = d.get_deref(b"SMask", pdf) {
            if let Some(mask) = decode_image(pdf, sm, fill, 1) {
                let mask = if mask.dimensions() != img.dimensions() { image::imageops::resize(&mask, img.width(), img.height(), image::imageops::FilterType::Triangle) } else { mask };
                for (p, m) in img.pixels_mut().zip(mask.pixels()) {
                    p.0[3] = m.0[0];
                }
            }
        }
    }
    Some(img)
}

// ---------------------------------------------------------------- interprete

struct Interp<'a> {
    pdf: &'a Pdf,
    out: Out,
    img_dir: PathBuf,
    page_no: usize,
    fonts: HashMap<String, Rc<Font>>,
    gs: Gs,
    stack: Vec<Gs>,
    ts: Ts,
    path: Vec<Vec<(f64, f64)>>,
    cur: (f64, f64),
}

impl<'a> Interp<'a> {
    fn new(pdf: &'a Pdf, img_dir: PathBuf, page_no: usize) -> Self {
        Self {
            pdf,
            out: Out::default(),
            img_dir,
            page_no,
            fonts: HashMap::new(),
            gs: Gs { ctm: ID, fill: [0, 0, 0], stroke: [0, 0, 0], width: 1.0 },
            stack: Vec::new(),
            ts: Ts { tm: ID, tlm: ID, font: None, size: 12.0, tc: 0.0, tw: 0.0, th: 1.0, tl: 0.0, rise: 0.0, mode: 0 },
            path: Vec::new(),
            cur: (0.0, 0.0),
        }
    }

    fn next_order(&mut self) -> usize {
        self.out.order += 1;
        self.out.order
    }

    fn font_for(&mut self, res: Option<&'a Dictionary>, name: &[u8], depth: u32) -> Option<Rc<Font>> {
        let fonts = res?.get_deref(b"Font", self.pdf).ok().and_then(|o| dict_of(self.pdf, o))?;
        let entry = fonts.get(name).ok()?;
        let key = match entry {
            Object::Reference(id) => format!("r{}_{}", id.0, id.1),
            _ => format!("d{depth}_{}", String::from_utf8_lossy(name)),
        };
        if let Some(f) = self.fonts.get(&key) {
            return Some(f.clone());
        }
        let d = dict_of(self.pdf, entry)?;
        let f = Rc::new(load_font(self.pdf, d));
        self.fonts.insert(key, f.clone());
        Some(f)
    }

    fn paint(&mut self, fill: bool, stroke: bool, evenodd: bool) {
        let polys: Vec<Vec<(f64, f64)>> = std::mem::take(&mut self.path).into_iter().filter(|p| p.len() >= 2).collect();
        if polys.is_empty() || !(fill || stroke) {
            return;
        }
        let order = self.next_order();
        if fill && polys.len() == 1 {
            if let Some((x0, y0, x1, y1)) = axis_rect(&polys[0]) {
                let sw = self.gs.width * (self.gs.ctm[0].hypot(self.gs.ctm[1]));
                self.out.rects.push(RectItem { x0, y0, x1, y1, fill: self.gs.fill, stroke: if stroke { Some((self.gs.stroke, sw)) } else { None }, order });
                return;
            }
        }
        let sw = self.gs.width * self.gs.ctm[0].hypot(self.gs.ctm[1]);
        self.out.paths.push(PathItem { polys, fill: if fill { Some((self.gs.fill, evenodd)) } else { None }, stroke: if stroke { Some((self.gs.stroke, sw)) } else { None }, order });
    }

    fn show(&mut self, bytes: &[u8]) {
        let Some(font) = self.ts.font.clone() else { return };
        let step = if font.two_byte { 2 } else { 1 };
        for chunk in bytes.chunks(step) {
            let code = chunk.iter().fold(0u32, |a, b| (a << 8) | *b as u32);
            let ch = font.decode(code);
            let w = font.width(code);
            let fs = self.ts.size;
            let tmctm = mul(&self.ts.tm, &self.gs.ctm);
            let trm = mul(&[fs * self.ts.th, 0.0, 0.0, fs, 0.0, self.ts.rise], &tmctm);
            if self.ts.mode == 3 {
                if !ch.trim().is_empty() {
                    self.out.hidden += 1;
                }
            } else if !ch.is_empty() {
                let angle = trm[1].atan2(trm[0]);
                let size = fs * tmctm[2].hypot(tmctm[3]);
                if ch.trim().is_empty() || angle.abs() < 0.12 {
                    let order = self.next_order();
                    self.out.glyphs.push(Glyph {
                        ch,
                        x: trm[4],
                        y: trm[5],
                        size,
                        w: w * fs * self.ts.th * tmctm[0].hypot(tmctm[1]),
                        angle,
                        bold: font.bold,
                        italic: font.italic,
                        family: font.family,
                        color: self.gs.fill,
                        order,
                    });
                } else {
                    self.out.rotated += 1;
                }
            }
            let word = if code == 32 && !font.two_byte { self.ts.tw } else { 0.0 };
            let tx = (w * fs + self.ts.tc + word) * self.ts.th;
            self.ts.tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &self.ts.tm);
        }
    }

    fn run(&mut self, ops: &[Operation], res: Option<&'a Dictionary>, depth: u32) {
        let pdf = self.pdf;
        for op in ops {
            let a = &op.operands;
            let n = |i: usize| a.get(i).and_then(num).unwrap_or(0.0);
            match op.operator.as_str() {
                "q" => self.stack.push(self.gs.clone()),
                "Q" => {
                    if let Some(g) = self.stack.pop() {
                        self.gs = g;
                    }
                }
                "cm" => {
                    if let Some(m) = matrix_of(a) {
                        self.gs.ctm = mul(&m, &self.gs.ctm);
                    }
                }
                "w" => self.gs.width = n(0),
                "g" => self.gs.fill = gray(n(0)),
                "G" => self.gs.stroke = gray(n(0)),
                "rg" => self.gs.fill = color_from(&[n(0), n(1), n(2)]).unwrap_or(self.gs.fill),
                "RG" => self.gs.stroke = color_from(&[n(0), n(1), n(2)]).unwrap_or(self.gs.stroke),
                "k" => self.gs.fill = cmyk(n(0), n(1), n(2), n(3)),
                "K" => self.gs.stroke = cmyk(n(0), n(1), n(2), n(3)),
                "sc" | "scn" => {
                    if let Some(c) = color_from(&nums(a)) {
                        self.gs.fill = c;
                    }
                }
                "SC" | "SCN" => {
                    if let Some(c) = color_from(&nums(a)) {
                        self.gs.stroke = c;
                    }
                }
                "m" => {
                    let p = app(&self.gs.ctm, n(0), n(1));
                    self.path.push(vec![p]);
                    self.cur = p;
                }
                "l" => {
                    let p = app(&self.gs.ctm, n(0), n(1));
                    if let Some(sp) = self.path.last_mut() {
                        sp.push(p);
                    }
                    self.cur = p;
                }
                "c" | "v" | "y" => {
                    let (p1, p2, p3) = match op.operator.as_str() {
                        "c" => (app(&self.gs.ctm, n(0), n(1)), app(&self.gs.ctm, n(2), n(3)), app(&self.gs.ctm, n(4), n(5))),
                        "v" => (self.cur, app(&self.gs.ctm, n(0), n(1)), app(&self.gs.ctm, n(2), n(3))),
                        _ => {
                            let p3 = app(&self.gs.ctm, n(2), n(3));
                            (app(&self.gs.ctm, n(0), n(1)), p3, p3)
                        }
                    };
                    let start = self.cur;
                    if let Some(sp) = self.path.last_mut() {
                        flatten_cubic(sp, start, p1, p2, p3);
                    }
                    self.cur = p3;
                }
                "h" => {
                    if let Some(sp) = self.path.last_mut() {
                        if let Some(first) = sp.first().copied() {
                            sp.push(first);
                            self.cur = first;
                        }
                    }
                }
                "re" => {
                    let (x, y, w, h) = (n(0), n(1), n(2), n(3));
                    let m = self.gs.ctm;
                    let pts: Vec<(f64, f64)> = [(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)].iter().map(|(px, py)| app(&m, *px, *py)).collect();
                    self.cur = pts[0];
                    self.path.push(pts);
                }
                "f" | "F" => self.paint(true, false, false),
                "f*" => self.paint(true, false, true),
                "B" => self.paint(true, true, false),
                "B*" => self.paint(true, true, true),
                "b" => {
                    if let Some(sp) = self.path.last_mut() {
                        if let Some(first) = sp.first().copied() {
                            sp.push(first);
                        }
                    }
                    self.paint(true, true, false)
                }
                "b*" => {
                    if let Some(sp) = self.path.last_mut() {
                        if let Some(first) = sp.first().copied() {
                            sp.push(first);
                        }
                    }
                    self.paint(true, true, true)
                }
                "S" => self.paint(false, true, false),
                "s" => {
                    if let Some(sp) = self.path.last_mut() {
                        if let Some(first) = sp.first().copied() {
                            sp.push(first);
                        }
                    }
                    self.paint(false, true, false)
                }
                "n" => self.path.clear(),
                "BT" => {
                    self.ts.tm = ID;
                    self.ts.tlm = ID;
                }
                "Tf" => {
                    if let Some(Object::Name(name)) = a.first() {
                        self.ts.font = self.font_for(res, name, depth);
                    }
                    self.ts.size = n(1);
                }
                "Tc" => self.ts.tc = n(0),
                "Tw" => self.ts.tw = n(0),
                "Tz" => self.ts.th = n(0) / 100.0,
                "TL" => self.ts.tl = n(0),
                "Ts" => self.ts.rise = n(0),
                "Tr" => self.ts.mode = n(0) as i64,
                "Td" | "TD" => {
                    if op.operator == "TD" {
                        self.ts.tl = -n(1);
                    }
                    self.ts.tlm = mul(&[1.0, 0.0, 0.0, 1.0, n(0), n(1)], &self.ts.tlm);
                    self.ts.tm = self.ts.tlm;
                }
                "Tm" => {
                    if let Some(m) = matrix_of(a) {
                        self.ts.tm = m;
                        self.ts.tlm = m;
                    }
                }
                "T*" => {
                    self.ts.tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -self.ts.tl], &self.ts.tlm);
                    self.ts.tm = self.ts.tlm;
                }
                "Tj" => {
                    if let Some(Object::String(s, _)) = a.first() {
                        self.show(s);
                    }
                }
                "'" | "\"" => {
                    self.ts.tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -self.ts.tl], &self.ts.tlm);
                    self.ts.tm = self.ts.tlm;
                    if let Some(Object::String(s, _)) = a.last() {
                        self.show(s);
                    }
                }
                "TJ" => {
                    if let Some(Object::Array(items)) = a.first() {
                        for it in items {
                            match it {
                                Object::String(s, _) => self.show(s),
                                other => {
                                    if let Some(v) = num(other) {
                                        let tx = -v / 1000.0 * self.ts.size * self.ts.th;
                                        self.ts.tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &self.ts.tm);
                                    }
                                }
                            }
                        }
                    }
                }
                "Do" => {
                    let Some(Object::Name(name)) = a.first() else { continue };
                    let Some(xo) = res.and_then(|r| r.get_deref(b"XObject", pdf).ok()).and_then(|o| dict_of(pdf, o)) else { continue };
                    let Some(Object::Stream(st)) = xo.get(name).ok().map(|o| deref(pdf, o)) else { continue };
                    match st.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) {
                        Some(b"Image") => self.place_image(st),
                        Some(b"Form") if depth < 8 => {
                            let Ok(data) = st.decompressed_content() else { continue };
                            let Ok(content) = Content::decode(&data) else { continue };
                            let saved = (self.gs.clone(), self.stack.len(), self.ts.clone());
                            if let Some(Ok(Object::Array(m))) = Some(st.dict.get_deref(b"Matrix", pdf)) {
                                if let Some(m) = matrix_of(m) {
                                    self.gs.ctm = mul(&m, &self.gs.ctm);
                                }
                            }
                            let fres = st.dict.get_deref(b"Resources", pdf).ok().and_then(|o| dict_of(pdf, o)).or(res);
                            self.run(&content.operations, fres, depth + 1);
                            self.gs = saved.0;
                            self.stack.truncate(saved.1);
                            self.ts = saved.2;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn place_image(&mut self, st: &Stream) {
        let Some(mut img) = decode_image(self.pdf, st, self.gs.fill, 0) else { return };
        let m = self.gs.ctm;
        let corners = [app(&m, 0.0, 0.0), app(&m, 1.0, 0.0), app(&m, 1.0, 1.0), app(&m, 0.0, 1.0)];
        let (x0, x1) = (corners.iter().map(|c| c.0).fold(f64::MAX, f64::min), corners.iter().map(|c| c.0).fold(f64::MIN, f64::max));
        let (y0, y1) = (corners.iter().map(|c| c.1).fold(f64::MAX, f64::min), corners.iter().map(|c| c.1).fold(f64::MIN, f64::max));
        // Riflessioni: l'immagine PDF ha la prima riga in alto quando d > 0.
        if m[3] < 0.0 && m[1].abs() < 1e-6 {
            image::imageops::flip_vertical_in_place(&mut img);
        }
        if m[0] < 0.0 && m[2].abs() < 1e-6 {
            image::imageops::flip_horizontal_in_place(&mut img);
        }
        self.out.img_n += 1;
        let _ = std::fs::create_dir_all(&self.img_dir);
        let path = self.img_dir.join(format!("p{}_img{}.png", self.page_no, self.out.img_n));
        if img.save(&path).is_err() {
            return;
        }
        let order = self.next_order();
        self.out.images.push(ImgItem { path, x0, y0, x1, y1, order });
    }
}

// ---------------------------------------------------------------- rasterizzazione dei vettoriali

fn fill_polys(buf: &mut [[f32; 4]], w: usize, h: usize, polys: &[Vec<(f64, f64)>], evenodd: bool, col: [u8; 3]) {
    let mut edges: Vec<(f64, f64, f64, f64)> = Vec::new();
    for p in polys {
        if p.len() < 2 {
            continue;
        }
        for i in 0..p.len() {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            if a.1 != b.1 {
                edges.push((a.0, a.1, b.0, b.1));
            }
        }
    }
    if edges.is_empty() {
        return;
    }
    let miny = edges.iter().map(|e| e.1.min(e.3)).fold(f64::MAX, f64::min).floor().max(0.0) as usize;
    let maxy = (edges.iter().map(|e| e.1.max(e.3)).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(h);
    const SUB: usize = 4;
    let c = [col[0] as f32, col[1] as f32, col[2] as f32];
    let mut cov = vec![0f32; w];
    let mut xs: Vec<(f64, i32)> = Vec::new();
    for py in miny..maxy {
        cov.iter_mut().for_each(|v| *v = 0.0);
        let mut any = false;
        for s in 0..SUB {
            let y = py as f64 + (s as f64 + 0.5) / SUB as f64;
            xs.clear();
            for e in &edges {
                let (lo, hi) = if e.1 < e.3 { (e.1, e.3) } else { (e.3, e.1) };
                if y >= lo && y < hi {
                    let x = e.0 + (y - e.1) / (e.3 - e.1) * (e.2 - e.0);
                    xs.push((x, if e.3 > e.1 { 1 } else { -1 }));
                }
            }
            xs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut wind = 0i32;
            for k in 0..xs.len().saturating_sub(1) {
                wind += xs[k].1;
                let inside = if evenodd { (k + 1) % 2 == 1 } else { wind != 0 };
                if inside {
                    let (xa, xb) = (xs[k].0.max(0.0), xs[k + 1].0.min(w as f64));
                    if xb <= xa {
                        continue;
                    }
                    any = true;
                    let (ia, ib) = (xa.floor() as usize, (xb.ceil() as usize).min(w));
                    for px in ia..ib {
                        let l = (px as f64).max(xa);
                        let r = ((px + 1) as f64).min(xb);
                        if r > l {
                            cov[px] += (r - l) as f32 / SUB as f32;
                        }
                    }
                }
            }
        }
        if !any {
            continue;
        }
        for (px, cv) in cov.iter().enumerate() {
            if *cv <= 0.0 {
                continue;
            }
            let a = cv.min(1.0);
            let d = &mut buf[py * w + px];
            for i in 0..3 {
                d[i] = c[i] * a + d[i] * (1.0 - a);
            }
            d[3] = a + d[3] * (1.0 - a);
        }
    }
}

fn ccw(mut p: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let area: f64 = (0..p.len()).map(|i| p[i].0 * p[(i + 1) % p.len()].1 - p[(i + 1) % p.len()].0 * p[i].1).sum();
    if area < 0.0 {
        p.reverse();
    }
    p
}

fn stroke_polys(polys: &[Vec<(f64, f64)>], width: f64) -> Vec<Vec<(f64, f64)>> {
    let hw = width / 2.0;
    let mut out = Vec::new();
    for p in polys {
        for i in 0..p.len().saturating_sub(1) {
            let (a, b) = (p[i], p[i + 1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dy);
            if len < 1e-9 {
                continue;
            }
            let (nx, ny) = (-dy / len * hw, dx / len * hw);
            out.push(ccw(vec![(a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny)]));
        }
        if hw > 1.0 {
            for q in p {
                let circle: Vec<(f64, f64)> = (0..10).map(|k| {
                    let t = k as f64 / 10.0 * std::f64::consts::TAU;
                    (q.0 + hw * t.cos(), q.1 + hw * t.sin())
                }).collect();
                out.push(ccw(circle));
            }
        }
    }
    out
}

/// Disegna un gruppo di tracciati in un'immagine RGBA (coordinate PDF in punti, y verso l'alto).
fn raster_cluster(items: &[&PathItem], bbox: (f64, f64, f64, f64), page_h: f64) -> Option<image::RgbaImage> {
    let (x0, y0, x1, y1) = bbox;
    let mut dpi = RASTER_DPI;
    while ((x1 - x0) * dpi / 72.0) * ((y1 - y0) * dpi / 72.0) > 30_000_000.0 && dpi > 30.0 {
        dpi *= 0.8;
    }
    let k = dpi / 72.0;
    let (w, h) = ((((x1 - x0) * k).ceil() as usize).max(1), (((y1 - y0) * k).ceil() as usize).max(1));
    let _ = page_h;
    let mut buf = vec![[0f32; 4]; w * h];
    let to_px = |p: &(f64, f64)| ((p.0 - x0) * k, (y1 - p.1) * k);
    for it in items {
        let polys: Vec<Vec<(f64, f64)>> = it.polys.iter().map(|p| p.iter().map(to_px).collect()).collect();
        if let Some((c, eo)) = it.fill {
            fill_polys(&mut buf, w, h, &polys, eo, c);
        }
        if let Some((c, sw)) = it.stroke {
            let sp = stroke_polys(&polys, (sw * k).max(1.0));
            fill_polys(&mut buf, w, h, &sp, false, c);
        }
    }
    let mut img = image::RgbaImage::new(w as u32, h as u32);
    for (i, px) in buf.iter().enumerate() {
        let a = px[3];
        if a > 0.0 {
            let (r, g, b) = ((px[0] / a).clamp(0.0, 255.0), (px[1] / a).clamp(0.0, 255.0), (px[2] / a).clamp(0.0, 255.0));
            img.put_pixel((i % w) as u32, (i / w) as u32, image::Rgba([r as u8, g as u8, b as u8, (a.min(1.0) * 255.0) as u8]));
        }
    }
    Some(img)
}

fn path_bbox(p: &PathItem) -> (f64, f64, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for poly in &p.polys {
        for q in poly {
            x0 = x0.min(q.0);
            y0 = y0.min(q.1);
            x1 = x1.max(q.0);
            y1 = y1.max(q.1);
        }
    }
    let pad = p.stroke.map(|s| s.1 / 2.0 + 0.5).unwrap_or(0.0);
    (x0 - pad, y0 - pad, x1 + pad, y1 + pad)
}

// ---------------------------------------------------------------- raggruppamento del testo

#[derive(Clone)]
struct Seg {
    text: String,
    x0: f64,
    x1: f64,
    y: f64,
    size: f64,
    bold: bool,
    italic: bool,
    family: Family,
    color: [u8; 3],
    order: usize,
}

fn same_style(a: &Seg, b: &Seg) -> bool {
    (a.size - b.size).abs() < 0.6 && a.bold == b.bold && a.italic == b.italic && a.family == b.family && a.color == b.color
}

fn segments(glyphs: &[Glyph]) -> Vec<Seg> {
    // Righe: stessa linea di base (con tolleranza), poi ordinate da sinistra a destra.
    let mut idx: Vec<usize> = (0..glyphs.len()).collect();
    idx.sort_by(|&a, &b| glyphs[b].y.partial_cmp(&glyphs[a].y).unwrap_or(std::cmp::Ordering::Equal));
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for i in idx {
        let g = &glyphs[i];
        match lines.last_mut() {
            Some(l) if (glyphs[l[0]].y - g.y).abs() < g.size.max(1.0) * 0.25 => l.push(i),
            _ => lines.push(vec![i]),
        }
    }
    let mut segs = Vec::new();
    for mut l in lines {
        l.sort_by(|&a, &b| glyphs[a].x.partial_cmp(&glyphs[b].x).unwrap_or(std::cmp::Ordering::Equal));
        let mut cur: Option<Seg> = None;
        for i in l {
            let g = &glyphs[i];
            let is_space = g.ch.trim().is_empty();
            let s = Seg { text: g.ch.clone(), x0: g.x, x1: g.x + g.w, y: g.y, size: g.size, bold: g.bold, italic: g.italic, family: g.family, color: g.color, order: g.order };
            match cur.as_mut() {
                Some(c) => {
                    let gap = g.x - c.x1;
                    let ends_space = c.text.ends_with(' ');
                    let breaks = gap > c.size * 0.8 || gap < -c.size * 0.5 || (!is_space && !same_style(c, &s));
                    if breaks {
                        segs.push(cur.take().expect("segmento aperto"));
                        if !is_space {
                            cur = Some(s);
                        }
                    } else {
                        if gap > c.size * 0.18 && !ends_space && !is_space {
                            c.text.push(' ');
                        }
                        if is_space {
                            if !ends_space {
                                c.text.push(' ');
                            }
                        } else {
                            c.text.push_str(&g.ch);
                        }
                        c.x1 = c.x1.max(g.x + g.w);
                        c.order = c.order.min(s.order);
                    }
                }
                None => {
                    if !is_space {
                        cur = Some(s);
                    }
                }
            }
        }
        if let Some(c) = cur {
            segs.push(c);
        }
    }
    for s in &mut segs {
        s.text = s.text.trim_end().to_string();
    }
    segs.retain(|s| !s.text.is_empty());
    segs
}

struct Block {
    lines: Vec<Seg>,
    center: bool,
}

fn blocks(mut segs: Vec<Seg>, page_w: f64) -> Vec<Block> {
    segs.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal).then(a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal)));
    let mut blocks: Vec<Block> = Vec::new();
    for s in segs {
        let mut joined = false;
        for b in blocks.iter_mut().rev() {
            let last = b.lines.last().expect("blocco non vuoto");
            let dy = last.y - s.y;
            if dy < s.size * 0.6 || dy > s.size * 2.4 || !same_style(last, &s) {
                continue;
            }
            let left = (last.x0 - s.x0).abs() < s.size * 0.5;
            let cent = ((last.x0 + last.x1) / 2.0 - (s.x0 + s.x1) / 2.0).abs() < s.size * 0.6 && (last.x0 - s.x0).abs() > s.size * 0.5;
            if (left && !b.center) || (cent && (b.center || b.lines.len() == 1)) {
                if cent {
                    b.center = true;
                }
                b.lines.push(s.clone());
                joined = true;
                break;
            }
        }
        if !joined {
            // Un riquadro con una sola riga resta centrato se il testo sta al centro della pagina.
            let center = ((s.x0 + s.x1) / 2.0 - page_w / 2.0).abs() < page_w * 0.02 && s.x0 > page_w * 0.15;
            blocks.push(Block { lines: vec![s], center });
        }
    }
    blocks
}

// ---------------------------------------------------------------- pagina → riquadri

struct Spec {
    order: usize,
    rect: Rect,
    kind: FrameKind,
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb(c[0], c[1], c[2])
}

fn inherited<'a>(pdf: &'a Pdf, page: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    let mut d = page;
    for _ in 0..16 {
        if let Ok(o) = d.get_deref(key, pdf) {
            return Some(o);
        }
        d = d.get_deref(b"Parent", pdf).ok().and_then(|o| dict_of(pdf, o))?;
    }
    None
}

fn page_box(pdf: &Pdf, page: &Dictionary) -> (f64, f64, f64, f64) {
    let b = inherited(pdf, page, b"CropBox").or_else(|| inherited(pdf, page, b"MediaBox"));
    match b {
        Some(Object::Array(a)) => {
            let v: Vec<f64> = a.iter().filter_map(|o| num(deref(pdf, o))).collect();
            if v.len() == 4 { (v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])) } else { (0.0, 0.0, 595.0, 842.0) }
        }
        _ => (0.0, 0.0, 595.0, 842.0),
    }
}

fn field_info<'a>(pdf: &'a Pdf, w: &'a Dictionary) -> (Option<String>, Option<String>, i64, Option<f64>) {
    // FT, V, Ff e dimensione da DA, risalendo ai genitori.
    let (mut ft, mut v, mut ff, mut size) = (None, None, 0i64, None);
    let mut d = w;
    for _ in 0..8 {
        if ft.is_none() {
            ft = d.get(b"FT").ok().and_then(|o| o.as_name().ok()).map(|n| String::from_utf8_lossy(n).to_string());
        }
        if v.is_none() {
            v = match d.get_deref(b"V", pdf) {
                Ok(Object::String(s, _)) => Some(lopdf::decode_text_string(&Object::String(s.clone(), lopdf::StringFormat::Literal)).unwrap_or_else(|_| String::from_utf8_lossy(s).to_string())),
                _ => None,
            };
        }
        if ff == 0 {
            ff = d.get(b"Ff").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0);
        }
        if size.is_none() {
            if let Ok(Object::String(s, _)) = d.get_deref(b"DA", pdf) {
                let da = String::from_utf8_lossy(s).to_string();
                let toks: Vec<&str> = da.split_whitespace().collect();
                if let Some(i) = toks.iter().position(|t| *t == "Tf") {
                    size = i.checked_sub(1).and_then(|k| toks[k].parse::<f64>().ok()).filter(|s| *s > 0.0);
                }
            }
        }
        match d.get_deref(b"Parent", pdf).ok().and_then(|o| dict_of(pdf, o)) {
            Some(p) => d = p,
            None => break,
        }
    }
    (ft, v, ff, size)
}

fn build_page(pdf: &Pdf, page_id: lopdf::ObjectId, page_no: usize, img_dir: &Path, doc: &mut Document, di: usize, stats: &mut Stats) {
    let Ok(page) = pdf.get_dictionary(page_id) else { return };
    let (bx0, by0, bx1, by1) = page_box(pdf, page);
    let (pw, ph) = (bx1 - bx0, by1 - by0);
    let data = pdf.get_page_content(page_id);
    let ops = Content::decode(&data).map(|c| c.operations).unwrap_or_default();
    let res = inherited(pdf, page, b"Resources").and_then(|o| dict_of(pdf, o));
    let mut it = Interp::new(pdf, img_dir.to_path_buf(), page_no);
    it.gs.ctm = [1.0, 0.0, 0.0, 1.0, -bx0, -by0];
    it.run(&ops, res, 0);
    let out = it.out;
    stats.rotated += out.rotated;
    stats.hidden += out.hidden;

    let to_rect = |x0: f64, y0: f64, x1: f64, y1: f64| Rect { x: (x0 * PT_MM) as f32, y: ((ph - y1) * PT_MM) as f32, w: ((x1 - x0) * PT_MM) as f32, h: ((y1 - y0) * PT_MM) as f32 };
    let mut specs: Vec<Spec> = Vec::new();

    for r in &out.rects {
        if (r.x1 - r.x0) < 0.05 && (r.y1 - r.y0) < 0.05 {
            continue;
        }
        let sw = r.stroke.map(|s| s.1 * PT_MM).unwrap_or(0.0) as f32;
        specs.push(Spec { order: r.order, rect: to_rect(r.x0, r.y0, r.x1, r.y1), kind: FrameKind::Rectangle { fill: rgb(r.fill), stroke: rgb(r.stroke.map(|s| s.0).unwrap_or(r.fill)), stroke_mm: sw } });
        stats.shapes += 1;
    }
    for im in &out.images {
        specs.push(Spec { order: im.order, rect: to_rect(im.x0, im.y0, im.x1, im.y1), kind: FrameKind::Picture { path: im.path.to_string_lossy().into_owned() } });
        stats.images += 1;
    }

    // Grafica vettoriale: gruppi di tracciati vicini → un'immagine ciascuno.
    let items: Vec<(&PathItem, (f64, f64, f64, f64))> = out.paths.iter().map(|p| (p, path_bbox(p))).filter(|(_, b)| b.2 > 0.0 && b.3 > 0.0 && b.0 < pw && b.1 < ph).collect();
    let mut parent: Vec<usize> = (0..items.len()).collect();
    fn find(p: &mut Vec<usize>, i: usize) -> usize {
        let mut r = i;
        while p[r] != r {
            r = p[r];
        }
        let mut c = i;
        while p[c] != r {
            let n = p[c];
            p[c] = r;
            c = n;
        }
        r
    }
    let area = |b: &(f64, f64, f64, f64)| (b.2 - b.0) * (b.3 - b.1);
    for i in 0..items.len() {
        if area(&items[i].1) > pw * ph * 0.35 {
            continue;
        }
        for j in i + 1..items.len() {
            if area(&items[j].1) > pw * ph * 0.35 {
                continue;
            }
            let (a, b) = (&items[i].1, &items[j].1);
            if a.0 - 2.0 <= b.2 && b.0 - 2.0 <= a.2 && a.1 - 2.0 <= b.3 && b.1 - 2.0 <= a.3 {
                let (ra, rb) = (find(&mut parent, i), find(&mut parent, j));
                parent[ra] = rb;
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..items.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut gk: Vec<_> = groups.into_iter().collect();
    gk.sort_by_key(|(r, _)| *r);
    for (n, (_, members)) in gk.into_iter().enumerate() {
        let list: Vec<&PathItem> = members.iter().map(|&i| items[i].0).collect();
        let bb = members.iter().map(|&i| items[i].1).fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)));
        let bb = (bb.0.max(0.0), bb.1.max(0.0), bb.2.min(pw), bb.3.min(ph));
        if bb.2 - bb.0 < 0.2 || bb.3 - bb.1 < 0.2 {
            continue;
        }
        let Some(img) = raster_cluster(&list, bb, ph) else { continue };
        let _ = std::fs::create_dir_all(img_dir);
        let path = img_dir.join(format!("p{page_no}_vec{n}.png"));
        if img.save(&path).is_ok() {
            let order = list.iter().map(|p| p.order).min().unwrap_or(0);
            specs.push(Spec { order, rect: to_rect(bb.0, bb.1, bb.2, bb.3), kind: FrameKind::Picture { path: path.to_string_lossy().into_owned() } });
            stats.vectors += 1;
        }
    }

    // Testo → riquadri di testo.
    for b in blocks(segments(&out.glyphs), pw) {
        let first = &b.lines[0];
        let size = first.size.clamp(2.0, 400.0);
        let n = b.lines.len();
        let pitch = if n > 1 { (b.lines[0].y - b.lines[n - 1].y) / (n - 1) as f64 } else { size * 1.22 };
        let ls = if n > 1 { (pitch / (size * 1.22)).clamp(0.6, 3.0) } else { 1.0 };
        let max_w = b.lines.iter().map(|l| l.x1 - l.x0).fold(0.0, f64::max);
        let width = max_w * 1.12 + size * 0.6;
        let x0 = if b.center {
            let c = b.lines.iter().map(|l| (l.x0 + l.x1) / 2.0).sum::<f64>() / n as f64;
            c - width / 2.0
        } else {
            b.lines.iter().map(|l| l.x0).fold(f64::MAX, f64::min)
        };
        let top = first.y + (0.61 * ls + 0.32) * size; // PDF: y verso l'alto
        let height = n as f64 * size * 1.22 * ls + size * 0.2;
        let text = b.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        let style = TextStyle { bold: first.bold, italic: first.italic, family: first.family, align: if b.center { Align::Center } else { Align::Left }, line_spacing: ls as f32, ..Default::default() };
        let order = b.lines.iter().map(|l| l.order).min().unwrap_or(0);
        specs.push(Spec { order, rect: to_rect(x0, top - height, x0 + width, top), kind: FrameKind::Text { text, size_pt: size as f32, color: rgb(first.color), style } });
        stats.texts += 1;
    }

    specs.sort_by_key(|s| s.order);
    for s in specs {
        doc.add_frame(di, s.rect, s.kind);
    }

    // Campi dei moduli (AcroForm): un riquadro di testo per ogni campo di testo.
    if let Ok(Object::Array(annots)) = page.get_deref(b"Annots", pdf) {
        for a in annots {
            let Some(w) = dict_of(pdf, a) else { continue };
            if w.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) != Some(b"Widget") {
                continue;
            }
            let (ft, value, _ff, size) = field_info(pdf, w);
            if !matches!(ft.as_deref(), Some("Tx") | Some("Ch")) {
                continue;
            }
            let Some(Object::Array(r)) = w.get_deref(b"Rect", pdf).ok() else { continue };
            let v: Vec<f64> = r.iter().filter_map(|o| num(deref(pdf, o))).collect();
            if v.len() != 4 {
                continue;
            }
            let (x0, x1, y0, y1) = (v[0].min(v[2]) - bx0, v[0].max(v[2]) - bx0, v[1].min(v[3]) - by0, v[1].max(v[3]) - by0);
            let size = size.unwrap_or(((y1 - y0) * 0.7).clamp(6.0, 14.0));
            doc.add_frame(
                di,
                to_rect(x0, y0, x1, y1),
                FrameKind::Text { text: value.unwrap_or_default(), size_pt: size as f32, color: Rgb(0x20, 0x20, 0x20), style: TextStyle::default() },
            );
            stats.fields += 1;
        }
    }
}

#[derive(Default)]
pub(crate) struct Stats {
    pub texts: usize,
    pub images: usize,
    pub shapes: usize,
    pub vectors: usize,
    pub fields: usize,
    pub rotated: usize,
    pub hidden: usize,
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))
}

/// Carta un PDF e restituisce il documento e un riepilogo. Le immagini estratte vanno in `%APPDATA%\PubliCraft\imports`.
pub(crate) fn import_pdf(path: &Path) -> Result<(Document, String), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let pdf = Pdf::load_mem(&bytes).map_err(|e| format!("PDF non leggibile: {e}"))?;
    if pdf.is_encrypted() {
        return Err("il PDF è protetto da password".into());
    }
    let pages = pdf.get_pages();
    if pages.is_empty() {
        return Err("il PDF non contiene pagine".into());
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "pdf".into());
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let img_dir = base.join("PubliCraft").join("imports").join(format!("{stem}-{:08x}", fnv(&bytes) as u32));

    let first = pdf.get_dictionary(*pages.values().next().expect("almeno una pagina")).map_err(|e| e.to_string())?;
    let (x0, y0, x1, y1) = page_box(&pdf, first);
    let mut doc = Document::new(PageSize { w_mm: ((x1 - x0) * PT_MM) as f32, h_mm: ((y1 - y0) * PT_MM) as f32 });
    let mut stats = Stats::default();
    let mut count = 0;
    for (n, (_, id)) in pages.iter().enumerate().take(200) {
        if n > 0 {
            doc.insert_page(n);
        }
        build_page(&pdf, *id, n + 1, &img_dir, &mut doc, n, &mut stats);
        count += 1;
    }
    let mut msg = format!("Importate {count} pagine: {} testi, {} immagini, {} forme, {} disegni vettoriali, {} campi", stats.texts, stats.images, stats.shapes, stats.vectors, stats.fields);
    if stats.rotated > 0 {
        msg.push_str(&format!(" — {} caratteri ruotati non importati", stats.rotated));
    }
    if stats.hidden > 0 && stats.texts == 0 {
        msg.push_str(" — sembra una scansione: contiene solo immagini (il testo non è riconoscibile senza OCR)");
    }
    Ok((doc, msg))
}
