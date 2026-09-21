//! A complete theme from one seed colour and one mood.
//!
//! The `brand-system` skill's engine. A model asked to "pick colours" picks
//! them per page and the site has no brand; a model asked to compute
//! contrast by hand gets it wrong. So the whole derivation is here, and the
//! MCP tool `theme_generate` hands back every token for light and dark as
//! hex, already contrast-checked, plus the `:root` / `.dark` blocks ready
//! for `globals.css`.
//!
//! Rules (each token is derived, never guessed):
//! - surfaces are the base (white / near-black) tinted with the seed;
//! - text roles are fitted toward black or white in 1 % steps until they meet
//!   4.5:1 against every surface; marks (primary, borders, charts, statuses)
//!   are fitted to 3:1;
//! - a `-foreground` companion is whichever of black/white contrasts more
//!   with its surface;
//! - hues for `secondary` and the charts rotate from the seed's hue; a grey
//!   seed takes the mood's hue.
//!
//! Method after a design by GPT-6-Astra (2026-09-14), ported and tested here.

use std::collections::BTreeMap;

use serde::Serialize;

const WHITE: &str = "#FFFFFF";
const BLACK: &str = "#000000";
const INK: &str = "#111318";

/// The moods a brief maps to. Each carries the hue used when the seed is
/// grey, and the rotation that gives `secondary` its own hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Calm,
    Warm,
    Serious,
    Energetic,
    Playful,
    Luxurious,
    Technical,
}

impl Mood {
    pub const ALL: [Mood; 7] = [
        Mood::Calm,
        Mood::Warm,
        Mood::Serious,
        Mood::Energetic,
        Mood::Playful,
        Mood::Luxurious,
        Mood::Technical,
    ];

    pub fn parse(raw: &str) -> Option<Mood> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "calm" => Some(Mood::Calm),
            "warm" => Some(Mood::Warm),
            "serious" => Some(Mood::Serious),
            "energetic" => Some(Mood::Energetic),
            "playful" => Some(Mood::Playful),
            "luxurious" => Some(Mood::Luxurious),
            "technical" => Some(Mood::Technical),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mood::Calm => "calm",
            Mood::Warm => "warm",
            Mood::Serious => "serious",
            Mood::Energetic => "energetic",
            Mood::Playful => "playful",
            Mood::Luxurious => "luxurious",
            Mood::Technical => "technical",
        }
    }

    /// (hue when the seed is grey, hue rotation for `secondary`)
    fn hues(self) -> (f64, f64) {
        match self {
            Mood::Calm => (210.0, 30.0),
            Mood::Warm => (25.0, 40.0),
            Mood::Serious => (220.0, 30.0),
            Mood::Energetic => (20.0, 150.0),
            Mood::Playful => (285.0, 120.0),
            Mood::Luxurious => (40.0, 160.0),
            Mood::Technical => (200.0, 60.0),
        }
    }

    /// The seed a brief without a colour gets.
    pub fn default_seed(self) -> &'static str {
        match self {
            Mood::Calm => "#1E66D6",
            Mood::Warm => "#C2410C",
            Mood::Serious => "#1D4ED8",
            Mood::Energetic => "#D1440B",
            Mood::Playful => "#DB2777",
            Mood::Luxurious => "#7C2D12",
            Mood::Technical => "#0F766E",
        }
    }
}

/// Type, radius, density and shadow for a mood — the non-colour half of a
/// brand, as a table the skill copies.
#[derive(Debug, Clone, Serialize)]
pub struct MoodStyle {
    pub display_font: &'static str,
    pub display_fallback: &'static str,
    pub sans_font: &'static str,
    pub h1_px: (u16, u16),
    pub h2_px: (u16, u16),
    pub display_line_height: f32,
    pub body_line_height: f32,
    pub radius_rem: f32,
    pub control_height_px: u16,
    pub card_padding_px: (u16, u16),
    pub grid_gap_px: u16,
    pub section_gap_px: (u16, u16),
    pub card_shadow: &'static str,
    pub fonts_href: String,
}

