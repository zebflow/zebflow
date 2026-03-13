//! Platform-bundled operational knowledge for agents.
//!
//! Skills are compiled into the binary via `include_str!()` — zero filesystem dependency at runtime.

/// One platform skill document.
pub struct Skill {
    /// Stable identifier used by `read_skill` MCP tool.
    pub name: &'static str,
    /// Short human-readable title.
    pub title: &'static str,
    /// First ~150 chars used as summary in `list_skills`.
    pub content: &'static str,
}

impl Skill {
    /// Returns a short summary (first 200 chars of content, trimmed to sentence).
    pub fn summary(&self) -> &str {
        let s = self.content.trim();
        let end = s
            .char_indices()
            .take_while(|(i, _)| *i < 200)
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(s.len());
        &s[..end]
    }
}

static SKILLS: &[Skill] = &[
    Skill {
        name: "agent-core",
        title: "Zebflow Agent Quick Start",
        content: include_str!("agent-core.md"),
    },
    Skill {
        name: "zebflow-overview",
        title: "Zebflow Platform Overview",
        content: include_str!("zebflow-overview.md"),
    },
    Skill {
        name: "pipeline-dsl",
        title: "Pipeline DSL Reference",
        content: include_str!("pipeline-dsl.md"),
    },
    Skill {
        name: "pipeline-authoring",
        title: "Pipeline Authoring Patterns",
        content: include_str!("pipeline-authoring.md"),
    },
    Skill {
        name: "pipeline-nodes",
        title: "Pipeline Node Catalog",
        content: include_str!("pipeline-nodes.md"),
    },
    Skill {
        name: "pipeline-dsl-rwe",
        title: "Pipeline DSL — RWE & web.render",
        content: include_str!("pipeline-dsl-rwe.md"),
    },
    Skill {
        name: "pipeline-dsl-web-auto",
        title: "Pipeline DSL — web.auto Language",
        content: include_str!("pipeline-dsl-web-auto.md"),
    },
    Skill {
        name: "sekejapql",
        title: "SekejapQL Query Language",
        content: include_str!("sekejapql.md"),
    },
    Skill {
        name: "rwe-templates",
        title: "RWE Template Authoring",
        content: include_str!("rwe-templates.md"),
    },
    Skill {
        name: "project-operations",
        title: "Project Operations Guide",
        content: include_str!("project-operations.md"),
    },
    Skill {
        name: "full-project-workflow",
        title: "Full Project Workflow — Concept to Live Website",
        content: include_str!("full-project-workflow.md"),
    },
    Skill {
        name: "api-reference",
        title: "REST API Reference",
        content: include_str!("api-reference.md"),
    },
];

/// Returns all available platform skills.
pub fn all_skills() -> &'static [Skill] {
    SKILLS
}

/// Find a skill by name.
pub fn get_skill(name: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.name == name)
}

/// Format all skill summaries into a system prompt section.
pub fn format_skills_for_system_prompt(skills: &[Skill]) -> String {
    skills
        .iter()
        .map(|s| format!("### {}\n{}\n", s.title, s.content))
        .collect::<Vec<_>>()
        .join("\n---\n\n")
}
