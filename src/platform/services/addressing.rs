//! Addressing — where a project answers on the network.
//!
//! `docs/contracts/addressing.md`. A project never contains its own address:
//! hosts and routes live in the project's `data/store/addressing.json`, which
//! is instance configuration — a bundle, a hub package or a `git clone`
//! carries none of it.
//!
//! Every project answers at its dev host `<project>.<owner>.localhost` with
//! no configuration; browsers resolve `*.localhost` to loopback themselves.
//! Custom hosts are added by the operator. On any project host the pages
//! surface is the root and the other surfaces sit under a reserved `_`
//! prefix (`/_files/…`), unless a route mounts a surface elsewhere
//! (`zebms.mydomain.com/service/`). The neutral form `/wh/{owner}/{project}`
//! keeps working on every host.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::slug_segment;

/// The file under `data/store/` — store tier: irreplaceable, never in the repo.
pub const ADDRESSING_FILE: &str = "addressing.json";
/// The suffix every dev host ends in.
pub const DEV_HOST_SUFFIX: &str = ".localhost";
/// The prefix reserved for platform surfaces on a project host; an app route
/// may not start with it.
pub const RESERVED_PREFIX: &str = "/_";

/// One thing a project serves. The platform form is host-agnostic and always
/// valid; the project-host path is where the surface answers by default on a
/// host of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Pages,
    Ws,
    Files,
    Static,
    Ms,
    Fs,
    Mcp,
}

impl Surface {
    pub const ALL: [Surface; 7] = [
        Surface::Pages,
        Surface::Ws,
        Surface::Files,
        Surface::Static,
        Surface::Ms,
        Surface::Fs,
        Surface::Mcp,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Surface::Pages => "pages",
            Surface::Ws => "ws",
            Surface::Files => "files",
            Surface::Static => "static",
            Surface::Ms => "ms",
            Surface::Fs => "fs",
            Surface::Mcp => "mcp",
        }
    }

    pub fn parse(raw: &str) -> Option<Surface> {
        Surface::ALL.iter().copied().find(|s| s.key() == raw.trim())
    }

    /// The default mount on a project host. Pages is the root.
    pub fn default_path(self) -> &'static str {
        match self {
            Surface::Pages => "/",
            Surface::Ws => "/_ws/",
            Surface::Files => "/_files/",
            Surface::Static => "/_static/",
            Surface::Ms => "/_ms/",
            Surface::Fs => "/_fs/",
            Surface::Mcp => "/_mcp",
        }
    }

    /// The host-agnostic prefix the platform serves this surface under.
    pub fn platform_prefix(self, owner: &str, project: &str) -> String {
        match self {
            Surface::Pages => format!("/wh/{owner}/{project}"),
            Surface::Ws => format!("/ws/{owner}/{project}"),
            Surface::Files => format!("/files/{owner}/{project}"),
            Surface::Static => format!("/static/{owner}/{project}"),
            Surface::Ms => format!("/ms/{owner}/{project}"),
            Surface::Fs => format!("/fs/{owner}/{project}"),
            Surface::Mcp => format!("/api/projects/{owner}/{project}/mcp"),
        }
    }

    /// Off unless the project turned it on: tiles need a published layer,
    /// private files need a reason to be reachable at all, and the agent
    /// endpoint on a public host is a door nobody asked for — agents work
    /// through the platform address, which this switch never touches.
    pub fn enabled_by_default(self) -> bool {
        !matches!(self, Surface::Ms | Surface::Fs | Surface::Mcp)
    }

    pub fn title(self) -> &'static str {
        match self {
            Surface::Pages => "Pages",
            Surface::Ws => "WebSocket rooms",
            Surface::Files => "Public files",
            Surface::Static => "Static assets and scripts",
            Surface::Ms => "Map tiles",
            Surface::Fs => "Private files",
            Surface::Mcp => "MCP",
        }
    }
}

/// `(host, path prefix) → surface`. A route on a project host overrides the
/// surface's default mount; a route on a host of its own makes that host
/// answer only that surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressRoute {
    pub host: String,
    pub path: String,
    pub surface: Surface,
}

