//! The theme a user project starts with — shadcn/ui's token names, verbatim,
//! Zebflow's values. Written into a new project's `globals.css`; the same
//! bytes `theme init` adds to a project that predates them. A theme exported
//! from ui.shadcn.com or tweakcn replaces the two blocks; see
//! `docs/contracts/kinds/ui-theme` for the profile a replacement must meet
//! (complete colour values, both blocks, the three status pairs).

/// `:root` (light) and `.dark`, every token the compiler accepts.
pub const THEME_CSS: &str = r#":root {
  --background: #e9edf3;
  --foreground: #1b1f24;
  --card: #ffffff;
  --card-foreground: #1b1f24;
  --popover: #ffffff;
  --popover-foreground: #1b1f24;
  --primary: #ea5a0c;
  --primary-foreground: #ffffff;
  --secondary: #f4f6f9;
  --secondary-foreground: #1b1f24;
  --muted: #f4f6f9;
  --muted-foreground: #525d68;
  --accent: #e9edf3;
  --accent-foreground: #1b1f24;
  --destructive: #dc2626;
  --destructive-foreground: #ffffff;
  --success: #15803d;
  --success-foreground: #ffffff;
  --warning: #b45309;
  --warning-foreground: #ffffff;
  --info: #1e66d6;
  --info-foreground: #ffffff;
  --border: #dce3ec;
  --input: #cdd6e1;
  --ring: #1e66d6;
  --chart-1: #ea5a0c;
  --chart-2: #1e66d6;
  --chart-3: #2f9e6a;
  --chart-4: #d98a1f;
  --chart-5: #b5443a;
  --sidebar: #ffffff;
  --sidebar-foreground: #1b1f24;
  --sidebar-primary: #ea5a0c;
  --sidebar-primary-foreground: #ffffff;
  --sidebar-accent: #f4f6f9;
  --sidebar-accent-foreground: #1b1f24;
  --sidebar-border: #dce3ec;
  --sidebar-ring: #1e66d6;
  --radius: 0.5rem;
}

.dark {
  --background: #14171b;
  --foreground: #e9edf3;
  --card: #16191e;
  --card-foreground: #e9edf3;
  --popover: #0f1115;
  --popover-foreground: #e9edf3;
  --primary: #ea5a0c;
  --primary-foreground: #ffffff;
  --secondary: #1b2026;
  --secondary-foreground: #e9edf3;
  --muted: #1b2026;
  --muted-foreground: #8e9cad;
  --accent: #222831;
  --accent-foreground: #e9edf3;
  --destructive: #ef4444;
  --destructive-foreground: #ffffff;
  --success: #7cd3a0;
  --success-foreground: #0f1a14;
  --warning: #f2a33d;
  --warning-foreground: #1b1408;
  --info: #1e66d6;
  --info-foreground: #ffffff;
  --border: #232932;
  --input: #2a313c;
  --ring: #1e66d6;
  --chart-1: #ea5a0c;
  --chart-2: #1e66d6;
  --chart-3: #7cd3a0;
  --chart-4: #f2a33d;
  --chart-5: #c46255;
  --sidebar: #16191e;
  --sidebar-foreground: #e9edf3;
  --sidebar-primary: #ea5a0c;
  --sidebar-primary-foreground: #ffffff;
  --sidebar-accent: #1b2026;
  --sidebar-accent-foreground: #e9edf3;
  --sidebar-border: #232932;
  --sidebar-ring: #1e66d6;
  color-scheme: dark;
}
"#;

/// The theme's token names, for a check that a project stylesheet defines
/// each of them (in `:root`) before a page that uses them renders unstyled.
pub fn token_names() -> Vec<&'static str> {
    THEME_CSS
        .split(".dark")
        .next()
        .unwrap_or("")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("--"))
        .filter_map(|rest| rest.split(':').next())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user theme and the studio theme are the same vocabulary. A token
    /// added to one and not the other is a component that renders in the
    /// studio and not in a project.
    #[test]
    fn user_theme_names_every_token_the_studio_theme_names() {
        let studio = include_str!("../web/templates/styles/main.css");
        let light = &studio[studio.find(":root {\n  --background").unwrap()..studio.find(".dark {").unwrap()];
        let studio_names: Vec<&str> = light
            .lines()
            .filter_map(|l| l.trim().strip_prefix("--"))
            .filter_map(|r| r.split(':').next())
            .collect();
        let ours = token_names();
        for name in &studio_names {
            assert!(ours.contains(name), "user theme lacks --{name}");
        }
        for name in &ours {
            assert!(studio_names.contains(name), "studio theme lacks --{name}");
        }
        // `--radius` is theme-wide, set once on :root like shadcn does; every
        // colour has a dark value.
        let dark = THEME_CSS.split(".dark").nth(1).unwrap();
        for name in ours.iter().filter(|n| **n != "radius") {
            assert!(dark.contains(&format!("--{name}:")), "dark block lacks --{name}");
        }
    }
}
