//! Stylesheet order, as Tailwind CSS 3.4 emits it.
//!
//! Two rules with the same specificity are decided by which comes later, so
//! the order is the cascade. Tailwind never orders by class name: it orders
//! by variant (none first, then each variant in its registration order, a
//! stacked token by its latest variant), then by core plugin (padding before
//! text colour, `transition` before `duration`), then inside a plugin by how
//! much of the box a utility covers (`p-4`, then `px-4`, then `pl-2`), so the
//! narrower utility always refines the broader one. Only a tie falls back to
//! the class name.

/// Where a utility's plugin sits in Tailwind's core plugin order.
///
/// The declared properties say what a utility is — `border-t-2` and
/// `border-t-red-500` share a prefix but not a plugin — so the first
/// declaration decides, after the few utilities whose name says it better.
pub(super) fn family_rank(utility: &str, declarations: &str) -> u16 {
    let name = utility.trim_start_matches('!').trim_start_matches('-');
    for (prefix, rank) in NAME_FAMILIES {
        if name == prefix.trim_end_matches('-') || name.starts_with(prefix) {
            if *prefix == "divide-" {
                return divide_rank(declarations);
            }
            return *rank;
        }
    }
    first_property(declarations)
        .and_then(property_rank)
        .unwrap_or(UNKNOWN)
}

/// Inside one plugin, broader first: a shorthand covers four sides, each
/// longhand one. `inset-0` (4) → `inset-x-0` (2) → `left-0` (1).
pub(super) fn narrowness(declarations: &str) -> u8 {
    let covered: u32 = properties(declarations)
        .filter(|prop| !prop.starts_with("--"))
        .map(|prop| if SHORTHANDS.contains(&prop) { 4 } else { 1 })
        .sum();
    u8::MAX - covered.min(u8::MAX as u32) as u8
}

/// A variant's place in Tailwind's variant order; one this engine does not
/// know sorts after every known variant.
pub(super) fn variant_rank(variant: &str) -> u16 {
    if variant == "*" || variant == "**" {
        return 1;
    }
    if let Some(index) = PSEUDO_ELEMENTS.iter().position(|v| *v == variant) {
        return 10 + index as u16;
    }
    if let Some(index) = PSEUDO_CLASSES.iter().position(|v| *v == variant) {
        return 30 + index as u16;
    }
    let base = |state: &str| {
        let state = state.split('/').next().unwrap_or(state);
        PSEUDO_CLASSES.iter().position(|v| *v == state).unwrap_or(PSEUDO_CLASSES.len())
    };
    if let Some(state) = variant.strip_prefix("group-") {
        return 70 + base(state) as u16;
    }
    if let Some(state) = variant.strip_prefix("peer-") {
        return 110 + base(state) as u16;
    }
    if variant.starts_with("has-") {
        return 150;
    }
    if variant.starts_with("aria-") {
        return 160;
    }
    if variant.starts_with("data-") {
        return 170;
    }
    // Tailwind registers these after every other variant, in this order
    // (`setupContextUtils` afterVariants): screens before dark, print last.
    if variant.starts_with("supports-") {
        return 180;
    }
    match variant {
        "motion-safe" => 190,
        "motion-reduce" => 191,
        "contrast-more" => 195,
        "contrast-less" => 196,
        "max-2xl" => 200,
        "max-xl" => 201,
        "max-lg" => 202,
        "max-md" => 203,
        "max-sm" => 204,
        "sm" => 210,
        "md" => 211,
        "lg" => 212,
        "xl" => 213,
        "2xl" => 214,
        "portrait" => 220,
        "landscape" => 221,
        "ltr" => 230,
        "rtl" => 231,
        "light" => 240,
        "dark" => 241,
        "forced-colors" => 250,
        "print" => 260,
        _ => 300,
    }
}

const UNKNOWN: u16 = 900;

/// Utilities whose name names the plugin better than their first property.
const NAME_FAMILIES: &[(&str, u16)] = &[
    ("container", 1),
    ("sr-only", 2),
    ("not-sr-only", 2),
    ("line-clamp-", 42),
    ("size-", 46),
    ("space-", 186),
    ("divide-", 187),
    ("truncate", 196),
    ("placeholder-", 316),
];

