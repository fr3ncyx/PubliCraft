//! Stampa su stampanti Windows (GDI): elenco stampanti e formati carta, impostazioni e invio delle pagine.

use crate::export::PageImage;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, GetDeviceCaps, SetStretchBltMode, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, DEVMODEW, DIB_RGB_COLORS, HALFTONE, SRCCOPY,
};
use windows_sys::Win32::Graphics::Printing::{ClosePrinter, DocumentPropertiesW, EnumPrintersW, GetDefaultPrinterW, OpenPrinterW, PRINTER_INFO_4W};
use windows_sys::Win32::Storage::Xps::{DeviceCapabilitiesW, EndDoc, EndPage, StartDocW, StartPage, DOCINFOW};

const PRINTER_ENUM_LOCAL: u32 = 2;
const PRINTER_ENUM_CONNECTIONS: u32 = 4;
const DM_IN_BUFFER: u32 = 8;
const DM_OUT_BUFFER: u32 = 2;
const DM_ORIENTATION: u32 = 0x1;
const DM_PAPERSIZE: u32 = 0x2;
const DM_COPIES: u32 = 0x100;
const DM_COLOR: u32 = 0x800;
const DM_DUPLEX: u32 = 0x1000;
const DC_PAPERS: u16 = 2;
const DC_PAPERNAMES: u16 = 16;
const HORZRES: i32 = 8;
const VERTRES: i32 = 10;
const LOGPIXELSX: i32 = 88;
const LOGPIXELSY: i32 = 90;
const PHYSICALOFFSETX: i32 = 112;
const PHYSICALOFFSETY: i32 = 113;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn from_wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut n = 0;
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
}

/// Nomi delle stampanti installate.
pub fn list_printers() -> Vec<String> {
    unsafe {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut needed, mut count) = (0u32, 0u32);
        EnumPrintersW(flags, null(), 4, null_mut(), 0, &mut needed, &mut count);
        if needed == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
        if EnumPrintersW(flags, null(), 4, buf.as_mut_ptr() as *mut u8, needed, &mut needed, &mut count) == 0 {
            return Vec::new();
        }
        let infos = std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_4W, count as usize);
        infos.iter().map(|i| from_wide(i.pPrinterName)).filter(|n| !n.is_empty()).collect()
    }
}

pub fn default_printer() -> Option<String> {
    unsafe {
        let mut n = 0u32;
        GetDefaultPrinterW(null_mut(), &mut n);
        if n == 0 {
            return None;
        }
        let mut buf = vec![0u16; n as usize];
        if GetDefaultPrinterW(buf.as_mut_ptr(), &mut n) == 0 {
            return None;
        }
        Some(from_wide(buf.as_ptr()))
    }
}

