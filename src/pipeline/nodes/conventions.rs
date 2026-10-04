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

    /// The contract is the one source of the closed lists these checks hold
    /// nodes to: read at test time, so editing the contract is editing the
    /// check.
    const CONTRACT: &str = include_str!("../../../docs/contracts/node-conventions.md");

    /// Every backticked token on the first contract line starting with `prefix`.
    fn contract_words(prefix: &str) -> Vec<String> {
        let line = CONTRACT
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("the contract has a line starting {prefix:?}"));
        line.split('`').skip(1).step_by(2).map(str::to_string).collect()
    }

    /// The dictionary's words: every `--flag` in the table under "### The dictionary".
    fn dictionary_words() -> std::collections::BTreeSet<String> {
        let start = CONTRACT.find("### The dictionary").expect("dictionary section");
        let table = CONTRACT[start..].split("\n\n").find(|block| block.starts_with("| Area")).expect("dictionary table");
        table.split('`').skip(1).step_by(2).filter(|w| w.starts_with("--")).map(str::to_string).collect()
    }

    /// Native nodes and the official composites shipped in the binary: the
    /// two origins that share plain names (§1).
    fn official_definitions() -> Vec<crate::pipeline::model::NodeDefinition> {
        let mut defs = crate::pipeline::nodes::builtin_node_definitions();
        defs.extend(crate::platform::services::NodeRegistryService::embedded_official_definitions());
        defs
    }

    /// §1: `family.noun.verb`, `trigger.<source>`, `input.<type>`,
    /// `logic.<verb>`; the family from the closed list; a verb from the shared
    /// list or used by one kind only.
    #[test]
    fn kinds_follow_the_shapes() {
        let mut families: std::collections::BTreeSet<String> = contract_words("| Now |").into_iter().collect();
        families.extend(contract_words("| Brands |"));
        let mut verbs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for row in ["| Read |", "| Write |", "| Make |", "| Talk |", "| Act |"] {
            verbs.extend(contract_words(row));
        }
        let logic_line = CONTRACT.lines().position(|l| l.contains("`logic.*` is closed:")).expect("logic list");
        let logic: Vec<String> = CONTRACT.lines().skip(logic_line).take(2).collect::<Vec<_>>().join(" ")
            .split("is closed:").nth(1).unwrap_or_default().split('(').next().unwrap_or_default()
            .split('`').skip(1).step_by(2).map(str::to_string).collect();
        let defs = official_definitions();
        let mut verb_uses: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for def in &defs {
            if let Some(verb) = def.kind.split('.').nth(2) {
                *verb_uses.entry(verb.to_string()).or_default() += 1;
            }
        }
        let mut problems = Vec::new();
        for def in &defs {
            let kind = def.kind.as_str();
            let segments: Vec<&str> = kind.split('.').collect();
            if segments.iter().any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')) {
                problems.push(format!("{kind}: a segment is not one lowercase word"));
                continue;
            }
            match segments.as_slice() {
                ["trigger" | "input", _] => {}
                ["logic", verb] if logic.iter().any(|l| l == verb) => {}
                ["logic", verb] => problems.push(format!("{kind}: logic.{verb} is not in the closed logic list")),
                [family, _noun, verb] => {
                    if !families.contains(*family) {
                        problems.push(format!("{kind}: family '{family}' is not in the contract's closed list"));
                    }
                    if !verbs.contains(*verb) && verb_uses.get(*verb).copied().unwrap_or(0) > 1 {
                        problems.push(format!("{kind}: verb '{verb}' is shared by several kinds but missing from the contract's verb list"));
                    }
                }
                _ => problems.push(format!("{kind}: not family.noun.verb, trigger.<source>, input.<type> or logic.<verb>")),
            }
        }
        report("kinds_follow_the_shapes", problems);
    }

    /// §2: a flag two kinds share is a dictionary word, and is a switch, a
    /// map or a value the same way everywhere.
    #[test]
    fn shared_words_mean_one_thing() {
        use crate::pipeline::model::DslFlagKind;
        let dictionary = dictionary_words();
        let common: std::collections::BTreeSet<String> =
            crate::pipeline::model::engine_common_dsl_flags().into_iter().map(|f| f.flag).collect();
        let mut uses: std::collections::BTreeMap<String, Vec<(String, &'static str)>> = std::collections::BTreeMap::new();
        for def in official_definitions() {
            for flag in &def.dsl_flags {
                if common.contains(&flag.flag) {
                    continue;
                }
                let shape = match flag.kind {
                    DslFlagKind::Bool => "a switch",
                    DslFlagKind::KeyValuePairs => "a map",
                    _ => "a value",
                };
                uses.entry(flag.flag.clone()).or_default().push((def.kind.clone(), shape));
            }
        }
        // §1: a brand's own concepts are local words of that brand
        // (`--keyboard` on `telegram.*`), so a word shared only inside one
        // brand family is not a dictionary word.
        let brands: std::collections::BTreeSet<String> = contract_words("| Brands |").into_iter().collect();
        let mut problems = Vec::new();
        for (flag, kinds) in &uses {
            let distinct_kinds: std::collections::BTreeSet<&String> = kinds.iter().map(|(k, _)| k).collect();
            if distinct_kinds.len() < 2 {
                continue;
            }
            let families: std::collections::BTreeSet<&str> =
                distinct_kinds.iter().map(|k| k.split('.').next().unwrap_or_default()).collect();
            let one_brand = families.len() == 1 && families.iter().all(|f| brands.contains(*f));
            if !dictionary.contains(flag) && !one_brand {
                problems.push(format!("{flag} is shared by {} kinds but is not a dictionary word: {}", distinct_kinds.len(), distinct_kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")));
            }
            let shapes: std::collections::BTreeSet<&str> = kinds.iter().map(|(_, s)| *s).collect();
            if shapes.len() > 1 {
                problems.push(format!("{flag} is {}", kinds.iter().map(|(k, s)| format!("{s} on {k}")).collect::<Vec<_>>().join(", ")));
            }
        }
        report("shared_words_mean_one_thing", problems);
    }

    /// §1, §12: the official composites shipped in the binary pass the same
    /// shape and flag rules as native nodes (the checks above read both), and
    /// an installed bundle's kinds are `x.<package>.<noun>.<verb>`.
    #[test]
    fn composites_follow_the_grammar() {
        use crate::contracts::kinds::{BundleScope, validate_bundle_namespace};
        let composites = crate::platform::services::NodeRegistryService::embedded_official_definitions();
        assert!(!composites.is_empty(), "the binary ships official composites");
        // A real installable bundle (package `composite`), its first node's
        // kind changed per case.
        let fixture = crate::contracts::kinds::decode_node_bundle(
            include_str!("../../../tests/fixtures/contracts/node-bundle/v1-composite.json").as_bytes(),
        )
        .expect("the composite bundle fixture decodes")
        .spec;
        let bundle = |kind: &str| {
            let mut spec = fixture.clone();
            spec.nodes[0].kind = kind.to_string();
            spec
        };
        assert!(validate_bundle_namespace(&bundle("x.composite.invoice.create"), BundleScope::Project).is_ok());
        for wrong in ["x.composite.invoice", "x.composite.invoice.create.now", "x.other.invoice.create", "invoice.document.create"] {
            assert!(
                validate_bundle_namespace(&bundle(wrong), BundleScope::Project).is_err(),
                "an installed kind '{wrong}' must be refused"
            );
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
        let defs = crate::pipeline::nodes::builtin_node_definitions();
        let mut checked = 0usize;
        let mut problems = Vec::new();
        for (file, text) in node_sources() {
            if !file.starts_with("basic/") || file.ends_with("tests.rs") {
                continue;
            }
            let touches = ["open_store(", "open_source(", "object_local_path("].iter().any(|n| text.contains(n));
            if !touches {
                continue;
            }
            // A kind is named in its own file as a string literal.
            let kinds: Vec<&str> = defs
                .iter()
                .map(|def| def.kind.as_str())
                .filter(|kind| text.contains(&format!("\"{kind}\"")))
                .collect();
            assert!(
                !kinds.is_empty() || !text.contains("NODE_KIND"),
                "{file} touches a store and declares a kind, but none was recognised: the check would pass by seeing nothing"
            );
            for kind in kinds {
                checked += 1;
                if !pinned.contains(kind) && !STORE_FROM_RECORD.contains(&kind) {
                    problems.push(format!("{file}: {kind} touches a store but declares no --store"));
                }
            }
        }
        assert!(checked >= 10, "only {checked} store-touching kinds were found; the check has gone blind");
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
