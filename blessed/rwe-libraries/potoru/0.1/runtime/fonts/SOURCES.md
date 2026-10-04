# Built-in fonts (font aliases)

The Designer and the Player ship these three fonts. Stories and libraries name them by alias
(`font:basic-font`, `font:basic-serif`, `font:basic-mono`), never by file, so an app can replace an
alias with its own font from an installed library (a font asset published with that `alias`). App
fonts are not stored in this repository. The alias table is `app/src/runtime/font-aliases.ts`.

All three are licensed under the SIL Open Font License 1.1. Each license file (with its copyright
notice) sits next to its font and must ship with it. None declares a Reserved Font Name, so the
subsets below keep their internal names. The OFL allows bundling with software; it does not allow
selling the fonts on their own.

| Alias | File | Font (internal name) | Upstream (`github.com/google/fonts` at `23e54b51ddffbc7713c583748e3bd86f62b1fa4a`) | Upstream SHA-256 | License file | Shipped SHA-256 | Bytes |
|---|---|---|---|---|---|---|---:|
| `basic-font` (sans) | `basic-font.woff2` | Space Grotesk, variable weight 300–700 | `ofl/spacegrotesk/SpaceGrotesk[wght].ttf` | `acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72` | `basic-font-OFL.txt` | `28140e467e21683169c0ec1370f56b17b1d552b4f3662e293fe5266c4a503769` | 26,640 |
| `basic-serif` | `basic-serif.woff2` | Crimson Pro, variable weight 200–900 | `ofl/crimsonpro/CrimsonPro[wght].ttf` | `16aa9fb7300a93637da51fac03a071b2ff08b6bbf65f99c794c25f040b58af6a` | `basic-serif-OFL.txt` | `e3c1de0e57a7ddb3e7e959253f31cc6c37e465d0f3b4a3bf4181c774a2cc743c` | 52,628 |
| `basic-mono` | `basic-mono.woff2` | Space Mono Regular | `ofl/spacemono/SpaceMono-Regular.ttf` | `95837e182baeeada83368f7748db28357f0a1b75c6b84ff7065b5edf933c8e18` | `basic-mono-OFL.txt` | `386dac74fc438e774fd227694ddb5b156c211da2d771f6bc1b14fc99572e11b9` | 19,476 |

## Why a Latin subset

The aliases must stay general-purpose: any story may type any text. So the subset is by script, not by
the glyphs of one story. Each file keeps the Google Fonts "latin" range (Basic Latin, Latin-1
Supplement, general punctuation, the euro and trademark signs, arrows used in UI copy), every
OpenType layout feature, every name record, the weight axis, and hinting, and is stored as WOFF2.
Together the three files are 98,744 bytes, against 482,356 bytes for the full upstream TTFs.
Characters outside the range (Cyrillic, Greek, Vietnamese, CJK, …) fall back to the CSS family
list stored with the text. An app that needs them overrides the alias with a fuller font in its
own library.

Regenerate (fontTools 4.53 with brotli; output is byte-identical across runs) from the upstream files:

```sh
U="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"
pyftsubset "SpaceGrotesk[wght].ttf" --unicodes="$U" --layout-features='*' --name-IDs='*' --name-languages='*' --flavor=woff2 --output-file=basic-font.woff2
pyftsubset "CrimsonPro[wght].ttf"   --unicodes="$U" --layout-features='*' --name-IDs='*' --name-languages='*' --flavor=woff2 --output-file=basic-serif.woff2
pyftsubset "SpaceMono-Regular.ttf"  --unicodes="$U" --layout-features='*' --name-IDs='*' --name-languages='*' --flavor=woff2 --output-file=basic-mono.woff2
```

If a SHA-256 here changes, update `BUILTIN_FONT_ALIASES` in `app/src/runtime/font-aliases.ts`: the
Designer and Player refuse a built-in file whose bytes do not match the table.