/// `divide-y` sets a width, `divide-dashed` a style, `divide-red-500` a
/// colour — Tailwind's divideWidth, divideStyle and divideColor, in order.
fn divide_rank(declarations: &str) -> u16 {
    match first_property(declarations) {
        Some(p) if p.ends_with("-style") => 188,
        Some(p) if p.ends_with("-color") => 189,
        _ => 187,
    }
}

/// Core plugin order (tailwindcss 3.4 `corePlugins`), by the property a
/// plugin writes first. Gaps leave room; only the relative order matters.
fn property_rank(property: &str) -> Option<u16> {
    let p = property
        .strip_prefix("-webkit-")
        .or_else(|| property.strip_prefix("-moz-"))
        .unwrap_or(property);
    let rank = match p {
        "pointer-events" => 3,
        "visibility" => 4,
        "position" => 5,
        "inset" | "top" | "right" | "bottom" | "left" | "inset-inline-start" | "inset-inline-end" => 6,
        "isolation" => 7,
        "z-index" => 8,
        "order" => 9,
        "grid-column" => 10,
        "grid-column-start" => 11,
        "grid-column-end" => 12,
        "grid-row" => 13,
        "grid-row-start" => 14,
        "grid-row-end" => 15,
        "float" => 16,
        "clear" => 17,
        _ if p == "margin" || p.starts_with("margin-") => 20,
        "box-sizing" => 30,
        "line-clamp" | "box-orient" => 42,
        "display" => 44,
        "aspect-ratio" => 45,
        "height" => 47,
        "max-height" => 48,
        "min-height" => 49,
        "width" => 50,
        "min-width" => 51,
        "max-width" => 52,
        "flex" => 53,
        "flex-shrink" => 54,
        "flex-grow" => 55,
        "flex-basis" => 56,
        "table-layout" => 57,
        "caption-side" => 58,
        "border-collapse" => 59,
        "--tw-border-spacing-x" | "--tw-border-spacing-y" | "border-spacing" => 60,
        "transform-origin" => 61,
        "--tw-translate-x" | "--tw-translate-y" => 62,
        "--tw-rotate" => 63,
        "--tw-skew-x" | "--tw-skew-y" => 64,
        "--tw-scale-x" | "--tw-scale-y" => 65,
        "transform" => 66,
        "animation" => 67,
        "cursor" => 68,
        "touch-action" => 69,
        "user-select" => 70,
        "resize" => 71,
        "scroll-snap-type" => 72,
        "scroll-snap-align" => 73,
        "scroll-snap-stop" => 74,
        _ if p.starts_with("scroll-margin") => 75,
        _ if p.starts_with("scroll-padding") => 76,
        "list-style-position" => 77,
        "list-style-type" => 78,
        "list-style-image" => 79,
        "appearance" => 80,
        "columns" => 81,
        "break-before" => 82,
        "break-inside" => 83,
        "break-after" => 84,
        "grid-auto-columns" => 85,
        "grid-auto-flow" => 86,
        "grid-auto-rows" => 87,
        "grid-template-columns" => 88,
        "grid-template-rows" => 89,
        "flex-direction" => 90,
        "flex-wrap" => 91,
        "place-content" => 92,
        "place-items" => 93,
        "align-content" => 94,
        "align-items" => 95,
        "justify-content" => 96,
        "justify-items" => 97,
        "gap" | "column-gap" | "row-gap" => 98,
        "place-self" => 190,
        "align-self" => 191,
        "justify-self" => 192,
        "overflow" | "overflow-x" | "overflow-y" => 193,
        _ if p.starts_with("overscroll-behavior") => 194,
        "scroll-behavior" => 195,
        "text-overflow" => 196,
        "hyphens" => 197,
        "white-space" => 198,
        "text-wrap" => 199,
        "word-break" | "overflow-wrap" => 200,
        _ if p.starts_with("border-") && p.ends_with("-radius") => 210,
        "border-radius" => 210,
        "border-width" => 211,
        _ if p.starts_with("border-") && p.ends_with("-width") => 211,
        "border-style" => 212,
        _ if p.starts_with("border-") && p.ends_with("-style") => 212,
        "border-color" => 213,
        _ if p.starts_with("border-") && p.ends_with("-color") => 213,
        "background-color" => 220,
        "background-image" => 221,
        _ if p.starts_with("--tw-gradient-") => 222,
        "box-decoration-break" => 223,
        "background-size" => 224,
        "background-attachment" => 225,
        "background-clip" => 226,
        "background-position" => 227,
        "background-repeat" => 228,
        "background-origin" => 229,
        "fill" => 230,
        "stroke" => 231,
        "stroke-width" => 232,
        "object-fit" => 233,
        "object-position" => 234,
        _ if p == "padding" || p.starts_with("padding-") => 240,
        "text-align" => 250,
        "text-indent" => 251,
        "vertical-align" => 252,
        "font-family" => 253,
        "font-size" => 254,
        "font-weight" => 255,
        "text-transform" => 256,
        "font-style" => 257,
        "font-variant-numeric" => 258,
        "line-height" => 259,
        "letter-spacing" => 260,
        "color" => 261,
        "text-decoration" | "text-decoration-line" => 262,
        "text-decoration-color" => 263,
        "text-decoration-style" => 264,
        "text-decoration-thickness" => 265,
        "text-underline-offset" => 266,
        "font-smoothing" | "osx-font-smoothing" => 267,
        "caret-color" => 318,
        "accent-color" => 319,
        "opacity" => 320,
        "background-blend-mode" => 321,
        "mix-blend-mode" => 322,
        "--tw-shadow" | "--tw-shadow-colored" | "box-shadow" => 330,
        "--tw-shadow-color" => 331,
        "outline" | "outline-style" => 340,
        "outline-width" => 341,
        "outline-offset" => 342,
        "outline-color" => 343,
        "--tw-ring-offset-shadow" | "--tw-ring-shadow" | "--tw-ring-inset" => 350,
        "--tw-ring-color" => 351,
        "--tw-ring-offset-width" => 352,
        "--tw-ring-offset-color" => 353,
        _ if p.starts_with("--tw-backdrop-") => 370,
        "backdrop-filter" => 371,
        "--tw-blur" | "--tw-brightness" | "--tw-contrast" | "--tw-drop-shadow" | "--tw-grayscale"
        | "--tw-hue-rotate" | "--tw-invert" | "--tw-saturate" | "--tw-sepia" => 360,
        "filter" => 361,
        "transition-property" => 380,
        "transition-delay" => 381,
        "transition-duration" => 382,
        "transition-timing-function" => 383,
        "will-change" => 390,
        "content" | "--tw-content" => 391,
        _ => return None,
    };
    Some(rank)
}