/// What the operator set for one project. Empty means "dev host, default
/// mounts, default switches" — every project starts here and most stay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAddressing {
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub routes: Vec<AddressRoute>,
    /// Surfaces switched off. A project starts with the ones that are off by
    /// default (`Surface::enabled_by_default`); the list is what is stored,
    /// so switching one on is removing it from here.
    #[serde(default = "default_disabled")]
    pub disabled: Vec<Surface>,
    /// The platform API (`/api/projects/{o}/{p}/…`) answers on the project's
    /// hosts. Off by default; the platform address always serves it
    /// (`addressing.md` §2a). Authentication applies either way.
    #[serde(default)]
    pub api_on_hosts: bool,
    /// What a 5xx shows on the project's hosts: `hidden` (the error page with a
    /// reference) or `shown` (plus code, message, node id, run link). A webhook
    /// overrides it for its own routes (`--errors`). Status codes never change.
    #[serde(default)]
    pub errors: ErrorDetail,
}

/// `addressing.md` §2a, `errors`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ErrorDetail {
    #[default]
    Hidden,
    Shown,
}

fn default_disabled() -> Vec<Surface> {
    Surface::ALL.iter().copied().filter(|s| !s.enabled_by_default()).collect()
}

impl Default for ProjectAddressing {
    fn default() -> Self {
        Self { hosts: Vec::new(), routes: Vec::new(), disabled: default_disabled(), api_on_hosts: false, errors: ErrorDetail::Hidden }
    }
}

impl ProjectAddressing {
    pub fn is_enabled(&self, surface: Surface) -> bool {
        !self.disabled.contains(&surface)
    }
}

/// Where one request goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub owner: String,
    pub project: String,
    pub surface: Surface,
    /// The path under the surface, always starting with `/`.
    pub rest: String,
    /// The host the request came in on, as the index knows it.
    pub host: String,
    /// Whether this is the automatic dev host.
    pub dev_host: bool,
}

impl Resolution {
    /// The platform-form path this request is served as.
    ///
    /// `files` is the *public* files surface: `/_files/photos/a.webp` is the
    /// object at `public/photos/a.webp`, so the store's `public/` folder is
    /// inserted here and never written by a page.
    pub fn platform_path(&self) -> String {
        let mut prefix = self.surface.platform_prefix(&self.owner, &self.project);
        if self.surface == Surface::Files {
            prefix.push_str("/public");
        }
        if self.rest == "/" {
            match self.surface {
                Surface::Pages | Surface::Mcp => prefix,
                _ => format!("{prefix}/"),
            }
        } else {
            format!("{prefix}{}", self.rest)
        }
    }
}

#[derive(Default)]
struct Index {
    /// host → (owner, project). Custom hosts only; dev hosts are computed.
    hosts: HashMap<String, (String, String)>,
    /// (owner, project) → addressing, as last read.
    projects: HashMap<(String, String), ProjectAddressing>,
}

pub struct AddressingService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    index: RwLock<Index>,
}

impl AddressingService {
    pub fn new(data: Arc<dyn DataAdapter>, file: Arc<dyn FileAdapter>) -> Self {
        Self { data, file, index: RwLock::new(Index::default()) }
    }

    /// The automatic host every project has: `<project>.<owner>.localhost`.
    pub fn dev_host(owner: &str, project: &str) -> String {
        format!("{}.{}{}", slug_segment(project), slug_segment(owner), DEV_HOST_SUFFIX)
    }

    /// `(owner, project)` when the host is a dev host, whether or not the
    /// project exists — existence is the router's problem, not the parser's.
    pub fn parse_dev_host(host: &str) -> Option<(String, String)> {
        let host = normalize_host(host);
        let stem = host.strip_suffix(DEV_HOST_SUFFIX)?;
        let mut parts = stem.split('.');
        let project = parts.next()?;
        let owner = parts.next()?;
        if parts.next().is_some() || project.is_empty() || owner.is_empty() {
            return None;
        }
        Some((owner.to_string(), project.to_string()))
    }

    /// Read every project's addressing into the index. Called once at boot;
    /// writes keep it current afterwards.
    pub fn rebuild_index(&self) -> Result<usize, PlatformError> {
        let mut index = Index::default();
        for user in self.data.list_users()? {
            for project in self.data.list_projects(&user.owner)? {
                let addressing = self.read_from_disk(&project.owner, &project.project)?;
                for host in &addressing.hosts {
                    index
                        .hosts
                        .insert(host.clone(), (project.owner.clone(), project.project.clone()));
                }
                index
                    .projects
                    .insert((project.owner.clone(), project.project.clone()), addressing);
            }
        }
        let count = index.hosts.len();
        *self.index.write().expect("addressing index poisoned") = index;
        Ok(count)
    }

