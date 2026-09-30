//! QR Code symbol construction (ISO/IEC 18004): one segment in the densest
//! mode the text allows, the smallest version that holds it, Reed–Solomon
//! blocks interleaved, function patterns, data placement, and the mask with
//! the lowest penalty.

use super::reed_solomon;
use super::tables::{self, Ecc, MAX_VERSION, MIN_VERSION};

/// A finished symbol: `modules[y][x]`, `true` = dark. No quiet zone.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub version: usize,
    pub ecc: Ecc,
    pub mask: u8,
    pub size: usize,
    pub modules: Vec<Vec<bool>>,
}

const ALPHANUMERIC: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

#[derive(Clone, Copy)]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

impl Mode {
    fn of(text: &str) -> Self {
        if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
            Self::Numeric
        } else if !text.is_empty() && text.chars().all(|c| ALPHANUMERIC.contains(c)) {
            Self::Alphanumeric
        } else {
            Self::Byte
        }
    }

    fn indicator(self) -> u32 {
        match self {
            Self::Numeric => 0b0001,
            Self::Alphanumeric => 0b0010,
            Self::Byte => 0b0100,
        }
    }

    fn count_bits(self, version: usize) -> usize {
        let band = if version <= 9 { 0 } else if version <= 26 { 1 } else { 2 };
        match self {
            Self::Numeric => [10, 12, 14][band],
            Self::Alphanumeric => [9, 11, 13][band],
            Self::Byte => [8, 16, 16][band],
        }
    }
}

struct Bits(Vec<bool>);

impl Bits {
    fn push(&mut self, value: u32, len: usize) {
        for i in (0..len).rev() {
            self.0.push((value >> i) & 1 == 1);
        }
    }
}

/// The segment's payload bits and character count, mode header excluded.
fn payload(text: &str, mode: Mode) -> (Bits, usize) {
    let mut bits = Bits(Vec::new());
    match mode {
        Mode::Numeric => {
            let digits = text.as_bytes();
            for chunk in digits.chunks(3) {
                let value = chunk.iter().fold(0u32, |acc, d| acc * 10 + (d - b'0') as u32);
                bits.push(value, chunk.len() * 3 + 1);
            }
            (bits, digits.len())
        }
        Mode::Alphanumeric => {
            let codes: Vec<u32> = text.chars().map(|c| ALPHANUMERIC.find(c).unwrap_or(0) as u32).collect();
            for pair in codes.chunks(2) {
                match pair {
                    [a, b] => bits.push(a * 45 + b, 11),
                    [a] => bits.push(*a, 6),
                    _ => {}
                }
            }
            (bits, codes.len())
        }
        Mode::Byte => {
            for &byte in text.as_bytes() {
                bits.push(byte as u32, 8);
            }
            (bits, text.len())
        }
    }
}

/// Encode `text` at `ecc`, in the smallest version from 1 to 40 that holds
/// it. `None` when even version 40 cannot.
pub fn encode(text: &str, ecc: Ecc) -> Option<Symbol> {
    let (version, codewords) = codewords(text, ecc)?;
    Some(build(version, ecc, &codewords))
}

/// The version chosen and every codeword in placement order: data and error
/// correction, blocks interleaved.
pub fn codewords(text: &str, ecc: Ecc) -> Option<(usize, Vec<u8>)> {
    let mode = Mode::of(text);
    let (body, count) = payload(text, mode);
    let version = (MIN_VERSION..=MAX_VERSION).find(|&v| {
        count < (1usize << mode.count_bits(v))
            && 4 + mode.count_bits(v) + body.0.len() <= tables::num_data_codewords(v, ecc) * 8
    })?;

    let capacity_bits = tables::num_data_codewords(version, ecc) * 8;
    let mut bits = Bits(Vec::with_capacity(capacity_bits));
    bits.push(mode.indicator(), 4);
    bits.push(count as u32, mode.count_bits(version));
    bits.0.extend(body.0);
    let terminator = (capacity_bits - bits.0.len()).min(4);
    bits.push(0, terminator);
    let to_byte = (8 - bits.0.len() % 8) % 8;
    bits.push(0, to_byte);
    let mut data: Vec<u8> = bits.0.chunks(8).map(|b| b.iter().fold(0u8, |acc, &bit| (acc << 1) | bit as u8)).collect();
    let mut pad = [0xEC, 0x11].into_iter().cycle();
    while data.len() < capacity_bits / 8 {
        data.push(pad.next().unwrap_or(0xEC));
    }

    Some((version, interleave_with_ecc(&data, version, ecc)))
}

/// Split into blocks, append each block's error correction, and interleave.
fn interleave_with_ecc(data: &[u8], version: usize, ecc: Ecc) -> Vec<u8> {
    let num_blocks = tables::num_blocks(ecc, version);
    let block_ecc_len = tables::ecc_codewords_per_block(ecc, version);
    let raw_codewords = tables::num_raw_data_modules(version) / 8;
    let num_short_blocks = num_blocks - raw_codewords % num_blocks;
    let short_block_len = raw_codewords / num_blocks;
    let divisor = reed_solomon::divisor(block_ecc_len);

    let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(num_blocks);
    let mut k = 0;
    for i in 0..num_blocks {
        let data_len = short_block_len - block_ecc_len + usize::from(i >= num_short_blocks);
        let mut block = data[k..k + data_len].to_vec();
        k += data_len;
        let ecc_bytes = reed_solomon::remainder(&block, &divisor);
        if i < num_short_blocks {
            block.push(0); // a placeholder, skipped when interleaving
        }
        block.extend(ecc_bytes);
        blocks.push(block);
    }

    let mut result = Vec::with_capacity(raw_codewords);
    for i in 0..blocks[0].len() {
        for (j, block) in blocks.iter().enumerate() {
            if i != short_block_len - block_ecc_len || j >= num_short_blocks {
                result.push(block[i]);
            }
        }
    }
    result
}