/// Properties that set every side (or both axes) at once.
const SHORTHANDS: &[&str] = &[
    "padding",
    "margin",
    "inset",
    "border-width",
    "border-style",
    "border-color",
    "border-radius",
    "scroll-margin",
    "scroll-padding",
    "gap",
    "overflow",
    "overscroll-behavior",
];

const PSEUDO_ELEMENTS: &[&str] = &[
    "first-letter",
    "first-line",
    "marker",
    "selection",
    "file",
    "placeholder",
    "backdrop",
    "before",
    "after",
];

/// Tailwind 3.4's pseudo-class variants in registration order: a later one
/// wins, so `focus:` beats `hover:` and `disabled:` beats both.
const PSEUDO_CLASSES: &[&str] = &[
    "first",
    "not-first",
    "last",
    "not-last",
    "only",
    "odd",
    "even",
    "first-of-type",
    "last-of-type",
    "only-of-type",
    "visited",
    "target",
    "open",
    "default",
    "checked",
    "indeterminate",
    "placeholder-shown",
    "autofill",
    "optional",
    "required",
    "valid",
    "invalid",
    "in-range",
    "out-of-range",
    "read-only",
    "empty",
    "focus-within",
    "hover",
    "focus",
    "focus-visible",
    "active",
    "enabled",
    "disabled",
];

fn properties(declarations: &str) -> impl Iterator<Item = &str> {
    declarations
        .split(';')
        .filter_map(|decl| decl.split_once(':').map(|(prop, _)| prop.trim()))
        .filter(|prop| !prop.is_empty())
}

fn first_property(declarations: &str) -> Option<&str> {
    properties(declarations).next()
}