/// Formati carta della stampante: (codice, nome).
pub fn paper_sizes(printer: &str) -> Vec<(i16, String)> {
    unsafe {
        let name = wide(printer);
        let n = DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, null_mut(), null());
        if n <= 0 {
            return Vec::new();
        }
        let mut codes = vec![0u16; n as usize];
        let mut names = vec![0u16; n as usize * 64];
        DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, codes.as_mut_ptr(), null());
        DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERNAMES, names.as_mut_ptr(), null());
        (0..n as usize).map(|i| (codes[i] as i16, from_wide(names[i * 64..].as_ptr()))).collect()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Orient {
    Auto,
    Portrait,
    Landscape,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Duplex {
    Off,
    LongEdge,
    ShortEdge,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub printer: String,
    pub copies: u16,
    pub color: bool,
    pub orient: Orient,
    pub duplex: Duplex,
    /// Codice carta (0 = predefinita della stampante).
    pub paper: i16,
    /// Adatta la pagina all'area stampabile (altrimenti dimensione reale).
    pub fit: bool,
}

/// Invia le pagine alla stampante. `pages` sono già in scala di grigi se richiesto.
pub fn print(job_name: &str, s: &Settings, pages: &[PageImage]) -> Result<(), String> {
    if pages.is_empty() {
        return Err("nessuna pagina da stampare".into());
    }
    unsafe {
        let name = wide(&s.printer);
        let mut hp: HANDLE = null_mut();
        if OpenPrinterW(name.as_ptr(), &mut hp, null()) == 0 {
            return Err(format!("impossibile aprire la stampante «{}»", s.printer));
        }
        let size = DocumentPropertiesW(null_mut(), hp, name.as_ptr(), null_mut(), null(), 0);
        if size <= 0 {
            ClosePrinter(hp);
            return Err("impossibile leggere le impostazioni della stampante".into());
        }
        let mut raw = vec![0u64; (size as usize).div_ceil(8)];
        let dm = raw.as_mut_ptr() as *mut DEVMODEW;
        if DocumentPropertiesW(null_mut(), hp, name.as_ptr(), dm, null(), DM_OUT_BUFFER) < 0 {
            ClosePrinter(hp);
            return Err("impossibile leggere le impostazioni della stampante".into());
        }
        let first = &pages[0];
        let landscape = match s.orient {
            Orient::Auto => first.w_mm > first.h_mm,
            Orient::Landscape => true,
            Orient::Portrait => false,
        };
        {
            let d = &mut *dm;
            d.dmFields |= DM_ORIENTATION | DM_COPIES | DM_COLOR | DM_DUPLEX;
            d.Anonymous1.Anonymous1.dmOrientation = if landscape { 2 } else { 1 };
            d.Anonymous1.Anonymous1.dmCopies = s.copies.max(1) as i16;
            if s.paper > 0 {
                d.dmFields |= DM_PAPERSIZE;
                d.Anonymous1.Anonymous1.dmPaperSize = s.paper;
            }
            d.dmColor = if s.color { 2 } else { 1 };
            d.dmDuplex = match s.duplex {
                Duplex::Off => 1,
                Duplex::LongEdge => 2,
                Duplex::ShortEdge => 3,
            };
        }
        // Lascia che il driver corregga i valori non supportati.
        DocumentPropertiesW(null_mut(), hp, name.as_ptr(), dm, dm, DM_IN_BUFFER | DM_OUT_BUFFER);
        ClosePrinter(hp);

        let dc = CreateDCW(null(), name.as_ptr(), null(), dm);
        if dc.is_null() {
            return Err("impossibile preparare la stampante".into());
        }
        let (hres, vres) = (GetDeviceCaps(dc, HORZRES), GetDeviceCaps(dc, VERTRES));
        let (lx, ly) = (GetDeviceCaps(dc, LOGPIXELSX) as f32, GetDeviceCaps(dc, LOGPIXELSY) as f32);
        let (ox, oy) = (GetDeviceCaps(dc, PHYSICALOFFSETX), GetDeviceCaps(dc, PHYSICALOFFSETY));

        let doc_name = wide(job_name);
        let info = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: doc_name.as_ptr(), lpszOutput: null(), lpszDatatype: null(), fwType: 0 };
        if StartDocW(dc, &info) <= 0 {
            DeleteDC(dc);
            return Err("la stampante ha rifiutato il lavoro".into());
        }
        SetStretchBltMode(dc, HALFTONE);
        let mut result = Ok(());
        for pg in pages {
            if StartPage(dc) <= 0 {
                result = Err("errore durante la stampa".to_string());
                break;
            }
            // Dimensione reale in pixel della stampante, poi eventuale adattamento.
            let (real_w, real_h) = (pg.w_mm / 25.4 * lx, pg.h_mm / 25.4 * ly);
            let (dw, dh, x, y) = if s.fit {
                let k = (hres as f32 / real_w).min(vres as f32 / real_h);
                let (dw, dh) = (real_w * k, real_h * k);
                (dw, dh, (hres as f32 - dw) / 2.0, (vres as f32 - dh) / 2.0)
            } else {
                (real_w, real_h, -(ox as f32), -(oy as f32))
            };
            // Immagine BGRA dall'alto in basso.
            let mut bgra = Vec::with_capacity(pg.w * pg.h * 4);
            for px in pg.rgb.chunks_exact(3) {
                bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: pg.w as i32,
                biHeight: -(pg.h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            };
            StretchDIBits(dc, x.round() as i32, y.round() as i32, dw.round() as i32, dh.round() as i32, 0, 0, pg.w as i32, pg.h as i32, bgra.as_ptr() as *const _, &bmi, DIB_RGB_COLORS, SRCCOPY);
            EndPage(dc);
        }
        EndDoc(dc);
        DeleteDC(dc);
        result
    }
}
