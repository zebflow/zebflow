//! Tailwind CSS 3.4 LTS compatibility contract.
//!
//! These tests intentionally assert semantic declarations, not merely that a
//! token produced some CSS. This prevents Zebflow semantic color tokens from
//! accidentally accepting a real Tailwind utility with the wrong meaning.

use std::collections::HashSet;

use super::compiler::{process_tailwind, token_css_rule};

fn assert_rule(token: &str, fragments: &[&str]) {
    let css = token_css_rule(token).unwrap_or_else(|| panic!("missing Tailwind token: {token}"));
    for fragment in fragments {
        assert!(
            css.contains(fragment),
            "Tailwind token {token:?} did not contain {fragment:?}: {css}"
        );
    }
}

fn assert_unsupported(token: &str) {
    assert!(
        token_css_rule(token).is_none(),
        "unsupported token {token:?} must not compile as an unrelated semantic color"
    );
}

#[test]
fn semantic_tokens_do_not_capture_tailwind_utility_names() {
    assert_unsupported("bg-not-a-declared-token");
    assert_unsupported("border-not-a-declared-token");
    assert_unsupported("outline-not-a-declared-token");
    assert_rule("bg-surface", &["background-color:var(--color-surface)"]);
    assert_rule("text-body-soft", &["color:var(--color-body-soft)"]);
    assert_rule("border-ui-border", &["border-color:var(--color-ui-border)"]);
}

#[test]
fn stable_layout_and_sizing_utilities_match_tailwind_meaning() {
    assert_rule("aspect-video", &["aspect-ratio:16 / 9"]);
    assert_rule("container", &["width:100%", "@media (min-width: 640px)"]);
    assert_rule("columns-3", &["columns:3"]);
    assert_rule("object-center", &["object-position:center"]);
    assert_rule("basis-1/2", &["flex-basis:50%"]);
    assert_rule("gap-x-6", &["column-gap:1.5rem"]);
    assert_rule("max-w-7xl", &["max-width:80rem"]);
    assert_rule("h-dvh", &["height:100dvh"]);
}

#[test]
fn backgrounds_borders_tables_and_svg_keep_their_real_properties() {
    assert_rule("bg-fixed", &["background-attachment:fixed"]);
    assert_rule("bg-clip-text", &["background-clip:text"]);
    assert_rule("bg-origin-padding", &["background-origin:padding-box"]);
    assert_rule("bg-top", &["background-position:top"]);
    assert_rule("border-collapse", &["border-collapse:collapse"]);
    assert_rule(
        "border-spacing-2",
        &[
            "--tw-border-spacing-x:0.5rem",
            "border-spacing:var(--tw-border-spacing-x) var(--tw-border-spacing-y)",
        ],
    );
    assert_rule("outline-offset-2", &["outline-offset:2px"]);
    assert_rule("fill-sky-500", &["fill:#0ea5e9"]);
    assert_rule("stroke-2", &["stroke-width:2"]);
}

#[test]
fn transforms_filters_rings_shadows_and_gradients_compose() {
    assert_rule(
        "translate-x-2",
        &["--tw-translate-x:0.5rem", "var(--tw-rotate,0)"],
    );
    assert_rule(
        "rotate-45",
        &["--tw-rotate:45deg", "var(--tw-translate-x,0)"],
    );
    assert_rule("scale-95", &["--tw-scale-x:.95", "var(--tw-skew-x,0)"]);
    assert_rule(
        "blur-md",
        &["--tw-blur:blur(12px)", "var(--tw-brightness,)"],
    );
    assert_rule("contrast-125", &["--tw-contrast:contrast(1.25)"]);
    assert_rule(
        "ring-2",
        &["--tw-ring-shadow:", "var(--tw-shadow,0 0 #0000)"],
    );
    assert_rule("ring-offset-2", &["--tw-ring-offset-width:2px"]);
    assert_rule("shadow-xl", &["--tw-shadow:", "box-shadow:"]);
    assert_rule(
        "bg-gradient-to-tr",
        &["linear-gradient(to top right,var(--tw-gradient-stops))"],
    );
    assert_rule("via-purple-500", &["--tw-gradient-stops:", "#a855f7"]);
}

