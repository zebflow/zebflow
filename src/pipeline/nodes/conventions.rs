//! Enforcement of `docs/contracts/node-conventions.md` §9.
//!
//! Each test reads either every official node definition or the source of
//! every official node, and names each place that breaks a rule. A new node
//! passes these before it is registered; a rule broken on purpose is named in
//! an allow-list here, with its reason, rather than left for a reviewer.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    /// Every official node source file, with its `#[cfg(test)]` module cut
    /// off: tests may build whatever payload they like.
    fn node_sources() -> Vec<(String, String)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pipeline/nodes");
        let mut files = Vec::new();
        for dir in ["basic", "shared"] {
            walk(&root.join(dir), &mut files);
        }
        files.sort();
        files
            .into_iter()
            .map(|path| {
                let text = std::fs::read_to_string(&path).expect("node source");
                // Only the test module is cut: a `#[cfg(test)]` on one item
                // earlier in the file must not hide the rest from the rules.
                let text = match text.find("#[cfg(test)]\nmod tests") {
                    Some(at) => text[..at].to_string(),
                    None => text,
                };
                let rel = path.strip_prefix(&root).expect("under nodes").to_string_lossy().replace('\\', "/");
                (rel, text)
            })
            .collect()
    }

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("node dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// The lines of `text` containing `needle`, as `file:line: text`.
    fn hits(file: &str, text: &str, needle: &str) -> Vec<String> {
        text.lines()
            .enumerate()
            .filter(|(_, line)| line.contains(needle) && !line.trim_start().starts_with("//"))
            .map(|(n, line)| format!("{file}:{}: {}", n + 1, line.trim()))
            .collect()
    }

    fn report(rule: &str, problems: Vec<String>) {
        assert!(problems.is_empty(), "{rule} — {} place(s):\n{}", problems.len(), problems.join("\n"));
    }

    /// Every key under any `properties` object in a schema.
    fn property_names(schema: &Value, out: &mut Vec<String>) {
        match schema {
            Value::Object(map) => {
                if let Some(Value::Object(props)) = map.get("properties") {
                    out.extend(props.keys().cloned());
                }
                map.values().for_each(|v| property_names(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| property_names(v, out)),
            _ => {}
        }
    }

    /// §1: no `--no-*`, no `-path` but `--path`, no `url` answered, and a
    /// select field lists its words.
    #[test]
    fn flags_follow_the_grammar() {
        let mut problems = Vec::new();
        for def in crate::pipeline::nodes::builtin_node_definitions() {
            for flag in &def.dsl_flags {
                if flag.flag.starts_with("--no-") {
                    problems.push(format!("{}: {} is a negative switch", def.kind, flag.flag));
                }
                if flag.flag.ends_with("-path") && flag.flag != "--path" {
                    problems.push(format!("{}: {} — a dot-path is --*-key", def.kind, flag.flag));
                }
            }
            // §3: where a stored file or a map layer can be reached is the
            // owner's exposure decision, so no file, map or site node answers
            // one. (A feed address a trigger listens to is not that.)
            let file_node = crate::pipeline::nodes::shared::project_store::store_node_kinds().contains(&def.kind)
                || def.kind.starts_with("mapserver.")
                || def.kind.starts_with("web.");
            let mut names = Vec::new();
            property_names(&def.output_schema, &mut names);
            for name in names.iter().filter(|n| file_node && matches!(n.as_str(), "url" | "urls" | "public_url")) {
                problems.push(format!("{}: answers `{name}`", def.kind));
            }
            for field in &def.fields {
                if matches!(field.field_type, crate::pipeline::model::NodeFieldType::Select)
                    && field.options.is_empty()
                    && field.data_source.is_none()
                {
                    problems.push(format!("{}: select field '{}' lists no words", def.kind, field.name));
                }
            }
        }
        report("flags_follow_the_grammar", problems);
    }

    /// §2–§4: a flag's declared value type is one of the closed list, a
    /// choice is a single value, and a repeat ceiling sits on a repeatable flag.
    #[test]
    fn flag_metadata_is_well_formed() {
        use crate::pipeline::model::{DslFlagKind, FLAG_VALUE_TYPES};
        let mut problems = Vec::new();
        for def in crate::pipeline::nodes::builtin_node_definitions() {
            for flag in &def.dsl_flags {
                if !flag.value.is_empty() && !FLAG_VALUE_TYPES.contains(&flag.value.as_str()) {
                    problems.push(format!("{}: {} declares value '{}'", def.kind, flag.flag, flag.value));
                }
                if !flag.choices.is_empty() && !matches!(flag.kind, DslFlagKind::Scalar | DslFlagKind::RepeatedList) {
                    problems.push(format!("{}: {} lists choices on a {:?} flag", def.kind, flag.flag, flag.kind));
                }
                if flag.max_repeat.is_some() && !matches!(flag.kind, DslFlagKind::RepeatedList | DslFlagKind::CommaSeparatedList) {
                    problems.push(format!("{}: {} has a repeat ceiling but does not repeat", def.kind, flag.flag));
                }
            }
        }
        report("flag_metadata_is_well_formed", problems);
    }

    /// The signature is generated from the definition: choices, value types,
    /// repeats, optional brackets and the answer key all come from it.
    #[test]
    fn the_signature_is_generated_from_the_definition() {
        use crate::pipeline::model::{DslFlag, DslFlagKind, NodeDefinition};
        let def = NodeDefinition {
            kind: "fs.image.thumbnail".to_string(),
            dsl_flags: vec![
                DslFlag { flag: "--from".into(), kind: DslFlagKind::Scalar, required: true, value: "file:image".into(), ..Default::default() },
                DslFlag { flag: "--width".into(), value: "number".into(), ..Default::default() },
                DslFlag { flag: "--fit".into(), choices: vec!["cover".into(), "contain".into(), "fill".into()], ..Default::default() },
                DslFlag { flag: "--image".into(), kind: DslFlagKind::RepeatedList, value: "file:image".into(), max_repeat: Some(4), ..Default::default() },
                DslFlag { flag: "--header".into(), kind: DslFlagKind::KeyValuePairs, value: "text".into(), ..Default::default() },
                DslFlag { flag: "--delete-source".into(), kind: DslFlagKind::Bool, ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(
            crate::pipeline::nodes::node_signature(&def),
            "fs.image.thumbnail --from IMAGE [--width N] [--fit cover|contain|fill] [--image IMAGE…] [--header KEY=TEXT…] [--delete-source] → image"
        );
        assert_eq!(crate::pipeline::nodes::answer_key("trigger.webhook").as_deref(), Some("webhook"));
        assert_eq!(crate::pipeline::nodes::answer_key("logic.if"), None);
        assert_eq!(crate::pipeline::nodes::answer_key("web.response.send"), None);
        assert_eq!(crate::pipeline::nodes::answer_key("web.site.generate").as_deref(), Some("site"));
    }

    /// §3 one door: bytes are reached through `shared/project_store.rs` (and
    /// the scratch it lends), never through the default store or a joined
    /// local path.
    #[test]
    fn nodes_use_one_door() {
        const DOOR: &[&str] = &["shared/project_store.rs", "shared/store_scratch.rs"];
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            if DOOR.contains(&file.as_str()) {
                continue;
            }
            for needle in [".open_files()", "zebfs.get(", ".fs.get(", "store.get(", "std::fs::read(", " fs::read("] {
                problems.extend(hits(&file, &text, needle));
            }
            // The same calls split over lines (`zebfs\n    .get(`).
            let joined: String = text.split('\n').map(str::trim).collect::<Vec<_>>().join("");
            for needle in ["zebfs.get(", "store.fs.get(", "self.store.get("] {
                if joined.matches(needle).count() > text.matches(needle).count() {
                    problems.push(format!("{file}: `{needle}` split over lines"));
                }
            }
        }
        report("nodes_use_one_door", problems);
    }

    /// §5: a node's answer is added through `with_answer`; routers and the
    /// terminal `web.response.send` are the named exceptions.
    #[test]
    fn answers_go_through_with_answer() {
        const OUTSIDE_THE_RULE: &[&str] = &["basic/logic/", "basic/web/response/"];
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            if OUTSIDE_THE_RULE.iter().any(|prefix| file.starts_with(prefix)) {
                continue;
            }
            for needle in ["payload: json!(", "payload: serde_json::json!(", "payload: Value::Object(", "payload: serde_json::Value::Object("] {
                problems.extend(hits(&file, &text, needle));
            }
            for needle in [".remove(top)", "payload.remove(", "out.remove("] {
                problems.extend(hits(&file, &text, needle));
            }
        }
        report("answers_go_through_with_answer", problems);
    }

    /// §3: every node that reads, writes or deletes stored files declares
    /// `--store`, which is what pins it at registration.
    #[test]
    fn store_nodes_declare_their_store() {
        // These share a file with a store node but take their store from the
        // layer record the registry holds, never from a flag.
        const STORE_FROM_RECORD: &[&str] = &["mapserver.layer.get", "mapserver.layer.list", "mapserver.layer.unpublish"];
        let pinned = crate::pipeline::nodes::shared::project_store::store_node_kinds();
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            if !file.starts_with("basic/") {
                continue;
            }
            let touches = ["open_store(", "open_source(", "object_local_path("].iter().any(|n| text.contains(n));
            if !touches {
                continue;
            }
            let kinds: Vec<&str> = text
                .match_indices("\"n.")
                .filter_map(|(at, _)| text[at + 1..].split('"').next())
                .filter(|kind| kind.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c == '_'))
                .filter(|kind| crate::pipeline::nodes::builtin_node_definitions().iter().any(|d| d.kind == *kind))
                .collect();
            for kind in kinds {
                if !pinned.contains(kind) && !STORE_FROM_RECORD.contains(&kind) {
                    problems.push(format!("{file}: {kind} touches a store but declares no --store"));
                }
            }
        }
        problems.sort();
        problems.dedup();
        report("store_nodes_declare_their_store", problems);
    }

    /// §1: a config key has one name.
    #[test]
    fn config_keys_have_one_name() {
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            problems.extend(hits(&file, &text, "alias = \""));
            problems.extend(hits(&file, &text, "alias=\""));
        }
        report("config_keys_have_one_name", problems);
    }

    /// §6: a code raised in `basic/<family>/…` is `FW_NODE_<FAMILY>_…`, or
    /// one of the two pass-through families, and is registered.
    #[test]
    fn codes_carry_the_node_family() {
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            let family = match file.strip_prefix("basic/").and_then(|rest| rest.split('/').next()) {
                Some(dir) => dir.trim_end_matches(".rs").to_ascii_uppercase(),
                None => continue,
            };
            let prefix = format!("FW_NODE_{family}_");
            let exact = format!("FW_NODE_{family}");
            let mut at = 0;
            while let Some(pos) = text[at..].find("PipelineError::new(") {
                // The code may sit on the next line: skip whitespace, then a
                // string literal is a code; anything else (a variable) is not.
                let after = at + pos + "PipelineError::new(".len();
                let rest = &text[after..];
                let skipped = rest.len() - rest.trim_start().len();
                at = after;
                if !rest.trim_start().starts_with('"') {
                    continue;
                }
                let start = after + skipped + 1;
                let end = text[start..].find('"').map(|e| start + e).unwrap_or(start);
                let code = &text[start..end];
                let ok = code == exact
                    || code.starts_with(&prefix)
                    || code.starts_with("ZEBFS_")
                    || code.strip_prefix("FW_").is_some_and(|rest| rest.starts_with("FILE_REF_"));
                if !ok {
                    problems.push(format!("{file}: {code} is not under {prefix}"));
                }
                if crate::pipeline::error_class::registered_class(code).is_none() {
                    problems.push(format!("{file}: {code} is not registered"));
                }
                at = end.max(start);
            }
        }
        problems.sort();
        problems.dedup();
        report("codes_carry_the_node_family", problems);
    }
}
