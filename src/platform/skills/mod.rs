//! Platform-bundled operational knowledge for agents.
//!
//! Skills are compiled into the binary via `include_str!()` — zero filesystem dependency at runtime.

/// One platform skill document.
pub struct Skill {
    /// Stable identifier used by `skill_read` MCP tool.
    pub name: &'static str,
    /// Short human-readable title.
    pub title: &'static str,
    /// First ~150 chars used as summary in `skill_list`.
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

/// One project archetype example (returned by `help_examples`).
pub struct Example {
    /// Stable slug used by `help_examples` tool.
    pub slug: &'static str,
    /// Short human-readable title.
    pub title: &'static str,
    /// One-line description shown in the listing.
    pub description: &'static str,
    /// Full markdown content returned when slug is specified.
    pub content: &'static str,
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
    Skill {
        name: "help-pipeline",
        title: "Pipeline System Guide",
        content: include_str!("help-pipeline.md"),
    },
];

static EXAMPLES: &[Example] = &[
    Example {
        slug: "webhook-restapi-postgres",
        title: "REST API + PostgreSQL",
        description: "JSON REST API (list, detail, create, update, delete) backed by PostgreSQL. Shows --params-path and --params-expr for safe parameterized queries with path params, query strings, and POST body.",
        content: include_str!("examples/webhook-restapi-postgres.md"),
    },
    Example {
        slug: "webhook-page-tsx",
        title: "Webhook → TSX Page",
        description: "Server-rendered HTML page from a GET webhook. pg.query result flows as `input` into the TSX template. Covers static, list, detail (path param), and query-string-filtered pages.",
        content: include_str!("examples/webhook-page-tsx.md"),
    },
    Example {
        slug: "cookie-jwt-auth",
        title: "Cookie + JWT Authentication",
        description: "Login issues a JWT in an HttpOnly cookie. Protected routes use --auth-type jwt to auto-verify; claims land in input.auth. Covers logout, role checks, and protected API endpoints.",
        content: include_str!("examples/cookie-jwt-auth.md"),
    },
    Example {
        slug: "agentic-scheduling",
        title: "Agentic Scheduling (AI + Cron)",
        description: "Scheduled pipelines that invoke zebtune AI agent to analyze data, generate reports, and classify queues.",
        content: include_str!("examples/agentic-scheduling.md"),
    },
    Example {
        slug: "blog-with-admin",
        title: "Blog with Admin",
        description: "Public blog with paginated listing, post detail, and JWT-protected admin CRUD. Uses Sekejap for posts.",
        content: include_str!("examples/blog-with-admin.md"),
    },
    Example {
        slug: "forum-with-chat",
        title: "Forum with Real-Time Chat",
        description: "Forum with threaded rooms and live WebSocket chat per room. Messages persisted in Sekejap.",
        content: include_str!("examples/forum-with-chat.md"),
    },
    Example {
        slug: "realtime-game",
        title: "Real-Time Game (WebSocket State Sync)",
        description: "Multiplayer game with server-side room state synced to all clients via ws.sync_state and ws.emit.",
        content: include_str!("examples/realtime-game.md"),
    },
    Example {
        slug: "scraping",
        title: "Web Scraping + Data Pipeline",
        description: "Cron-based scrapers that fetch external APIs or HTML pages, parse with script nodes, and upsert to Sekejap.",
        content: include_str!("examples/scraping.md"),
    },
    Example {
        slug: "auth-and-authorization",
        title: "Auth and Authorization",
        description: "Full JWT auth: login, register, session cookies, role-based access control, protected routes.",
        content: include_str!("examples/auth-and-authorization.md"),
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

/// Returns all available project archetypes.
pub fn all_examples() -> &'static [Example] {
    EXAMPLES
}

/// Find an example archetype by slug.
pub fn get_example(slug: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.slug == slug)
}

/// Format all skill summaries into a system prompt section.
pub fn format_skills_for_system_prompt(skills: &[Skill]) -> String {
    skills
        .iter()
        .map(|s| format!("### {}\n{}\n", s.title, s.content))
        .collect::<Vec<_>>()
        .join("\n---\n\n")
}
