//! Drawing a barcode's modules as SVG or PNG. Shared by every symbology: a
//! 2D code is a grid of modules, a linear one a single row repeated to its
//! bar height.

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage};

use crate::pipeline::PipelineError;

/// The modules to draw, `rows[y][x]`, `true` = dark, without the quiet zone.
pub struct Drawing<'a> {
    pub rows: &'a [Vec<bool>],
    /// Light modules around the code, in module widths.
    pub margin: usize,
    /// Pixels per module horizontally.
    pub module_px: u32,
    /// Pixels per row vertically (a linear code's bar height is one row).
    pub row_px: u32,
    pub dark: [u8; 3],
    pub light: [u8; 3],
}

impl Drawing<'_> {
    fn columns(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    pub fn width_px(&self) -> u32 {
        (self.columns() + 2 * self.margin) as u32 * self.module_px
    }

    pub fn height_px(&self) -> u32 {
        self.rows.len() as u32 * self.row_px + 2 * self.margin as u32 * self.module_px
    }

    /// One path of unit squares in module coordinates, scaled by the SVG's
    /// width and height: a sharp code at any print size.
    pub fn svg(&self) -> String {
        let cols = self.columns() + 2 * self.margin;
        let unit_h = self.row_px as f64 / self.module_px as f64;
        let rows_h = self.rows.len() as f64 * unit_h + 2.0 * self.margin as f64;
        let mut path = String::new();
        for (y, row) in self.rows.iter().enumerate() {
            let top = self.margin as f64 + y as f64 * unit_h;
            let mut x = 0;
            while x < row.len() {
                if row[x] {
                    let start = x;
                    while x < row.len() && row[x] {
                        x += 1;
                    }
                    path.push_str(&format!("M{},{}h{}v{}h-{}z", start + self.margin, trim(top), x - start, trim(unit_h), x - start));
                } else {
                    x += 1;
                }
            }
        }
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {cols} {vh}\" shape-rendering=\"crispEdges\">\
             <rect width=\"100%\" height=\"100%\" fill=\"{light}\"/><path fill=\"{dark}\" d=\"{path}\"/></svg>",
            w = self.width_px(),
            h = self.height_px(),
            vh = trim(rows_h),
            light = hex(self.light),
            dark = hex(self.dark),
        )
    }

    pub fn png(&self) -> Result<Vec<u8>, PipelineError> {
        let (w, h) = (self.width_px(), self.height_px());
        let top = self.margin as u32 * self.module_px;
        let mut img = RgbImage::from_pixel(w, h, Rgb(self.light));
        for (y, row) in self.rows.iter().enumerate() {
            for (x, &dark) in row.iter().enumerate() {
                if !dark {
                    continue;
                }
                let px = (x + self.margin) as u32 * self.module_px;
                let py = top + y as u32 * self.row_px;
                for dy in 0..self.row_px {
                    for dx in 0..self.module_px {
                        img.put_pixel(px + dx, py + dy, Rgb(self.dark));
                    }
                }
            }
        }
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .map_err(|e| PipelineError::new("FS_BARCODE", format!("PNG encode: {e}")))?;
        Ok(out)
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// `#rgb` or `#rrggbb`; anything else is refused, so a colour can never
/// carry markup into the SVG.
pub fn parse_colour(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#')?;
    let digits: Vec<u8> = hex.chars().map(|c| c.to_digit(16).map(|d| d as u8)).collect::<Option<_>>()?;
    match digits.as_slice() {
        [r, g, b] => Some([r * 17, g * 17, b * 17]),
        [r1, r2, g1, g2, b1, b2] => Some([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2]),
        _ => None,
    }
}