#[test]
fn gradient_stops_follow_tailwind_cascade_order() {
    let html = process_tailwind(
        "<html><head></head><body><div class=\"to-pink-500 via-purple-500 from-sky-500 bg-gradient-to-r\"></div></body></html>",
        &HashSet::new(),
    );
    let from = html.find(".from-sky-500").expect("from rule");
    let via = html.find(".via-purple-500").expect("via rule");
    let to = html.find(".to-pink-500").expect("to rule");
    assert!(
        from < via && via < to,
        "gradient stop order was {from}, {via}, {to}"
    );
}

#[test]
fn common_tailwind_variants_transform_selectors_and_at_rules() {
    assert_rule(
        "group-hover:text-white",
        &[".group:hover ", "color:#ffffff"],
    );
    assert_rule("peer-checked:block", &[".peer:checked ~ ", "display:block"]);
    assert_rule("has-[:checked]:ring-2", &[":has(:checked)"]);
    assert_rule("aria-checked:bg-blue-500", &["[aria-checked=\"true\"]"]);
    assert_rule("data-[state=open]:block", &["[data-state=\"open\"]"]);
    assert_rule(
        "supports-[display:grid]:grid",
        &["@supports (display:grid)"],
    );
    assert_rule("rtl:text-right", &["[dir=\"rtl\"]"]);
    assert_rule("portrait:hidden", &["@media (orientation: portrait)"]);
    assert_rule("marker:text-sky-500", &["::marker"]);
    assert_rule("*:border", &["> *"]);
}

#[test]
fn representative_v3_lts_surface_remains_supported() {
    let tokens = [
        // Layout, flexbox, grid, spacing, and sizing.
        "aspect-square",
        "container",
        "columns-3",
        "break-after-page",
        "box-border",
        "inline-grid",
        "object-cover",
        "object-center",
        "overflow-clip",
        "overscroll-contain",
        "sticky",
        "inset-1/2",
        "visible",
        "z-50",
        "basis-1/2",
        "flex-row-reverse",
        "grow",
        "order-first",
        "grid-cols-12",
        "grid-cols-subgrid",
        "col-span-3",
        "grid-rows-4",
        "gap-x-6",
        "justify-between",
        "content-between",
        "place-content-center",
        "place-items-center",
        "p-4",
        "-mt-2",
        "space-x-4",
        "space-x-reverse",
        "w-1/3",
        "min-w-min",
        "max-w-7xl",
        "h-dvh",
        "max-h-screen",
        "size-10",
        // Typography, backgrounds, borders, and effects.
        "font-sans",
        "text-4xl",
        "antialiased",
        "italic",
        "font-semibold",
        "tabular-nums",
        "tracking-tight",
        "line-clamp-3",
        "list-image-none",
        "list-outside",
        "text-center",
        "text-slate-700",
        "underline",
        "decoration-2",
        "uppercase",
        "truncate",
        "text-balance",
        "indent-4",
        "align-middle",
        "whitespace-pre-wrap",
        "break-all",
        "hyphens-auto",
        "content-['x']",
        "bg-fixed",
        "bg-clip-text",
        "bg-slate-900/50",
        "bg-origin-padding",
        "bg-top",
        "bg-no-repeat",
        "bg-cover",
        "bg-gradient-to-tr",
        "from-sky-500",
        "via-purple-500",
        "to-pink-500",
        "rounded-t-xl",
        "border-2",
        "border-slate-300",
        "border-dotted",
        "divide-y",
        "divide-slate-200",
        "outline-2",
        "outline-offset-2",
        "ring-2",
        "ring-sky-500",
        "ring-offset-2",
        "shadow-xl",
        "shadow-slate-900/20",
        "opacity-75",
        "mix-blend-multiply",
        "bg-blend-overlay",
        "blur-md",
        "brightness-125",
        "contrast-125",
        "drop-shadow-md",
        "grayscale",
        "hue-rotate-60",
        "invert",
        "saturate-150",
        "sepia",
        "backdrop-blur-md",
        "backdrop-brightness-125",
        // Tables, transforms, interaction, SVG, accessibility, and variants.
        "border-collapse",
        "border-spacing-2",
        "table-fixed",
        "caption-bottom",
        "transition-colors",
        "duration-300",
        "ease-in-out",
        "delay-150",
        "animate-spin",
        "scale-95",
        "rotate-45",
        "translate-x-2",
        "skew-y-3",
        "origin-center",
        "accent-sky-500",
        "appearance-none",
        "cursor-pointer",
        "caret-red-500",
        "pointer-events-none",
        "resize-y",
        "scroll-smooth",
        "scroll-mt-8",
        "scroll-p-4",
        "snap-center",
        "snap-mandatory",
        "touch-pan-y",
        "select-none",
        "will-change-transform",
        "fill-sky-500",
        "stroke-current",
        "stroke-2",
        "sr-only",
        "forced-color-adjust-none",
        "hover:bg-sky-600",
        "focus-visible:ring-2",
        "disabled:opacity-50",
        "first:pt-0",
        "only:block",
        "checked:bg-blue-500",
        "indeterminate:bg-gray-300",
        "group-hover:text-white",
        "peer-checked:block",
        "has-[:checked]:ring-2",
        "aria-checked:bg-blue-500",
        "data-[state=open]:block",
        "dark:bg-slate-900",
        "md:grid-cols-3",
        "max-lg:hidden",
        "supports-[display:grid]:grid",
        "rtl:text-right",
        "portrait:hidden",
        "print:hidden",
        "before:content-['x']",
        "marker:text-sky-500",
        "*:border",
    ];

    let missing = tokens
        .iter()
        .filter(|token| token_css_rule(token).is_none())
        .copied()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing Tailwind 3.4 tokens: {missing:?}"
    );
}