pub fn mood_style(mood: Mood) -> MoodStyle {
    let (display, fallback, sans, h1, h2, dlh, blh, radius, control, pad, gap, section, shadow) = match mood {
        Mood::Calm => ("Lora", "serif", "Inter", (48, 32), (32, 24), 1.2, 1.6, 0.5, 48, (16, 24), 24, (48, 64), "0 2px 8px color-mix(in srgb, var(--foreground) 6%, transparent)"),
        Mood::Warm => ("Fraunces", "serif", "Nunito Sans", (48, 32), (32, 24), 1.15, 1.6, 0.75, 48, (16, 24), 24, (48, 64), "0 2px 8px color-mix(in srgb, var(--foreground) 6%, transparent)"),
        Mood::Serious => ("Source Serif 4", "serif", "Source Sans 3", (48, 32), (32, 24), 1.2, 1.6, 0.25, 44, (16, 24), 24, (32, 48), "none"),
        Mood::Energetic => ("Montserrat", "sans-serif", "Source Sans 3", (64, 40), (40, 32), 1.1, 1.5, 0.5, 48, (16, 24), 24, (48, 64), "0 4px 16px color-mix(in srgb, var(--foreground) 10%, transparent)"),
        Mood::Playful => ("Nunito", "sans-serif", "DM Sans", (48, 32), (32, 24), 1.15, 1.6, 1.0, 48, (24, 32), 24, (48, 64), "0 4px 16px color-mix(in srgb, var(--foreground) 10%, transparent)"),
        Mood::Luxurious => ("Playfair Display", "serif", "Manrope", (64, 40), (40, 32), 1.15, 1.7, 0.0, 48, (24, 32), 32, (64, 96), "0 2px 8px color-mix(in srgb, var(--foreground) 6%, transparent)"),
        Mood::Technical => ("Space Grotesk", "sans-serif", "IBM Plex Sans", (48, 32), (32, 24), 1.15, 1.5, 0.375, 44, (16, 16), 16, (32, 48), "none"),
    };
    let fonts_href = format!(
        "https://fonts.googleapis.com/css2?family={}:wght@600&family={}:wght@400;600&display=swap",
        display.replace(' ', "+"),
        sans.replace(' ', "+")
    );
    MoodStyle {
        display_font: display,
        display_fallback: fallback,
        sans_font: sans,
        h1_px: h1,
        h2_px: h2,
        display_line_height: dlh,
        body_line_height: blh,
        radius_rem: radius,
        control_height_px: control,
        card_padding_px: pad,
        grid_gap_px: gap,
        section_gap_px: section,
        card_shadow: shadow,
        fonts_href,
    }
}

/// One measured pair from the report.
#[derive(Debug, Clone, Serialize)]
pub struct ContrastRow {
    pub text: String,
    pub surface: String,
    pub light: f64,
    pub dark: f64,
    pub minimum: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedTheme {
    pub seed: String,
    pub mood: Mood,
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
    /// `:root { … }` and `.dark { … }` with every token and `--radius` — paste
    /// over the two blocks in `globals.css`.
    pub css: String,
    pub style: MoodStyle,
    pub contrast: Vec<ContrastRow>,
    pub notes: Vec<String>,
    /// `@font-face` rules for the mood's two families when the project holds
    /// them under `static/fonts/` — then the page loads its own files and
    /// `style.fonts_href` (the Google Fonts link) is not needed. Absent when
    /// either family has no file there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fonts_css: Option<String>,
}

// ── colour maths ─────────────────────────────────────────────────────────────

fn rgb(hex: &str) -> [f64; 3] {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64;
    [p(0), p(2), p(4)]
}

fn hex(c: [f64; 3]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        c[0].round().clamp(0.0, 255.0) as u8,
        c[1].round().clamp(0.0, 255.0) as u8,
        c[2].round().clamp(0.0, 255.0) as u8
    )
}

fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (rgb(a), rgb(b));
    hex([
        a[0] * (1.0 - t) + b[0] * t,
        a[1] * (1.0 - t) + b[1] * t,
        a[2] * (1.0 - t) + b[2] * t,
    ])
}

