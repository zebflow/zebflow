//! The capacity tables of ISO/IEC 18004 (QR Code), indexed by version 1–40.
//! Index 0 is unused so a version reads its own row.

/// Error-correction level, in the order the tables below are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecc {
    Low,
    Medium,
    Quartile,
    High,
}

impl Ecc {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "L" | "LOW" => Some(Self::Low),
            "M" | "MEDIUM" | "" => Some(Self::Medium),
            "Q" | "QUARTILE" => Some(Self::Quartile),
            "H" | "HIGH" => Some(Self::High),
            _ => None,
        }
    }

    pub fn letter(self) -> &'static str {
        match self {
            Self::Low => "L",
            Self::Medium => "M",
            Self::Quartile => "Q",
            Self::High => "H",
        }
    }

    fn row(self) -> usize {
        self as usize
    }

    /// The two bits the format information carries for this level.
    pub fn format_bits(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 0,
            Self::Quartile => 3,
            Self::High => 2,
        }
    }
}

pub const MIN_VERSION: usize = 1;
pub const MAX_VERSION: usize = 40;

/// Error-correction codewords in each block.
const ECC_CODEWORDS_PER_BLOCK: [[i16; 41]; 4] = [
    [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
];

/// Error-correction blocks the codewords are split into.
const NUM_ERROR_CORRECTION_BLOCKS: [[i16; 41]; 4] = [
    [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    [-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    [-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81],
];

pub fn ecc_codewords_per_block(ecc: Ecc, version: usize) -> usize {
    ECC_CODEWORDS_PER_BLOCK[ecc.row()][version] as usize
}

pub fn num_blocks(ecc: Ecc, version: usize) -> usize {
    NUM_ERROR_CORRECTION_BLOCKS[ecc.row()][version] as usize
}

/// Side length in modules.
pub fn size(version: usize) -> usize {
    version * 4 + 17
}

/// Modules left for data and error correction once every function pattern
/// (finders, timing, alignment, format and version information) is drawn.
pub fn num_raw_data_modules(version: usize) -> usize {
    let v = version;
    let mut result = (16 * v + 128) * v + 64;
    if v >= 2 {
        let num_align = v / 7 + 2;
        result -= (25 * num_align - 10) * num_align - 55;
        if v >= 7 {
            result -= 36;
        }
    }
    result
}

/// Data codewords this version and level carry, error correction excluded.
pub fn num_data_codewords(version: usize, ecc: Ecc) -> usize {
    num_raw_data_modules(version) / 8 - ecc_codewords_per_block(ecc, version) * num_blocks(ecc, version)
}

/// Centres of the alignment patterns along one axis, ascending.
pub fn alignment_positions(version: usize) -> Vec<usize> {
    if version == 1 {
        return Vec::new();
    }
    let num_align = version / 7 + 2;
    let step = (version * 8 + num_align * 3 + 5) / (num_align * 4 - 4) * 2;
    let last = size(version) - 7;
    let mut result = vec![6usize];
    result.extend((1..num_align).map(|i| last - (num_align - 1 - i) * step));
    result
}
