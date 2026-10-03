//! The `typescript` family: `typescript.script.run`, TypeScript in the Deno sandbox. It
//! answers `script: <what the code returned>`, the rest of the payload kept.
//!
//! The engine is shared with `javascript.script.run` and lives in
//! [`crate::pipeline::nodes::shared::script`]; this family holds only its
//! language and codes.

use crate::pipeline::NodeDefinition;
use crate::pipeline::nodes::shared::script::{self, Language};

pub const NODE_KIND: &str = "typescript.script.run";

/// TypeScript, and the family's codes.
pub static LANGUAGE: Language = Language {
    kind: NODE_KIND,
    title: "TypeScript",
    name: "TypeScript",
    editor: "typescript",
    run_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN",
    config_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN_CONFIG",
    parse_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN_PARSE",
    rejected_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN_REJECTED",
    compile_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN_COMPILE",
    input_pin_code: "FW_NODE_TYPESCRIPT_SCRIPT_RUN_INPUT_PIN",
};

/// The family list: this family is one node, and it lives in this file.
pub fn definitions() -> Vec<NodeDefinition> {
    vec![script::definition(&LANGUAGE)]
}

#[cfg(test)]
mod tests {
    /// No flags of its own (the source is the body) and one answer key.
    #[test]
    fn the_kind_takes_its_body_and_answers_script() {
        let def = &super::definitions()[0];
        assert_eq!(def.kind, "typescript.script.run");
        assert!(def.dsl_flags.is_empty(), "{:?}", def.dsl_flags);
        assert!(def.output_schema["properties"].get("script").is_some());
    }

    #[test]
    fn an_empty_body_is_refused_under_the_family_code() {
        let err = crate::pipeline::nodes::shared::script::Node::build(
            "s",
            &serde_json::json!({ "source": "  " }),
            &super::LANGUAGE,
            std::sync::Arc::new(crate::language::NoopLanguageEngine),
        )
        .err()
        .expect("refused");
        assert_eq!(err.code, "FW_NODE_TYPESCRIPT_SCRIPT_RUN_CONFIG");
    }
}
