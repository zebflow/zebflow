//! A readable line diff for the plan: the old file against the new one,
//! unchanged runs folded to a few lines of context.

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Line<'a> {
    Same(&'a str),
    Gone(&'a str),
    Added(&'a str),
}

/// The longest common subsequence diff of two line lists. Pipelines and
/// pages are hundreds of lines, so the quadratic table is fine; past a
/// ceiling the diff is the whole old file against the whole new one.
fn lines<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Line<'a>> {
    const CEILING: usize = 4_000_000;
    if old.len().saturating_mul(new.len()) > CEILING {
        let mut out: Vec<Line<'a>> = old.iter().map(|l| Line::Gone(l)).collect();
        out.extend(new.iter().map(|l| Line::Added(l)));
        return out;
    }
    let (n, m) = (old.len(), new.len());
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n && j < m {
        if old[i] == new[j] {
            out.push(Line::Same(old[i]));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            out.push(Line::Gone(old[i]));
            i += 1;
        } else {
            out.push(Line::Added(new[j]));
            j += 1;
        }
    }
    out.extend(old[i..].iter().map(|l| Line::Gone(l)));
    out.extend(new[j..].iter().map(|l| Line::Added(l)));
    out
}

/// A unified-style diff of `old` against `new` (`-` gone, `+` added, two
/// lines of context, `…` for a folded run); empty when they are equal.
pub fn unified(old: &str, new: &str) -> String {
    const CONTEXT: usize = 2;
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let diff = lines(&old_lines, &new_lines);
    if diff.iter().all(|l| matches!(l, Line::Same(_))) {
        return String::new();
    }
    let changed: Vec<bool> = diff.iter().map(|l| !matches!(l, Line::Same(_))).collect();
    let near = |index: usize| {
        let start = index.saturating_sub(CONTEXT);
        let end = (index + CONTEXT + 1).min(changed.len());
        changed[start..end].iter().any(|c| *c)
    };
    let mut out = String::new();
    let mut folded = false;
    for (index, line) in diff.iter().enumerate() {
        match line {
            Line::Same(text) if near(index) => {
                out.push_str(&format!("  {text}\n"));
                folded = false;
            }
            Line::Same(_) => {
                if !folded {
                    out.push_str("  …\n");
                    folded = true;
                }
            }
            Line::Gone(text) => {
                out.push_str(&format!("- {text}\n"));
                folded = false;
            }
            Line::Added(text) => {
                out.push_str(&format!("+ {text}\n"));
                folded = false;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::unified;

    #[test]
    fn equal_texts_have_no_diff() {
        assert_eq!(unified("a\nb\n", "a\nb\n"), "");
    }

    #[test]
    fn a_changed_line_shows_with_its_context_and_far_lines_fold() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n";
        let new = "1\n2\n3\n4\nfive\n6\n7\n8\n";
        assert_eq!(unified(old, new), "  …\n  3\n  4\n- 5\n+ five\n  6\n  7\n  …\n");
    }
}
