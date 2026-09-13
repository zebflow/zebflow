//! Skills — the discipline layer an agent loads before a kind of task.
//!
//! The help tree says how the platform is shaped. A skill says *when* to do
//! what, in what order, and what proves it worked: a `SKILL.md` in the Agent
//! Skills format (frontmatter `name`, `description`, `license`; body under
//! 500 lines; `references/` and `scripts/` beside it, read on demand).
//!
//! Two sources, one list:
//!
//! - **Blessed** skills ship inside the binary (`blessed/skills/<name>/`),
//!   the way `zeb/ui` does. They are the MCP's own — every project lists
//!   them, none can opt out, and they are never hub items.
//! - **Project** skills live in the repository at `skills/<name>/SKILL.md`,
//!   written by the project or added from the hub. The optional skills
//!   (`blessed/skill-extras/<name>/`) arrive this way and only this way: a
//!   project sees one after it added it. A project skill with a blessed
//!   skill's name shadows it — that is how "clone to own" works for skills.
//!
//! Progressive disclosure is the contract: `list` is name + description only
//! (tier 1, what an agent sees on every session); `read` is the body (tier
//! 2); `read` with a path is a reference file (tier 3).

use std::collections::BTreeMap;
use std::path::Path;

use crate::platform::web::embedded::PLATFORM_SKILL_ASSETS;

/// Where the project keeps its own skills, relative to the source root.
pub const PROJECT_SKILLS_DIR: &str = "skills";
/// The file every skill folder must have.
pub const SKILL_FILE: &str = "SKILL.md";
/// Bodies longer than this stop being a skill and start being a manual; the
/// listing budget every client gives skills assumes they stay short.
pub const MAX_BODY_LINES: usize = 500;
/// The spec's ceiling for a description; clients truncate above it.
pub const MAX_DESCRIPTION_CHARS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Blessed,
    Project,
}

impl SkillSource {
    pub fn label(&self) -> &'static str {
        match self {
            SkillSource::Blessed => "blessed",
            SkillSource::Project => "project",
        }
    }
}

/// One skill as the listing shows it.
#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub source: SkillSource,
}

/// Frontmatter fields of a `SKILL.md`, plus the body after it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub extra: BTreeMap<String, String>,
    pub body: String,
}

/// Split a `SKILL.md` into its frontmatter and body. The frontmatter is the
/// flat `key: value` YAML the format uses; a value may be quoted. Nested
/// keys (`metadata:`) are kept as `metadata.<key>` in `extra`.
pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut out = Frontmatter::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = text.strip_prefix("---") else {
        out.body = text.to_string();
        return out;
    };
    let rest = rest.trim_start_matches(['\r', '\n']);
    let Some(end) = rest.find("\n---") else {
        out.body = text.to_string();
        return out;
    };
    let (front, after) = rest.split_at(end);
    out.body = after
        .trim_start_matches('\n')
        .trim_start_matches("---")
        .trim_start_matches(['\r', '\n'])
        .to_string();

    let mut parent: Option<String> = None;
    for line in front.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.trim();
        let value = unquote(value.trim());
        if indented {
            if let Some(p) = &parent {
                out.extra.insert(format!("{p}.{key}"), value);
            }
            continue;
        }
        if value.is_empty() {
            parent = Some(key.to_string());
            continue;
        }
        parent = None;
        match key {
            "name" => out.name = Some(value),
            "description" => out.description = Some(value),
            "license" => out.license = Some(value),
            other => {
                out.extra.insert(other.to_string(), value);
            }
        }
    }
    out
}

fn unquote(value: &str) -> String {
    let v = value.trim();
    if v.len() >= 2 {
        let first = v.as_bytes()[0];
        let last = v.as_bytes()[v.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return v[1..v.len() - 1].to_string();
        }
    }
    v.to_string()
}

