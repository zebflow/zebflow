use super::encode::{codewords, encode};
use super::tables::{self, Ecc};

/// Data codewords per version and level, from the standard's capacity table.
#[test]
fn capacities_match_the_standard() {
    let table = [
        (1, [19, 16, 13, 9]),
        (2, [34, 28, 22, 16]),
        (5, [108, 86, 62, 46]),
        (10, [274, 216, 154, 122]),
        (40, [2956, 2334, 1666, 1276]),
    ];
    for (version, per_level) in table {
        for (ecc, expected) in [Ecc::Low, Ecc::Medium, Ecc::Quartile, Ecc::High].into_iter().zip(per_level) {
            assert_eq!(tables::num_data_codewords(version, ecc), expected, "v{version} {}", ecc.letter());
        }
    }
}

#[test]
fn alignment_patterns_sit_where_the_standard_puts_them() {
    assert_eq!(tables::alignment_positions(1), Vec::<usize>::new());
    assert_eq!(tables::alignment_positions(2), vec![6, 18]);
    assert_eq!(tables::alignment_positions(7), vec![6, 22, 38]);
    assert_eq!(tables::alignment_positions(14), vec![6, 26, 46, 66]);
    assert_eq!(tables::alignment_positions(32), vec![6, 34, 60, 86, 112, 138]);
    assert_eq!(tables::alignment_positions(40), vec![6, 30, 58, 86, 114, 142, 170]);
    for version in 2..=40 {
        let p = tables::alignment_positions(version);
        assert_eq!(p.len(), version / 7 + 2, "v{version}");
        assert_eq!(*p.last().unwrap(), tables::size(version) - 7, "v{version}");
        assert!(p.windows(2).all(|w| w[0] < w[1]), "v{version} ascending");
    }
}

/// The widely published worked example: "HELLO WORLD" at 1-M.
#[test]
fn hello_world_gives_the_published_codewords() {
    let (version, words) = codewords("HELLO WORLD", Ecc::Medium).expect("fits");
    assert_eq!(version, 1);
    assert_eq!(
        words,
        vec![32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17, 196, 35, 39, 119, 235, 215, 231, 226, 93, 23]
    );
}

/// Read the 15 format bits back from the first copy, undo the XOR mask, and
/// check the BCH remainder: a scanner does exactly this first.
fn format_word(m: &[Vec<bool>]) -> u32 {
    let mut bits = 0u32;
    let mut put = |i: usize, dark: bool| bits |= (dark as u32) << i;
    for i in 0..=5 {
        put(i, m[i][8]);
    }
    put(6, m[7][8]);
    put(7, m[8][8]);
    put(8, m[8][7]);
    for i in 9..15 {
        put(i, m[8][14 - i]);
    }
    bits ^ 0x5412
}

#[test]
fn every_symbol_carries_readable_structure() {
    let long = "x".repeat(600);
    for (text, ecc) in [("HELLO WORLD", Ecc::Medium), ("https://example.com/certificates/CERT-2026-0412", Ecc::High), ("0123456789", Ecc::Low), (long.as_str(), Ecc::Quartile)] {
        let s = encode(text, ecc).expect("fits");
        let n = s.size;
        assert_eq!(n, tables::size(s.version));
        // The three finder centres are dark, surrounded by a light ring.
        for (x, y) in [(3, 3), (n - 4, 3), (3, n - 4)] {
            assert!(s.modules[y][x] && !s.modules[y][x - 2] && s.modules[y][x - 3]);
        }
        // Timing alternates between the finders.
        for i in 8..n - 8 {
            assert_eq!(s.modules[6][i], i % 2 == 0);
        }
        let word = format_word(&s.modules);
        let mut rem = word >> 10;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        assert_eq!(word & 0x3FF, rem & 0x3FF, "format BCH for {text:.20}");
        assert_eq!(word >> 13, ecc.format_bits());
        assert_eq!((word >> 10) & 7, s.mask as u32);
    }
}

#[test]
fn text_beyond_version_40_is_refused() {
    assert!(encode(&"x".repeat(2953), Ecc::Low).is_some());
    assert!(encode(&"x".repeat(2954), Ecc::Low).is_none());
}

/// Writes a spread of symbols as PNG to `<temp>/zebflow-qr-check/` with the
/// expected text beside each, for decoding by an independent scanner. Run by
/// hand: `cargo test --lib qr::tests::write_symbols_for_an_outside_decoder -- --ignored`.
#[test]
#[ignore]
fn write_symbols_for_an_outside_decoder() {
    use super::super::render::Drawing;
    let dir = std::env::temp_dir().join("zebflow-qr-check");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let mut cases: Vec<(String, Ecc)> = Vec::new();
    for ecc in [Ecc::Low, Ecc::Medium, Ecc::Quartile, Ecc::High] {
        cases.push(("HELLO WORLD".into(), ecc));
        cases.push(("https://example.com/certificates/CERT-2026-0412".into(), ecc));
        cases.push(("31415926535897932384626433832795028841971693993751".into(), ecc));
        cases.push(("Sertifikat · Peserta — Seminar Kesehatan Ibu ✓".into(), ecc));
        for len in [80, 300, 700, 1200] {
            let text: String = (0..len).map(|i| char::from(b'a' + (i * 7 % 26) as u8)).collect();
            cases.push((text, ecc));
        }
    }
    cases.push(("z".repeat(2900), Ecc::Low)); // version 40
    for (i, (text, ecc)) in cases.iter().enumerate() {
        let s = encode(text, *ecc).expect("fits");
        let png = Drawing { rows: &s.modules, margin: 4, module_px: 4, row_px: 4, dark: [0, 0, 0], light: [255, 255, 255] }
            .png()
            .expect("png");
        std::fs::write(dir.join(format!("{i:03}-v{}-{}.png", s.version, ecc.letter())), png).expect("png");
        std::fs::write(dir.join(format!("{i:03}.txt")), text).expect("txt");
        let bits: String = s.modules.iter().map(|row| row.iter().map(|&d| if d { '1' } else { '0' }).collect::<String>() + "\n").collect();
        std::fs::write(dir.join(format!("{i:03}.bits")), format!("{} {} {}\n{bits}", s.version, ecc.letter(), s.mask)).expect("bits");
    }
}

/// When the stream already ends on a byte boundary after the terminator, the
/// pad codewords follow at once: no extra zero byte (a mistake one reference
/// encoder makes; the standard pads bits only to reach a boundary).
#[test]
fn an_aligned_stream_goes_straight_to_pad_codewords() {
    // 4 + 8 + 49 × 8 + 4 = 408 bits: aligned after the terminator.
    let text = "https://example.com/certificates/CERT-2026-000412";
    assert_eq!(text.len(), 49);
    let (version, words) = codewords(text, Ecc::Low).expect("fits");
    assert_eq!(version, 3);
    assert_eq!(&words[50..55], &[32, 236, 17, 236, 17]);
}