fn luminance(hex: &str) -> f64 {
    let lin = |v: f64| {
        let v = v / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    let c = rgb(hex);
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG contrast ratio.
pub fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn worst(c: &str, surfaces: &[String]) -> f64 {
    surfaces.iter().map(|s| contrast(c, s)).fold(f64::INFINITY, f64::min)
}

fn foreground_for(surface: &str) -> String {
    if contrast(BLACK, surface) >= contrast(WHITE, surface) { BLACK } else { WHITE }.to_string()
}

/// Move `c` toward black or white — whichever end serves the surfaces
/// better — in 1 % steps until it meets `min` against every surface.
fn fit(c: &str, surfaces: &[String], min: f64) -> String {
    let end = if worst(BLACK, surfaces) >= worst(WHITE, surfaces) { BLACK } else { WHITE };
    for i in 0..=100 {
        let x = mix(c, end, i as f64 / 100.0);
        if worst(&x, surfaces) >= min {
            return x;
        }
    }
    end.to_string()
}

fn hue_sat(hex: &str) -> (f64, f64) {
    let c = rgb(hex).map(|v| v / 255.0);
    let (r, g, b) = (c[0], c[1], c[2]);
    let hi = r.max(g).max(b);
    let lo = r.min(g).min(b);
    let d = hi - lo;
    let l = (hi + lo) / 2.0;
    if d == 0.0 {
        return (0.0, 0.0);
    }
    let h = if hi == r {
        ((g - b) / d) % 6.0
    } else if hi == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    ((h * 60.0 + 360.0) % 360.0, d / (1.0 - (2.0 * l - 1.0).abs()))
}

fn hsl(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = ((h % 360.0) + 360.0) % 360.0 / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if hp < 1.0 {
        (c, x, 0.0)
    } else if hp < 2.0 {
        (x, c, 0.0)
    } else if hp < 3.0 {
        (0.0, c, x)
    } else if hp < 4.0 {
        (0.0, x, c)
    } else if hp < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    hex([(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0])
}

const SURFACE_KEYS: [&str; 8] = ["background", "card", "popover", "muted", "secondary", "accent", "sidebar", "sidebar-accent"];

fn one_mode(seed: &str, mood: Mood, dark: bool) -> BTreeMap<String, String> {
    let (fallback_hue, turn) = mood.hues();
    let (mut h, mut s) = hue_sat(seed);
    if s < 0.08 {
        h = fallback_hue;
    }
    s = s.clamp(0.45, 0.75);
    let base = if dark { INK } else { WHITE };
    let mut t: BTreeMap<String, String> = BTreeMap::new();
    fn set(t: &mut BTreeMap<String, String>, k: &str, v: String) {
        t.insert(k.to_string(), v);
    }
    let bg = mix(base, seed, if dark { 0.06 } else { 0.02 });
    set(&mut t, "background", bg.clone());
    let card = if dark { mix(&bg, WHITE, 0.04) } else { WHITE.to_string() };
    set(&mut t, "card", card.clone());
    set(&mut t, "popover", if dark { mix(&bg, WHITE, 0.08) } else { WHITE.to_string() });
    set(&mut t, "muted", mix(&bg, if dark { WHITE } else { INK }, if dark { 0.08 } else { 0.05 }));
    set(&mut t, "secondary", mix(&bg, &hsl(h + turn, s, 0.5), if dark { 0.18 } else { 0.10 }));
    let accent = mix(&bg, seed, if dark { 0.20 } else { 0.12 });
    set(&mut t, "accent", accent.clone());
    set(&mut t, "sidebar", card);
    set(&mut t, "sidebar-accent", accent);
    let surfaces: Vec<String> = SURFACE_KEYS.iter().map(|k| t[*k].clone()).collect();

    let foreground = fit(if dark { "#F5F5F5" } else { INK }, &surfaces, 4.5);
    set(&mut t, "foreground", foreground.clone());
    // A surface's text is the page's text when it reads there (it nearly
    // always does — pure black on a white card beside #111318 body text is
    // a visible seam); black or white only when the surface is too dark or
    // too saturated for it.
    for k in ["card", "popover", "secondary", "accent", "sidebar", "sidebar-accent"] {
        let v = if contrast(&foreground, &t[k]) >= 4.5 { foreground.clone() } else { foreground_for(&t[k]) };
        set(&mut t, &format!("{k}-foreground"), v);
    }
    set(&mut t, "sidebar-foreground", foreground.clone());
    set(&mut t, "muted-foreground", fit(&mix(&foreground, &bg, 0.40), &surfaces, 4.5));
    let primary = fit(seed, &surfaces, 3.0);
    let primary_fg = foreground_for(&primary);
    set(&mut t, "primary", primary.clone());
    set(&mut t, "primary-foreground", primary_fg.clone());

    let families = [
        ("success", "#15803D", "#4ADE80"),
        ("warning", "#A16207", "#FACC15"),
        ("info", "#0369A1", "#38BDF8"),
        ("destructive", "#B91C1C", "#F87171"),
    ];
    for (k, light, darkv) in families {
        let v = fit(if dark { darkv } else { light }, &surfaces, 3.0);
        set(&mut t, &format!("{k}-foreground"), foreground_for(&v));
        set(&mut t, k, v);
    }
    let border = fit(
        &mix(&bg, &foreground, 0.16),
        &[t["background"].clone(), t["card"].clone(), t["popover"].clone()],
        1.5,
    );
    set(&mut t, "border", border.clone());
    set(&mut t, "input", fit(&border, &surfaces, 3.0));
    let ring = fit(&primary, &surfaces, 3.0);
    set(&mut t, "ring", ring.clone());
    for i in 0..5 {
        let v = fit(&hsl(h + i as f64 * 72.0, s, if dark { 0.65 } else { 0.42 }), &surfaces, 3.0);
        set(&mut t, &format!("chart-{}", i + 1), v);
    }
    set(&mut t, "sidebar-primary", primary);
    set(&mut t, "sidebar-primary-foreground", primary_fg);
    set(&mut t, "sidebar-border", border);
    set(&mut t, "sidebar-ring", ring);
    t
}

/// Every colour token the platform's `globals.css` carries, in the order the
/// CSS is written.
pub const TOKEN_ORDER: [&str; 38] = [
    "background", "foreground", "card", "card-foreground", "popover", "popover-foreground",
    "primary", "primary-foreground", "secondary", "secondary-foreground", "muted", "muted-foreground",
    "accent", "accent-foreground", "destructive", "destructive-foreground", "success", "success-foreground",
    "warning", "warning-foreground", "info", "info-foreground", "border", "input", "ring",
    "chart-1", "chart-2", "chart-3", "chart-4", "chart-5",
    "sidebar", "sidebar-foreground", "sidebar-primary", "sidebar-primary-foreground",
    "sidebar-accent", "sidebar-accent-foreground", "sidebar-border", "sidebar-ring",
];

fn css_block(selector: &str, tokens: &BTreeMap<String, String>, radius_rem: f32) -> String {
    let mut out = format!("{selector} {{\n");
    for k in TOKEN_ORDER {
        out.push_str(&format!("  --{k}: {};\n", tokens[k].to_ascii_lowercase()));
    }
    out.push_str(&format!("  --radius: {radius_rem}rem;\n}}\n"));
    out
}

/// The whole theme. `seed` is `#RRGGBB` (case-insensitive); an invalid seed
/// is an `Err` naming the rule.
pub fn generate(seed: &str, mood: Mood) -> Result<GeneratedTheme, String> {
    let seed = seed.trim();
    let valid = seed.len() == 7
        && seed.starts_with('#')
        && seed[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(format!("seed must be #RRGGBB, got `{seed}`"));
    }
    let seed = seed.to_ascii_uppercase();
    let light = one_mode(&seed, mood, false);
    let dark = one_mode(&seed, mood, true);
    let style = mood_style(mood);
    let mut notes = Vec::new();
    if light["primary"] != seed {
        notes.push(format!("primary on light was moved from {seed} to {} to reach 3:1 against the surfaces", light["primary"]));
    }
    if dark["primary"] != seed {
        notes.push(format!("primary on dark was moved from {seed} to {} to stay visible on dark surfaces", dark["primary"]));
    }
    if hue_sat(&seed).1 < 0.08 {
        notes.push(format!("the seed is grey; secondary and chart hues come from the mood ({})", mood.key()));
    }
    let pairs: [(&str, &str, f64); 9] = [
        ("foreground", "background", 4.5),
        ("muted-foreground", "background", 4.5),
        ("muted-foreground", "card", 4.5),
        ("card-foreground", "card", 4.5),
        ("primary-foreground", "primary", 4.5),
        ("secondary-foreground", "secondary", 4.5),
        ("accent-foreground", "accent", 4.5),
        ("primary", "background", 3.0),
        ("input", "background", 3.0),
    ];
    let contrast_rows = pairs
        .iter()
        .map(|(text, surface, minimum)| ContrastRow {
            text: text.to_string(),
            surface: surface.to_string(),
            light: (contrast(&light[*text], &light[*surface]) * 100.0).round() / 100.0,
            dark: (contrast(&dark[*text], &dark[*surface]) * 100.0).round() / 100.0,
            minimum: *minimum,
        })
        .collect();
    let css = format!(
        "{}\n{}",
        css_block(":root", &light, style.radius_rem),
        css_block(".dark", &dark, style.radius_rem)
    );
    Ok(GeneratedTheme { seed, mood, light, dark, css, style, contrast: contrast_rows, notes, fonts_css: None })
}

/// `@font-face` rules for the mood's display and sans families from the
/// files a project has under `static/fonts/` (bare names, any of `.woff2`,
/// `.otf`, `.ttf`), served at `/static/{owner}/{project}/fonts/…`. A file
/// belongs to a family when its name starts with the family's name without
/// spaces (`SourceSerif4-SemiBold.woff2`); the weight is read from a
/// `-<Weight>` or `-<number>` suffix, `400` otherwise. `None` unless both
/// families have at least one file — half a pairing is the Google link.
pub fn font_face_css(style: &MoodStyle, files: &[String], owner: &str, project: &str) -> Option<String> {
    fn weight_of(stem: &str) -> u16 {
        let tail = stem.rsplit_once('-').map(|(_, t)| t.to_ascii_lowercase()).unwrap_or_default();
        match tail.as_str() {
            "thin" => 100, "extralight" | "ultralight" => 200, "light" => 300, "regular" | "book" | "" => 400,
            "medium" => 500, "semibold" | "demibold" => 600, "bold" => 700, "extrabold" | "ultrabold" => 800, "black" | "heavy" => 900,
            other => other.parse().unwrap_or(400),
        }
    }
    let mut css = String::new();
    for family in [style.display_font, style.sans_font] {
        let key = family.replace(' ', "").to_ascii_lowercase();
        let mut found = false;
        for file in files {
            let Some((stem, ext)) = file.rsplit_once('.') else { continue };
            let format = match ext.to_ascii_lowercase().as_str() { "woff2" => "woff2", "otf" => "opentype", "ttf" => "truetype", _ => continue };
            if !stem.to_ascii_lowercase().starts_with(&key) {
                continue;
            }
            found = true;
            css.push_str(&format!(
                "@font-face {{ font-family: \"{family}\"; font-weight: {}; font-display: swap; src: url(\"/static/{owner}/{project}/fonts/{file}\") format(\"{format}\"); }}\n",
                weight_of(stem)
            ));
        }
        if !found {
            return None;
        }
    }
    Some(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_default_seed() -> Vec<(String, Mood)> {
        Mood::ALL.iter().map(|m| (m.default_seed().to_string(), *m)).collect()
    }

    /// The guarantee the skill relies on: for every mood's default seed and
    /// a handful of awkward seeds, both modes carry every token as hex and
    /// every text pair meets 4.5:1, every mark 3:1, against all surfaces.
    #[test]
    fn every_generated_theme_is_complete_and_meets_contrast() {
        let mut cases = every_default_seed();
        for s in ["#FFFF00", "#000000", "#FFFFFF", "#808080", "#EA5A0C", "#00FF88"] {
            cases.push((s.to_string(), Mood::Calm));
        }
        for (seed, mood) in cases {
            let theme = generate(&seed, mood).expect("generates");
            for (mode, tokens) in [("light", &theme.light), ("dark", &theme.dark)] {
                assert_eq!(tokens.len(), 38, "{seed} {mode}: 38 tokens");
                for k in TOKEN_ORDER {
                    let v = &tokens[k];
                    assert!(v.len() == 7 && v.starts_with('#'), "{seed} {mode} {k} = {v}");
                }
                let surfaces: Vec<String> = SURFACE_KEYS.iter().map(|k| tokens[*k].clone()).collect();
                for k in ["foreground", "muted-foreground"] {
                    assert!(worst(&tokens[k], &surfaces) >= 4.5, "{seed} {mode} {k} on surfaces = {}", worst(&tokens[k], &surfaces));
                }
                for (k, v) in tokens {
                    if let Some(surface) = k.strip_suffix("-foreground") {
                        let base = if surface.is_empty() { "background" } else { surface };
                        assert!(contrast(v, &tokens[base]) >= 4.5, "{seed} {mode} {k} on {base} = {}", contrast(v, &tokens[base]));
                    }
                }
                for k in ["primary", "success", "warning", "info", "destructive", "input", "ring", "chart-1", "chart-2", "chart-3", "chart-4", "chart-5"] {
                    assert!(worst(&tokens[k], &surfaces) >= 3.0, "{seed} {mode} {k} = {}", worst(&tokens[k], &surfaces));
                }
            }
            assert!(theme.css.contains(":root {") && theme.css.contains(".dark {") && theme.css.contains("--radius:"));
        }
    }

    #[test]
    fn the_css_names_exactly_the_platform_tokens() {
        let mut expected: Vec<&str> = crate::platform::theme::token_names().into_iter().filter(|t| *t != "radius").collect();
        expected.sort();
        let mut ours: Vec<&str> = TOKEN_ORDER.to_vec();
        ours.sort();
        assert_eq!(ours, expected, "the generator and globals.css name the same tokens");
    }

    #[test]
    fn a_bad_seed_is_refused_with_the_rule() {
        assert!(generate("blue", Mood::Calm).is_err());
        assert!(generate("#12345", Mood::Calm).is_err());
        assert!(generate(" #1e66d6 ", Mood::Calm).is_ok());
    }

    #[test]
    fn font_face_rules_come_from_the_project_files_or_not_at_all() {
        let style = mood_style(Mood::Serious);
        let files: Vec<String> = ["SourceSerif4-SemiBold.woff2", "SourceSans3-Regular.woff2", "SourceSans3-600.ttf", "Other-Bold.otf"].iter().map(|s| s.to_string()).collect();
        let css = font_face_css(&style, &files, "acme", "site").expect("both families present");
        assert!(css.contains("font-family: \"Source Serif 4\"; font-weight: 600") && css.contains("/static/acme/site/fonts/SourceSerif4-SemiBold.woff2\") format(\"woff2\")"), "{css}");
        assert!(css.contains("font-family: \"Source Sans 3\"; font-weight: 400") && css.contains("font-weight: 600; font-display: swap; src: url(\"/static/acme/site/fonts/SourceSans3-600.ttf\") format(\"truetype\")"), "{css}");
        assert!(!css.contains("Other-Bold"));
        assert_eq!(css.matches("@font-face").count(), 3);
        // Half a pairing stays on the Google link.
        assert!(font_face_css(&style, &files[..1], "acme", "site").is_none());
        assert!(font_face_css(&style, &[], "acme", "site").is_none());
    }

    #[test]
    fn the_mood_style_builds_a_google_fonts_link() {
        let s = mood_style(Mood::Serious);
        assert!(s.fonts_href.starts_with("https://fonts.googleapis.com/css2?family=Source+Serif+4:wght@600&family=Source+Sans+3"));
        assert!(s.fonts_href.ends_with("display=swap"));
    }
}