/// The blessed skills, from the embedded assets. Each folder with a
/// `SKILL.md` is one skill; the folder name is authoritative for the name.
pub fn blessed_skills() -> Vec<SkillMeta> {
    let mut out = Vec::new();
    for asset in PLATFORM_SKILL_ASSETS {
        let Some(folder) = asset.path.strip_suffix(&format!("/{SKILL_FILE}")) else { continue };
        if folder.contains('/') {
            continue;
        }
        let text = String::from_utf8_lossy(asset.bytes);
        let fm = parse_frontmatter(&text);
        out.push(SkillMeta {
            name: folder.to_string(),
            description: fm.description.unwrap_or_default(),
            license: fm.license,
            source: SkillSource::Blessed,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// A blessed skill's file, by skill name and path inside its folder
/// (`SKILL.md` when `path` is empty).
pub fn blessed_file(name: &str, path: &str) -> Option<String> {
    let rel = skill_file_path(name, path)?;
    PLATFORM_SKILL_ASSETS
        .iter()
        .find(|a| a.path == rel)
        .map(|a| String::from_utf8_lossy(a.bytes).into_owned())
}

/// The project's own skills under `<source root>/skills/*/SKILL.md`.
pub fn project_skills(source_root: &Path) -> Vec<SkillMeta> {
    let mut out = Vec::new();
    let dir = source_root.join(PROJECT_SKILLS_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if !valid_name(name) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path.join(SKILL_FILE)) else { continue };
        let fm = parse_frontmatter(&text);
        out.push(SkillMeta {
            name: name.to_string(),
            description: fm.description.unwrap_or_default(),
            license: fm.license,
            source: SkillSource::Project,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// A project skill's file, refusing anything that would leave the folder.
pub fn project_file(source_root: &Path, name: &str, path: &str) -> Option<String> {
    let rel = skill_file_path(name, path)?;
    std::fs::read_to_string(source_root.join(PROJECT_SKILLS_DIR).join(rel)).ok()
}

/// Every skill an agent in this project sees: project skills first, then
/// the blessed ones a project skill has not shadowed.
pub fn list(source_root: &Path) -> Vec<SkillMeta> {
    let mut out = project_skills(source_root);
    for blessed in blessed_skills() {
        if !out.iter().any(|s| s.name == blessed.name) {
            out.push(blessed);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Read a skill's `SKILL.md`, or a file beside it. The project's copy wins.
pub fn read(source_root: &Path, name: &str, path: &str) -> Option<(SkillSource, String)> {
    if let Some(text) = project_file(source_root, name, path) {
        return Some((SkillSource::Project, text));
    }
    blessed_file(name, path).map(|text| (SkillSource::Blessed, text))
}

/// `a-z0-9-`, 1–64 chars, as the format requires; the folder name and the
/// frontmatter `name` must both satisfy it.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

fn skill_file_path(name: &str, path: &str) -> Option<String> {
    if !valid_name(name) {
        return None;
    }
    let path = path.trim();
    let path = if path.is_empty() { SKILL_FILE } else { path };
    // Relative, inside the folder, no tricks: an absolute path, a `..`, an
    // empty segment or a backslash is refused rather than normalised.
    if path.starts_with('/')
        || path.contains('\\')
        || path.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return None;
    }
    Some(format!("{name}/{path}"))
}

/// The tier-1 listing as text: one line per skill, the way `start_here`
/// shows it and `skill_list` returns it.
pub fn render_listing(skills: &[SkillMeta]) -> String {
    let mut out = String::new();
    for s in skills {
        let tag = match s.source {
            SkillSource::Blessed => "",
            SkillSource::Project => " (project)",
        };
        out.push_str(&format!("- `{}`{tag} — {}\n", s.name, s.description));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_parses_flat_keys_quotes_and_nested_metadata() {
        let fm = parse_frontmatter(
            "---\nname: zebflow-basic\ndescription: \"When: every session. What: orient.\"\nlicense: MIT\nmetadata:\n  version: \"1\"\n  derived_from: none\n---\n# Body\n\ntext\n",
        );
        assert_eq!(fm.name.as_deref(), Some("zebflow-basic"));
        assert_eq!(fm.description.as_deref(), Some("When: every session. What: orient."));
        assert_eq!(fm.license.as_deref(), Some("MIT"));
        assert_eq!(fm.extra.get("metadata.version").map(String::as_str), Some("1"));
        assert!(fm.body.starts_with("# Body"));
    }

    #[test]
    fn a_file_without_frontmatter_is_all_body() {
        let fm = parse_frontmatter("# Just a doc\n");
        assert!(fm.name.is_none());
        assert_eq!(fm.body, "# Just a doc\n");
    }

    #[test]
    fn names_follow_the_format() {
        assert!(valid_name("zebflow-basic"));
        assert!(!valid_name("Zebflow"));
        assert!(!valid_name("-x"));
        assert!(!valid_name("a b"));
        assert!(!valid_name(""));
    }

    #[test]
    fn a_path_cannot_leave_the_skill_folder() {
        assert_eq!(skill_file_path("s", "").as_deref(), Some("s/SKILL.md"));
        assert_eq!(skill_file_path("s", "references/a.md").as_deref(), Some("s/references/a.md"));
        assert!(skill_file_path("s", "../other/SKILL.md").is_none());
        assert!(skill_file_path("s", "/etc/passwd").is_none());
        assert!(skill_file_path("../s", "").is_none());
    }

    #[test]
    fn a_project_skill_shadows_a_blessed_one_by_name() {
        let tmp = tempfile::tempdir().expect("tmp");
        let Some(first) = blessed_skills().first().cloned() else { return };
        let dir = tmp.path().join(PROJECT_SKILLS_DIR).join(&first.name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SKILL_FILE), "---\nname: x\ndescription: mine\n---\nours\n").unwrap();
        let listed = list(tmp.path());
        let hit = listed.iter().find(|s| s.name == first.name).expect("listed");
        assert_eq!(hit.source, SkillSource::Project);
        assert_eq!(hit.description, "mine");
        assert_eq!(listed.iter().filter(|s| s.name == first.name).count(), 1);
        let (source, text) = read(tmp.path(), &first.name, "").expect("read");
        assert_eq!(source, SkillSource::Project);
        assert!(text.contains("ours"));
    }

    /// Every blessed skill is well-formed: folder name = frontmatter name,
    /// a description within the spec's limit, an explicit license, and a
    /// body short enough to be a skill rather than a manual.
    #[test]
    fn every_blessed_skill_is_well_formed() {
        let skills = blessed_skills();
        for s in &skills {
            let text = blessed_file(&s.name, "").expect("SKILL.md");
            let fm = parse_frontmatter(&text);
            assert!(valid_name(&s.name), "{}: bad folder name", s.name);
            assert_eq!(fm.name.as_deref(), Some(s.name.as_str()), "{}: frontmatter name must equal the folder", s.name);
            let description = fm.description.unwrap_or_default();
            assert!(!description.is_empty() && description.len() <= MAX_DESCRIPTION_CHARS, "{}: description missing or over {} chars", s.name, MAX_DESCRIPTION_CHARS);
            assert!(fm.license.is_some(), "{}: license is required", s.name);
            let lines = fm.body.lines().count();
            assert!(lines <= MAX_BODY_LINES, "{}: body is {lines} lines; over {}", s.name, MAX_BODY_LINES);
        }
    }
}
