//! The mask penalty of ISO/IEC 18004 §7.8.3: the mask with the lowest score
//! gives the symbol a scanner reads most easily. Every mask decodes; this
//! only chooses among them.

const N1: u32 = 3;
const N2: u32 = 3;
const N3: u32 = 40;
const N4: u32 = 10;

/// A finder-like run 1:1:3:1:1 with four light modules on one side.
const FINDER_LIKE: [bool; 11] = [true, false, true, true, true, false, true, false, false, false, false];

pub fn score(modules: &[Vec<bool>]) -> u32 {
    let size = modules.len();
    let row = |i: usize| -> Vec<bool> { modules[i].clone() };
    let col = |i: usize| -> Vec<bool> { (0..size).map(|y| modules[y][i]).collect() };

    let mut total = 0;
    for i in 0..size {
        for line in [row(i), col(i)] {
            total += runs(&line) + finder_like(&line);
        }
    }
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let c = modules[y][x];
            if c == modules[y][x + 1] && c == modules[y + 1][x] && c == modules[y + 1][x + 1] {
                total += N2;
            }
        }
    }
    let dark = modules.iter().flatten().filter(|&&m| m).count() as u32;
    let all = (size * size) as u32;
    // Each 5% step away from half dark costs N4.
    let k = ((dark * 20).abs_diff(all * 10)).div_ceil(all).saturating_sub(1);
    total + k * N4
}

/// Rule 1: five or more same-coloured modules in a row.
fn runs(line: &[bool]) -> u32 {
    let mut score = 0;
    let mut run = 1;
    for i in 1..=line.len() {
        if i < line.len() && line[i] == line[i - 1] {
            run += 1;
        } else {
            if run >= 5 {
                score += N1 + (run - 5);
            }
            run = 1;
        }
    }
    score
}

/// Rule 3: the 1:1:3:1:1 finder shape with a light margin, either way round.
fn finder_like(line: &[bool]) -> u32 {
    if line.len() < FINDER_LIKE.len() {
        return 0;
    }
    let reversed: Vec<bool> = FINDER_LIKE.iter().rev().copied().collect();
    line.windows(FINDER_LIKE.len())
        .map(|w| u32::from(w == FINDER_LIKE) + u32::from(w == reversed.as_slice()))
        .sum::<u32>()
        * N3
}