#[test]
fn common_tailwind_v3_pareto_surface_is_supported() {
    let groups: &[(&str, &[&str])] = &[
        (
            "layout",
            &[
                "aspect-video",
                "container",
                "columns-2",
                "break-inside-avoid",
                "box-decoration-clone",
                "box-border",
                "block",
                "inline-flex",
                "flow-root",
                "float-left",
                "clear-both",
                "isolate",
                "object-cover",
                "object-top",
                "overflow-x-auto",
                "overscroll-y-contain",
                "static",
                "inset-x-0",
                "visible",
                "z-50",
            ],
        ),
        (
            "flex_grid",
            &[
                "basis-1/3",
                "flex-row",
                "flex-wrap",
                "flex-auto",
                "grow-0",
                "shrink",
                "order-last",
                "grid-cols-3",
                "grid-cols-[200px_minmax(0,1fr)]",
                "col-span-full",
                "col-start-2",
                "grid-rows-2",
                "grid-rows-[auto_1fr]",
                "row-span-2",
                "row-start-1",
                "grid-flow-col",
                "auto-cols-fr",
                "auto-rows-min",
                "gap-4",
                "gap-y-2",
                "justify-evenly",
                "justify-items-center",
                "justify-self-end",
                "content-around",
                "items-baseline",
                "self-stretch",
                "place-content-between",
                "place-items-center",
                "place-self-auto",
            ],
        ),
        (
            "spacing_sizing",
            &[
                "p-4",
                "px-6",
                "pt-2",
                "-m-1",
                "mx-auto",
                "-mb-3",
                "space-x-4",
                "space-y-reverse",
                "w-1/2",
                "w-screen",
                "min-w-0",
                "min-w-full",
                "max-w-xs",
                "max-w-screen-xl",
                "h-12",
                "h-screen",
                "h-dvh",
                "min-h-0",
                "min-h-screen",
                "max-h-full",
                "max-h-dvh",
                "size-8",
            ],
        ),
        (
            "typography",
            &[
                "font-serif",
                "text-6xl",
                "antialiased",
                "not-italic",
                "font-black",
                "ordinal",
                "tracking-tighter",
                "line-clamp-2",
                "leading-loose",
                "list-image-none",
                "list-outside",
                "list-square",
                "text-start",
                "text-gray-600",
                "underline",
                "decoration-wavy",
                "decoration-red-500",
                "underline-offset-4",
                "normal-case",
                "truncate",
                "text-pretty",
                "indent-8",
                "align-super",
                "whitespace-break-spaces",
                "break-words",
                "hyphens-auto",
                "content-none",
                "placeholder-gray-400",
            ],
        ),
        (
            "backgrounds",
            &[
                "bg-fixed",
                "bg-clip-text",
                "bg-blue-500/50",
                "bg-origin-content",
                "bg-left-bottom",
                "bg-repeat-round",
                "bg-cover",
                "bg-none",
                "bg-[url('/image.png')]",
                "bg-gradient-to-bl",
                "from-cyan-500",
                "via-blue-500",
                "to-purple-500",
            ],
        ),
        (
            "borders",
            &[
                "rounded",
                "rounded-tr-xl",
                "border",
                "border-x-2",
                "border-b-4",
                "border-gray-300",
                "border-t-red-500",
                "border-x-transparent",
                "border-double",
                "divide-x-2",
                "divide-y",
                "divide-gray-200",
                "divide-dashed",
                "outline-2",
                "outline-red-500",
                "outline-dashed",
                "outline-offset-4",
                "ring-4",
                "ring-green-500",
                "ring-offset-2",
                "ring-offset-white",
            ],
        ),
        (
            "effects_filters",
            &[
                "shadow",
                "shadow-lg",
                "shadow-black/20",
                "opacity-50",
                "mix-blend-screen",
                "bg-blend-multiply",
                "blur-sm",
                "brightness-110",
                "contrast-125",
                "drop-shadow-lg",
                "grayscale",
                "hue-rotate-30",
                "invert",
                "saturate-200",
                "sepia",
                "backdrop-blur-xl",
                "backdrop-opacity-50",
            ],
        ),
        (
            "tables_motion",
            &[
                "border-collapse",
                "border-spacing-x-2",
                "table-auto",
                "caption-top",
                "transition",
                "transition-shadow",
                "duration-200",
                "ease-out",
                "delay-75",
                "animate-pulse",
                "scale-x-110",
                "-rotate-6",
                "-translate-y-1/2",
                "skew-x-6",
                "origin-bottom-right",
            ],
        ),
        (
            "interaction_svg",
            &[
                "accent-pink-500",
                "appearance-none",
                "cursor-grab",
                "caret-blue-500",
                "pointer-events-auto",
                "resize",
                "scroll-smooth",
                "scroll-mx-4",
                "scroll-py-2",
                "snap-x",
                "snap-always",
                "touch-manipulation",
                "select-text",
                "will-change-scroll",
                "fill-current",
                "fill-red-500",
                "stroke-blue-500",
                "stroke-1",
                "sr-only",
                "not-sr-only",
                "forced-color-adjust-none",
            ],
        ),
        (
            "variants",
            &[
                "hover:bg-blue-600",
                "focus:ring-2",
                "active:scale-95",
                "visited:text-purple-600",
                "disabled:cursor-not-allowed",
                "first:mt-0",
                "last:border-0",
                "only:block",
                "odd:bg-gray-50",
                "checked:border-blue-500",
                "required:border-red-500",
                "invalid:text-red-600",
                "group-hover:underline",
                "group-focus-within:ring-2",
                "peer-invalid:block",
                "peer-checked:bg-blue-500",
                "has-[:checked]:border-blue-500",
                "aria-expanded:block",
                "aria-[sort=ascending]:rotate-180",
                "data-[state=open]:block",
                "sm:grid-cols-2",
                "max-md:hidden",
                "dark:bg-gray-900",
                "motion-reduce:transition-none",
                "contrast-more:border-black",
                "portrait:flex",
                "print:hidden",
                "rtl:text-right",
                "supports-[backdrop-filter:blur(0)]:backdrop-blur",
                "before:content-['x']",
                "placeholder:text-gray-400",
                "selection:bg-blue-200",
                "marker:text-gray-500",
                "file:border-0",
                "*:p-2",
            ],
        ),
    ];
    for (group, tokens) in groups {
        let missing = tokens
            .iter()
            .filter(|token| token_css_rule(token).is_none())
            .copied()
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "Tailwind v3 Pareto group {group} is missing {missing:?}"
        );
    }
}