    fn read_from_disk(&self, owner: &str, project: &str) -> Result<ProjectAddressing, PlatformError> {
        let layout = self.file.ensure_project_layout(owner, project)?;
        let path = layout.data_store_dir().join(ADDRESSING_FILE);
        if !path.exists() {
            return Ok(ProjectAddressing::default());
        }
        let bytes = std::fs::read(&path).map_err(|err| {
            PlatformError::new("ADDRESSING_READ", format!("{}: {err}", path.display()))
        })?;
        serde_json::from_slice(&bytes).map_err(|err| {
            PlatformError::new("ADDRESSING_INVALID", format!("{}: {err}", path.display()))
        })
    }

    /// The project's addressing; the default when nothing was set.
    pub fn read(&self, owner: &str, project: &str) -> Result<ProjectAddressing, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        if let Some(found) = self
            .index
            .read()
            .expect("addressing index poisoned")
            .projects
            .get(&(owner.clone(), project.clone()))
        {
            return Ok(found.clone());
        }
        let addressing = self.read_from_disk(&owner, &project)?;
        self.index
            .write()
            .expect("addressing index poisoned")
            .projects
            .insert((owner, project), addressing.clone());
        Ok(addressing)
    }

    /// Validate, persist, and re-index. Refuses a host another project holds,
    /// a route on a host the project does not have, a pages route anywhere
    /// but `/`, and a path outside the surface grammar.
    pub fn write(
        &self,
        owner: &str,
        project: &str,
        mut addressing: ProjectAddressing,
    ) -> Result<ProjectAddressing, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let dev_host = Self::dev_host(&owner, &project);

        let mut hosts = Vec::new();
        for raw in &addressing.hosts {
            let host = normalize_host(raw);
            if host.is_empty() {
                continue;
            }
            if host == dev_host {
                continue; // implicit, never stored
            }
            if !valid_host(&host) {
                return Err(PlatformError::new(
                    "ADDRESSING_HOST_INVALID",
                    format!("`{raw}` is not a hostname (letters, digits, `-`, dots; no port, no path)"),
                ));
            }
            if host.ends_with(DEV_HOST_SUFFIX) && Self::parse_dev_host(&host).is_some() {
                return Err(PlatformError::new(
                    "ADDRESSING_HOST_RESERVED",
                    format!("`{host}` has the shape of another project's dev host"),
                ));
            }
            if !hosts.contains(&host) {
                hosts.push(host);
            }
        }
        {
            let index = self.index.read().expect("addressing index poisoned");
            for host in &hosts {
                if let Some((o, p)) = index.hosts.get(host)
                    && (o != &owner || p != &project)
                {
                    return Err(PlatformError::new(
                        "ADDRESSING_HOST_TAKEN",
                        format!("`{host}` already answers for `{o}/{p}`"),
                    ));
                }
            }
        }
        addressing.hosts = hosts;

        let mut routes = Vec::new();
        for route in &addressing.routes {
            let host = normalize_host(&route.host);
            if host != dev_host && !addressing.hosts.contains(&host) {
                return Err(PlatformError::new(
                    "ADDRESSING_ROUTE_HOST",
                    format!("route `{}{}` names a host this project does not have; add the host first", route.host, route.path),
                ));
            }
            let path = normalize_route_path(&route.path, route.surface)?;
            if routes.iter().any(|r: &AddressRoute| r.host == host && r.path == path) {
                return Err(PlatformError::new(
                    "ADDRESSING_ROUTE_DUPLICATE",
                    format!("two routes at `{host}{path}`"),
                ));
            }
            routes.push(AddressRoute { host, path, surface: route.surface });
        }
        addressing.routes = routes;
        addressing.disabled.sort_by_key(|s| s.key());
        addressing.disabled.dedup();

        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let dir = layout.data_store_dir();
        std::fs::create_dir_all(&dir).map_err(|err| {
            PlatformError::new("ADDRESSING_WRITE", format!("{}: {err}", dir.display()))
        })?;
        let path = dir.join(ADDRESSING_FILE);
        let bytes = serde_json::to_vec_pretty(&addressing)
            .map_err(|err| PlatformError::new("ADDRESSING_WRITE", err.to_string()))?;
        crate::rwe::core::write_atomic(&path, &bytes).map_err(|err| {
            PlatformError::new("ADDRESSING_WRITE", format!("{}: {err}", path.display()))
        })?;

        let mut index = self.index.write().expect("addressing index poisoned");
        index
            .hosts
            .retain(|_, (o, p)| !(o == &owner && p == &project));
        for host in &addressing.hosts {
            index.hosts.insert(host.clone(), (owner.clone(), project.clone()));
        }
        index.projects.insert((owner, project), addressing.clone());
        Ok(addressing)
    }

    /// Where a request for `host` + `path` goes, if the host is a project's.
    /// `None` means "not a project host — route as the platform".
    pub fn resolve(&self, host: &str, path: &str) -> Option<Resolution> {
        let host = normalize_host(host);
        if host.is_empty() {
            return None;
        }
        let (owner, project, dev_host) = if let Some((o, p)) = Self::parse_dev_host(&host) {
            (o, p, true)
        } else {
            let index = self.index.read().expect("addressing index poisoned");
            let (o, p) = index.hosts.get(&host)?.clone();
            (o, p, false)
        };
        let addressing = self.read(&owner, &project).unwrap_or_default();
        let path = if path.is_empty() { "/" } else { path };

        // Custom routes on this host, longest prefix first.
        let mut candidates: Vec<&AddressRoute> = addressing
            .routes
            .iter()
            .filter(|r| r.host == host)
            .collect();
        candidates.sort_by_key(|r| std::cmp::Reverse(r.path.len()));
        let custom_hit = candidates
            .iter()
            .find(|r| path_matches(path, &r.path))
            .map(|r| (r.surface, strip_prefix(path, &r.path)));

        // A host that carries any custom route serves only its routes — a
        // dedicated tiles host does not also serve the site at `/`.
        let hit = match custom_hit {
            Some(hit) => Some(hit),
            None if !candidates.is_empty() => None,
            None => {
                // Default mounts: `_` prefixes, then pages at the root.
                Surface::ALL
                    .iter()
                    .copied()
                    .filter(|s| *s != Surface::Pages)
                    .find(|s| path_matches(path, s.default_path()))
                    .map(|s| (s, strip_prefix(path, s.default_path())))
                    .or(Some((Surface::Pages, path.to_string())))
            }
        };
        let (surface, rest) = hit?;
        if !addressing.is_enabled(surface) {
            return Some(Resolution {
                owner,
                project,
                surface,
                rest: String::new(), // the empty rest marks "disabled" for the gate
                host,
                dev_host,
            });
        }
        Some(Resolution { owner, project, surface, rest, host, dev_host })
    }

    /// Whether a platform-form request (`/files/{o}/{p}/…`) is for a surface
    /// the project switched off.
    pub fn platform_form_disabled(&self, path: &str) -> bool {
        let mut parts = path.trim_start_matches('/').splitn(4, '/');
        let (first, owner, project) = (parts.next(), parts.next(), parts.next());
        let (Some(first), Some(owner), Some(project)) = (first, owner, project) else {
            return false;
        };
        let surface = match first {
            "wh" => Surface::Pages,
            "ws" => Surface::Ws,
            "files" => Surface::Files,
            "static" => Surface::Static,
            "ms" => Surface::Ms,
            "fs" => Surface::Fs,
            _ => return false,
        };
        let project = project.split(['?', '#']).next().unwrap_or(project);
        self.read(owner, project)
            .map(|a| !a.is_enabled(surface))
            .unwrap_or(false)
    }
}