struct Grid {
    size: usize,
    modules: Vec<Vec<bool>>,
    function: Vec<Vec<bool>>,
}

impl Grid {
    fn set_function(&mut self, x: usize, y: usize, dark: bool) {
        self.modules[y][x] = dark;
        self.function[y][x] = true;
    }
}

fn build(version: usize, ecc: Ecc, codewords: &[u8]) -> Symbol {
    let size = tables::size(version);
    let mut grid = Grid { size, modules: vec![vec![false; size]; size], function: vec![vec![false; size]; size] };
    draw_function_patterns(&mut grid, version, ecc);
    draw_codewords(&mut grid, codewords);

    let mut best: Option<(u32, u8, Vec<Vec<bool>>)> = None;
    for mask in 0..8u8 {
        apply_mask(&mut grid, mask);
        draw_format_bits(&mut grid, ecc, mask);
        let score = super::penalty::score(&grid.modules);
        if best.as_ref().is_none_or(|(s, ..)| score < *s) {
            best = Some((score, mask, grid.modules.clone()));
        }
        apply_mask(&mut grid, mask); // XOR again undoes it
    }
    let (_, mask, modules) = best.expect("eight masks were tried");
    Symbol { version, ecc, mask, size, modules }
}

fn draw_function_patterns(grid: &mut Grid, version: usize, ecc: Ecc) {
    let size = grid.size;
    for i in 0..size {
        grid.set_function(6, i, i % 2 == 0);
        grid.set_function(i, 6, i % 2 == 0);
    }
    for (cx, cy) in [(3, 3), (size - 4, 3), (3, size - 4)] {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                if (0..size as i32).contains(&x) && (0..size as i32).contains(&y) {
                    let dist = dx.abs().max(dy.abs());
                    grid.set_function(x as usize, y as usize, dist != 2 && dist != 4);
                }
            }
        }
    }
    let positions = tables::alignment_positions(version);
    let last = positions.len().saturating_sub(1);
    for (i, &cx) in positions.iter().enumerate() {
        for (j, &cy) in positions.iter().enumerate() {
            let on_finder = (i == 0 && j == 0) || (i == 0 && j == last) || (i == last && j == 0);
            if on_finder {
                continue;
            }
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let dark = dx.abs().max(dy.abs()) != 1;
                    grid.set_function((cx as i32 + dx) as usize, (cy as i32 + dy) as usize, dark);
                }
            }
        }
    }
    draw_format_bits(grid, ecc, 0); // reserves the area; redrawn per mask
    draw_version_bits(grid, version);
}

fn bit(value: u32, i: usize) -> bool {
    (value >> i) & 1 == 1
}

fn draw_format_bits(grid: &mut Grid, ecc: Ecc, mask: u8) {
    let data = (ecc.format_bits() << 3) | mask as u32;
    let mut rem = data;
    for _ in 0..10 {
        rem = (rem << 1) ^ ((rem >> 9) * 0x537);
    }
    let bits = ((data << 10) | rem) ^ 0x5412;
    let size = grid.size;
    for i in 0..=5 {
        grid.set_function(8, i, bit(bits, i));
    }
    grid.set_function(8, 7, bit(bits, 6));
    grid.set_function(8, 8, bit(bits, 7));
    grid.set_function(7, 8, bit(bits, 8));
    for i in 9..15 {
        grid.set_function(14 - i, 8, bit(bits, i));
    }
    for i in 0..8 {
        grid.set_function(size - 1 - i, 8, bit(bits, i));
    }
    for i in 8..15 {
        grid.set_function(8, size - 15 + i, bit(bits, i));
    }
    grid.set_function(8, size - 8, true);
}

fn draw_version_bits(grid: &mut Grid, version: usize) {
    if version < 7 {
        return;
    }
    let mut rem = version as u32;
    for _ in 0..12 {
        rem = (rem << 1) ^ ((rem >> 11) * 0x1F25);
    }
    let bits = ((version as u32) << 12) | rem;
    for i in 0..18 {
        let a = grid.size - 11 + i % 3;
        let b = i / 3;
        grid.set_function(a, b, bit(bits, i));
        grid.set_function(b, a, bit(bits, i));
    }
}

/// The zigzag: column pairs from the right, alternating up and down,
/// stepping over the vertical timing column.
fn draw_codewords(grid: &mut Grid, codewords: &[u8]) {
    let size = grid.size;
    let total_bits = codewords.len() * 8;
    let mut i = 0;
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        for vert in 0..size {
            for j in 0..2 {
                let x = (right - j) as usize;
                let upward = (right + 1) & 2 == 0;
                let y = if upward { size - 1 - vert } else { vert };
                if !grid.function[y][x] && i < total_bits {
                    grid.modules[y][x] = (codewords[i >> 3] >> (7 - (i & 7))) & 1 == 1;
                    i += 1;
                }
            }
        }
        right -= 2;
    }
}

fn apply_mask(grid: &mut Grid, mask: u8) {
    for y in 0..grid.size {
        for x in 0..grid.size {
            let invert = match mask {
                0 => (x + y) % 2 == 0,
                1 => y % 2 == 0,
                2 => x % 3 == 0,
                3 => (x + y) % 3 == 0,
                4 => (x / 3 + y / 2) % 2 == 0,
                5 => x * y % 2 + x * y % 3 == 0,
                6 => (x * y % 2 + x * y % 3) % 2 == 0,
                _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
            };
            if invert && !grid.function[y][x] {
                grid.modules[y][x] ^= true;
            }
        }
    }
}
