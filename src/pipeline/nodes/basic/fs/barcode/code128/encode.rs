//! Code 128 (ISO/IEC 15417): printable ASCII in code set B, runs of four or
//! more digits packed two to a symbol in code set C, a mod-103 check symbol.

/// Bar and space widths, in modules, for symbol values 0–106. Each symbol is
/// three bars and three spaces over 11 modules; STOP (106) is 13 with its
/// final bar.
const PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212", "221213",
    "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221", "223211", "221132",
    "221231", "213212", "223112", "312131", "311222", "321122", "321221", "312212", "322112", "322211",
    "212123", "212321", "232121", "111323", "131123", "131321", "112313", "132113", "132311", "211313",
    "231113", "231311", "112133", "112331", "132131", "113123", "113321", "133121", "313121", "211331",
    "231131", "213113", "213311", "213131", "311123", "311321", "331121", "312113", "312311", "332111",
    "314111", "221411", "431111", "111224", "111422", "121124", "121421", "141122", "141221", "112214",
    "112412", "122114", "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111",
    "111242", "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311", "113141",
    "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

const CODE_C: u16 = 99;
const CODE_B: u16 = 100;
const START_B: u16 = 104;
const START_C: u16 = 105;
const STOP: u16 = 106;

/// The symbol values, check symbol and STOP included. `Err` names the first
/// character Code 128 set B cannot carry.
pub fn values(text: &str) -> Result<Vec<u16>, char> {
    if let Some(bad) = text.chars().find(|c| !(' '..='\u{7f}').contains(c)) {
        return Err(bad);
    }
    let bytes = text.as_bytes();
    let digit_run = |from: usize| bytes[from..].iter().take_while(|b| b.is_ascii_digit()).count();

    let mut out = Vec::with_capacity(bytes.len() + 3);
    let mut i = 0;
    let first_run = digit_run(0);
    let mut in_c = first_run >= 4 && first_run % 2 == 0;
    out.push(if in_c { START_C } else { START_B });
    while i < bytes.len() {
        let run = digit_run(i);
        if in_c {
            if run >= 2 {
                out.push(((bytes[i] - b'0') * 10 + (bytes[i + 1] - b'0')) as u16);
                i += 2;
                continue;
            }
            out.push(CODE_B);
            in_c = false;
        }
        // In set B: an odd run of 4+ digits sends one digit here first, then
        // the rest go in pairs.
        if run >= 4 && run % 2 == 0 {
            out.push(CODE_C);
            in_c = true;
            continue;
        }
        out.push((bytes[i] - b' ') as u16);
        i += 1;
    }
    let check = out.iter().enumerate().fold(0u32, |acc, (pos, &v)| acc + v as u32 * (pos.max(1) as u32)) % 103;
    out.push(check as u16);
    out.push(STOP);
    Ok(out)
}

/// One row of modules, `true` = bar, no quiet zone.
pub fn modules(values: &[u16]) -> Vec<bool> {
    let mut row = Vec::with_capacity(values.len() * 11 + 2);
    for &v in values {
        for (k, width) in PATTERNS[v as usize].bytes().enumerate() {
            row.extend(std::iter::repeat_n(k % 2 == 0, (width - b'0') as usize));
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_is_eleven_modules_and_stop_thirteen() {
        for (v, p) in PATTERNS.iter().enumerate() {
            let sum: u32 = p.bytes().map(|b| (b - b'0') as u32).sum();
            assert_eq!(sum, if v == 106 { 13 } else { 11 }, "value {v}");
        }
    }

    /// "PJJ123C" stays in set B (a three-digit run is not worth set C). Check:
    /// 104 + 48·1 + 42·2 + 42·3 + 17·4 + 18·5 + 19·6 + 35·7 = 879; 879 mod 103 = 55.
    #[test]
    fn the_check_symbol_is_the_weighted_sum_mod_103() {
        assert_eq!(values("PJJ123C").unwrap(), vec![104, 48, 42, 42, 17, 18, 19, 35, 55, 106]);
    }

    #[test]
    fn digit_runs_pack_two_to_a_symbol() {
        // An even run from the start begins in set C.
        assert_eq!(&values("12345678").unwrap()[..5], &[105, 12, 34, 56, 78]);
        // An odd run: one digit in B, then C.
        assert_eq!(&values("A12345").unwrap()[..6], &[104, 33, 17, 99, 23, 45]);
        // Short runs stay in B.
        assert_eq!(&values("A12").unwrap()[..4], &[104, 33, 17, 18]);
    }

    #[test]
    fn a_character_outside_ascii_is_refused_by_name() {
        assert_eq!(values("café"), Err('é'));
    }
}

/// Writes PNGs to `<temp>/zebflow-code128-check/` with the expected text
/// beside each, for an outside decoder. Run by hand with `-- --ignored`.
#[cfg(test)]
#[test]
#[ignore]
fn write_barcodes_for_an_outside_decoder() {
    use super::super::render::Drawing;
    let dir = std::env::temp_dir().join("zebflow-code128-check");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let cases = ["CERT-2026-0412", "T-000481", "12345678", "A12345", "Hello, world!", "0012", "x", "PJJ123C", "Order #88213 / batch 7"];
    for (i, text) in cases.iter().enumerate() {
        let row = vec![modules(&values(text).expect("ascii"))];
        let png = Drawing { rows: &row, margin: 10, module_px: 3, row_px: 120, dark: [0, 0, 0], light: [255, 255, 255] }.png().expect("png");
        std::fs::write(dir.join(format!("{i:03}.png")), png).expect("png");
        std::fs::write(dir.join(format!("{i:03}.txt")), text).expect("txt");
    }
}