/// Lowercase, no port, no trailing dot.
pub fn normalize_host(raw: &str) -> String {
    let host = raw.trim().to_ascii_lowercase();
    let host = host.strip_prefix('[').unwrap_or(&host).to_string();
    let host = match host.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) && !h.contains(':') => h.to_string(),
        _ => host,
    };
    host.trim_end_matches('.').to_string()
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// A route path: starts with `/`; a prefix ends with `/` except the exact
/// mounts (`/_mcp`); pages may only be `/`.
fn normalize_route_path(raw: &str, surface: Surface) -> Result<String, PlatformError> {
    let mut path = raw.trim().to_string();
    if path.is_empty() {
        path = "/".to_string();
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if surface == Surface::Pages && path != "/" {
        return Err(PlatformError::new(
            "ADDRESSING_PAGES_ROOT",
            "pages answer at the root of a host or not at all — a page's links are root-relative and would escape a sub-path",
        ));
    }
    if surface != Surface::Pages && surface != Surface::Mcp && !path.ends_with('/') {
        path.push('/');
    }
    if path.contains("..") || path.contains('?') || path.contains('#') || path.contains(' ') {
        return Err(PlatformError::new("ADDRESSING_ROUTE_PATH", format!("`{raw}` is not a route path")));
    }
    Ok(path)
}

fn path_matches(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }
    if prefix.ends_with('/') {
        path.starts_with(prefix) || path == prefix.trim_end_matches('/')
    } else {
        path == prefix || path.starts_with(&format!("{prefix}/")) || path.starts_with(&format!("{prefix}?"))
    }
}

fn strip_prefix(path: &str, prefix: &str) -> String {
    if prefix == "/" {
        return path.to_string();
    }
    let cut = prefix.trim_end_matches('/');
    let rest = path.strip_prefix(cut).unwrap_or("");
    if rest.is_empty() || rest.starts_with('?') {
        format!("/{rest}")
    } else {
        rest.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dev_host_is_project_dot_owner_dot_localhost_and_parses_back() {
        assert_eq!(AddressingService::dev_host("superadmin", "northside"), "northside.superadmin.localhost");
        assert_eq!(
            AddressingService::parse_dev_host("Northside.superadmin.localhost:10610"),
            Some(("superadmin".to_string(), "northside".to_string()))
        );
        assert_eq!(AddressingService::parse_dev_host("localhost:10610"), None);
        assert_eq!(AddressingService::parse_dev_host("a.b.c.localhost"), None);
        assert_eq!(AddressingService::parse_dev_host("northside.example"), None);
    }

    #[test]
    fn default_mounts_put_pages_at_the_root_and_surfaces_under_underscore() {
        assert!(path_matches("/book", "/"));
        assert!(path_matches("/_files/a.jpg", Surface::Files.default_path()));
        assert!(path_matches("/_mcp", Surface::Mcp.default_path()));
        assert!(!path_matches("/_mcpx", Surface::Mcp.default_path()));
        assert_eq!(strip_prefix("/_files/a.jpg", "/_files/"), "/a.jpg");
        assert_eq!(strip_prefix("/_files", "/_files/"), "/");
        assert_eq!(strip_prefix("/service/tiles/1/2/3", "/service/"), "/tiles/1/2/3");
    }

    #[test]
    fn a_pages_route_off_the_root_is_refused_and_prefixes_are_normalised() {
        assert!(normalize_route_path("/shop/", Surface::Pages).is_err());
        assert_eq!(normalize_route_path("service", Surface::Ms).unwrap(), "/service/");
        assert_eq!(normalize_route_path("", Surface::Fs).unwrap(), "/");
        assert_eq!(normalize_route_path("/_mcp", Surface::Mcp).unwrap(), "/_mcp");
    }

    #[test]
    fn a_fresh_project_has_tiles_and_private_files_off() {
        let fresh = ProjectAddressing::default();
        assert!(!fresh.is_enabled(Surface::Ms));
        assert!(!fresh.is_enabled(Surface::Fs));
        assert!(fresh.is_enabled(Surface::Pages) && fresh.is_enabled(Surface::Files));
        let stored: ProjectAddressing = serde_json::from_str(r#"{"hosts":["a.example"]}"#).unwrap();
        assert_eq!(stored.disabled, fresh.disabled, "an old file without the key gets the defaults");
        let explicit: ProjectAddressing = serde_json::from_str(r#"{"disabled":[]}"#).unwrap();
        assert!(explicit.is_enabled(Surface::Ms), "an explicit empty list means everything on");
    }

    #[test]
    fn hosts_normalise_and_validate() {
        assert_eq!(normalize_host(" Northside.Example:443 "), "northside.example");
        assert!(valid_host("zebms.mydomain.com"));
        assert!(!valid_host("localhost"));
        assert!(!valid_host("bad_host.example"));
        assert!(!valid_host("-x.example"));
    }

    #[test]
    fn a_resolution_maps_to_the_platform_form() {
        let r = Resolution {
            owner: "o".into(),
            project: "p".into(),
            surface: Surface::Files,
            rest: "/a.jpg".into(),
            host: "p.o.localhost".into(),
            dev_host: true,
        };
        assert_eq!(r.platform_path(), "/files/o/p/public/a.jpg", "the public surface adds the store's public/ folder");
        let r = Resolution { surface: Surface::Pages, rest: "/".into(), ..r };
        assert_eq!(r.platform_path(), "/wh/o/p");
        let r = Resolution { surface: Surface::Pages, rest: "/book?x=1".into(), ..r };
        assert_eq!(r.platform_path(), "/wh/o/p/book?x=1");
    }
}

// ── Generated web-server configuration ───────────────────────────────────────

/// Everything a proxy needs to know. Zebflow routes by `Host`, so every
/// server block is the same: pass the request through with the Host header
/// intact, keep WebSocket upgrades, allow the instance's upload size.
pub struct ProxyFacts<'a> {
    pub hosts: &'a [String],
    /// `host:port` the proxy forwards to — the instance's listen address.
    pub upstream: &'a str,
    pub upload_mb: u32,
    pub owner: &'a str,
    pub project: &'a str,
}

/// One config per server, each ending in the command that proves it.
/// Keyed by a stable id the UI renders as a button; order is the order shown.
pub fn server_configs(facts: &ProxyFacts<'_>) -> Vec<(&'static str, &'static str, String)> {
    if facts.hosts.is_empty() {
        return Vec::new();
    }
    vec![
        ("nginx", "Nginx & OpenResty", nginx(facts)),
        ("apache", "Apache httpd", apache(facts)),
        ("caddy", "Caddy", caddy(facts)),
        ("traefik", "Traefik", traefik(facts)),
        ("cloudflared", "Cloudflare Tunnel", cloudflared(facts)),
        ("haproxy", "HAProxy", haproxy(facts)),
        ("compose", "Docker Compose + Caddy", compose(facts)),
    ]
}

fn check_lines(facts: &ProxyFacts<'_>) -> String {
    let first = &facts.hosts[0];
    format!(
        "# Check: every host must answer from this project.\n\
         #   curl -sI https://{first}/ | grep -i x-zebflow-project     → x-zebflow-project: {owner}/{project}\n\
         #   Settings → Addressing shows DNS ✓ and Verify ✓ for each host.",
        owner = facts.owner,
        project = facts.project
    )
}

fn nginx(facts: &ProxyFacts<'_>) -> String {
    let names = facts.hosts.join(" ");
    format!(
        "# {names} → Zebflow project {owner}/{project}\n\
         # Zebflow routes by Host: the proxy_set_header Host line is required.\n\
         server {{\n\
         \x20   listen 80;\n\
         \x20   listen 443 ssl http2;                 # certbot --nginx -d {first} fills in the certificate lines\n\
         \x20   server_name {names};\n\
         \x20   client_max_body_size {mb}m;             # this instance's upload limit\n\
         \n\
         \x20   location / {{\n\
         \x20       proxy_pass http://{upstream};\n\
         \x20       proxy_set_header Host $host;              # required\n\
         \x20       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n\
         \x20       proxy_set_header X-Forwarded-Proto $scheme;\n\
         \x20       proxy_http_version 1.1;                    # WebSocket rooms (/_ws) and SSE\n\
         \x20       proxy_set_header Upgrade $http_upgrade;\n\
         \x20       proxy_set_header Connection $connection_upgrade;\n\
         \x20       proxy_read_timeout 3600s;\n\
         \x20       proxy_buffering off;\n\
         \x20   }}\n\
         }}\n\
         # Once, in the http {{}} block, if not already there:\n\
         map $http_upgrade $connection_upgrade {{ default upgrade; '' close; }}\n\
         \n\
         # Then:  sudo nginx -t && sudo systemctl reload nginx\n\
         #        sudo certbot --nginx {certbot}\n\
         {check}\n",
        names = names,
        first = facts.hosts[0],
        owner = facts.owner,
        project = facts.project,
        mb = facts.upload_mb,
        upstream = facts.upstream,
        certbot = facts.hosts.iter().map(|h| format!("-d {h}")).collect::<Vec<_>>().join(" "),
        check = check_lines(facts),
    )
}

fn apache(facts: &ProxyFacts<'_>) -> String {
    let first = &facts.hosts[0];
    let aliases = facts.hosts.iter().skip(1).map(|h| format!("    ServerAlias {h}\n")).collect::<String>();
    format!(
        "# {names} → Zebflow project {owner}/{project}\n\
         # a2enmod proxy proxy_http proxy_wstunnel headers rewrite ssl\n\
         <VirtualHost *:80>\n\
         \x20   ServerName {first}\n\
         {aliases}\
         \x20   ProxyPreserveHost On                     # required: Zebflow routes by Host\n\
         \x20   ProxyRequests Off\n\
         \x20   LimitRequestBody {bytes}             # this instance's upload limit\n\
         \x20   RequestHeader set X-Forwarded-Proto expenv=HTTPS\n\
         \n\
         \x20   # WebSocket rooms (/_ws) and everything else\n\
         \x20   RewriteEngine On\n\
         \x20   RewriteCond %{{HTTP:Upgrade}} =websocket [NC]\n\
         \x20   RewriteRule ^/(.*)  ws://{upstream}/$1 [P,L]\n\
         \x20   ProxyPass        / http://{upstream}/ timeout=3600\n\
         \x20   ProxyPassReverse / http://{upstream}/\n\
         </VirtualHost>\n\
         # TLS:  sudo certbot --apache {certbot}   (adds the *:443 vhost)\n\
         # Then: sudo apachectl configtest && sudo systemctl reload apache2\n\
         {check}\n",
        names = facts.hosts.join(" "),
        owner = facts.owner,
        project = facts.project,
        bytes = u64::from(facts.upload_mb) * 1024 * 1024,
        upstream = facts.upstream,
        certbot = facts.hosts.iter().map(|h| format!("-d {h}")).collect::<Vec<_>>().join(" "),
        check = check_lines(facts),
    )
}

fn caddy(facts: &ProxyFacts<'_>) -> String {
    format!(
        "# {names} → Zebflow project {owner}/{project}\n\
         # Caddy issues and renews TLS on its own; Host is passed through by default.\n\
         {names_comma} {{\n\
         \x20   request_body {{\n\
         \x20       max_size {mb}MB\n\
         \x20   }}\n\
         \x20   reverse_proxy {upstream}\n\
         }}\n\
         # Then:  caddy validate --config Caddyfile && caddy reload --config Caddyfile\n\
         {check}\n",
        names = facts.hosts.join(" "),
        names_comma = facts.hosts.join(", "),
        owner = facts.owner,
        project = facts.project,
        mb = facts.upload_mb,
        upstream = facts.upstream,
        check = check_lines(facts),
    )
}

fn traefik(facts: &ProxyFacts<'_>) -> String {
    let rule = facts.hosts.iter().map(|h| format!("Host(`{h}`)")).collect::<Vec<_>>().join(" || ");
    format!(
        "# {names} → Zebflow project {owner}/{project}  (dynamic configuration, file provider)\n\
         http:\n\
         \x20 routers:\n\
         \x20   zebflow-{project}:\n\
         \x20     rule: \"{rule}\"\n\
         \x20     entryPoints: [websecure]\n\
         \x20     service: zebflow-{project}\n\
         \x20     tls:\n\
         \x20       certResolver: letsencrypt\n\
         \x20 services:\n\
         \x20   zebflow-{project}:\n\
         \x20     loadBalancer:\n\
         \x20       passHostHeader: true                  # required: Zebflow routes by Host\n\
         \x20       servers:\n\
         \x20         - url: \"http://{upstream}\"\n\
         # Static config needs an entryPoint `websecure` (:443) and a certResolver `letsencrypt`;\n\
         # raise the request body limit with a buffering middleware: maxRequestBodyBytes: {bytes}\n\
         {check}\n",
        names = facts.hosts.join(" "),
        owner = facts.owner,
        project = facts.project,
        rule = rule,
        upstream = facts.upstream,
        bytes = u64::from(facts.upload_mb) * 1024 * 1024,
        check = check_lines(facts),
    )
}

fn cloudflared(facts: &ProxyFacts<'_>) -> String {
    let ingress = facts
        .hosts
        .iter()
        .map(|h| {
            format!(
                "  - hostname: {h}\n    service: http://{upstream}\n    originRequest:\n      httpHostHeader: {h}      # required: Zebflow routes by Host\n",
                upstream = facts.upstream
            )
        })
        .collect::<String>();
    format!(
        "# {names} → Zebflow project {owner}/{project}  (no public IP needed)\n\
         # cloudflared tunnel create zebflow-{project}\n\
         # cloudflared tunnel route dns zebflow-{project} <each host>\n\
         tunnel: zebflow-{project}\n\
         credentials-file: ~/.cloudflared/<tunnel-id>.json\n\
         ingress:\n\
         {ingress}\
         \x20 - service: http_status:404\n\
         # Then:  cloudflared tunnel run zebflow-{project}\n\
         # Uploads above 100 MB need Cloudflare's paid plan or a direct host for /_files.\n\
         {check}\n",
        names = facts.hosts.join(" "),
        owner = facts.owner,
        project = facts.project,
        ingress = ingress,
        check = check_lines(facts),
    )
}

fn haproxy(facts: &ProxyFacts<'_>) -> String {
    let acl = facts.hosts.iter().map(|h| format!("    acl is_{p} hdr(host) -i {h}\n", p = facts.project)).collect::<String>();
    format!(
        "# {names} → Zebflow project {owner}/{project}\n\
         frontend https_in\n\
         \x20   bind *:443 ssl crt /etc/haproxy/certs/\n\
         \x20   bind *:80\n\
         \x20   http-request set-header X-Forwarded-Proto https if {{ ssl_fc }}\n\
         {acl}\
         \x20   use_backend zebflow_{project} if is_{project}\n\
         \n\
         backend zebflow_{project}\n\
         \x20   # Host is forwarded unchanged by default — required: Zebflow routes by Host\n\
         \x20   timeout tunnel 1h                        # WebSocket rooms (/_ws)\n\
         \x20   server zebflow {upstream} check\n\
         # Request body limit: set `tune.bufsize` / `option http-buffer-request` per upload needs ({mb} MB here).\n\
         # Then:  haproxy -c -f /etc/haproxy/haproxy.cfg && sudo systemctl reload haproxy\n\
         {check}\n",
        names = facts.hosts.join(" "),
        owner = facts.owner,
        project = facts.project,
        acl = acl,
        upstream = facts.upstream,
        mb = facts.upload_mb,
        check = check_lines(facts),
    )
}

fn compose(facts: &ProxyFacts<'_>) -> String {
    format!(
        "# One VPS, one file: Caddy in front of this Zebflow, TLS automatic.\n\
         # {names} → Zebflow project {owner}/{project}\n\
         services:\n\
         \x20 zebflow:\n\
         \x20   image: zebflow/zebflow:latest\n\
         \x20   environment:\n\
         \x20     ZEBFLOW_PLATFORM_HOST: 0.0.0.0\n\
         \x20     ZEBFLOW_PLATFORM_PORT: \"10610\"\n\
         \x20   volumes:\n\
         \x20     - zebflow-data:/var/lib/zebflow/data\n\
         \x20 caddy:\n\
         \x20   image: caddy:2\n\
         \x20   ports: [\"80:80\", \"443:443\"]\n\
         \x20   volumes:\n\
         \x20     - ./Caddyfile:/etc/caddy/Caddyfile:ro\n\
         \x20     - caddy-data:/data\n\
         volumes:\n\
         \x20 zebflow-data: {{}}\n\
         \x20 caddy-data: {{}}\n\
         \n\
         # Caddyfile beside it:\n\
         # {names_comma} {{\n\
         #     request_body {{ max_size {mb}MB }}\n\
         #     reverse_proxy zebflow:10610\n\
         # }}\n\
         # Then:  docker compose up -d\n\
         {check}\n",
        names = facts.hosts.join(" "),
        names_comma = facts.hosts.join(", "),
        owner = facts.owner,
        project = facts.project,
        mb = facts.upload_mb,
        check = check_lines(facts),
    )
}

#[cfg(test)]
mod config_tests {
    use super::*;

    /// Every generated config names every host, passes the Host header, and
    /// ends with its check — the three things that decide whether it works.
    #[test]
    fn every_server_config_carries_hosts_host_header_and_its_check() {
        let hosts = vec!["northside.example".to_string(), "zebfs.mydomain.com".to_string()];
        let facts = ProxyFacts { hosts: &hosts, upstream: "127.0.0.1:10610", upload_mb: 1024, owner: "acme", project: "northside" };
        let configs = server_configs(&facts);
        assert_eq!(configs.len(), 7);
        for (id, _title, text) in configs {
            for host in &hosts {
                assert!(text.contains(host), "{id} names {host}");
            }
            if id != "compose" {
                assert!(text.contains("127.0.0.1:10610"), "{id} points at the upstream");
            }
            assert!(text.to_lowercase().contains("host"), "{id} mentions the Host header");
            assert!(text.contains("x-zebflow-project"), "{id} ends with its check");
        }
        assert!(server_configs(&ProxyFacts { hosts: &[], ..facts }).is_empty());
    }
}
