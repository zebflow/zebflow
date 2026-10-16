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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::slug_segment;

pub mod sections;
use sections::SectionTable;

/// The file under `data/store/` — store tier: irreplaceable, never in the repo.
pub const ADDRESSING_FILE: &str = "addressing.json";
/// The suffix every dev host ends in.
pub const DEV_HOST_SUFFIX: &str = ".localhost";

/// `localhost` or a name under it — the dev hosts' namespace, never a
/// project's stored host.
fn is_localhost_name(host: &str) -> bool {
    let host = normalize_host(host);
    host == "localhost" || host.ends_with(DEV_HOST_SUFFIX)
}
/// The suffix every dev file host ends in.
pub const DEV_FILE_HOST_SUFFIX: &str = ".fs.localhost";
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
    Mcp,
}

impl Surface {
    pub const ALL: [Surface; 6] = [
        Surface::Pages,
        Surface::Ws,
        Surface::Files,
        Surface::Static,
        Surface::Ms,
        Surface::Mcp,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Surface::Pages => "pages",
            Surface::Ws => "ws",
            Surface::Files => "files",
            Surface::Static => "static",
            Surface::Ms => "ms",
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
            Surface::Mcp => "/_mcp/",
        }
    }

    /// The host-agnostic prefix the platform serves this surface under.
    /// `files` has none (`addressing.md` §2): exposed objects answer only
    /// through the ZebFS gateway on a project's hosts.
    pub fn platform_prefix(self, owner: &str, project: &str) -> Option<String> {
        Some(match self {
            Surface::Pages => format!("/wh/{owner}/{project}"),
            Surface::Ws => format!("/ws/{owner}/{project}"),
            Surface::Files => return None,
            Surface::Static => format!("/static/{owner}/{project}"),
            Surface::Ms => format!("/ms/{owner}/{project}"),
            Surface::Mcp => format!("/mcp/{owner}/{project}"),
        })
    }

    /// Off unless the project turned it on: WebSocket rooms, map tiles,
    /// exposed files, and published MCP servers are all opt-in surfaces.
    /// The switch governs both forms (`/_ws` and `/ws/{o}/{p}`, etc.). The
    /// project's dev MCP is not a surface: it answers only on the platform
    /// address.
    pub fn enabled_by_default(self) -> bool {
        !matches!(self, Surface::Ws | Surface::Ms | Surface::Files | Surface::Mcp)
    }

    pub fn title(self) -> &'static str {
        match self {
            Surface::Pages => "Pages",
            Surface::Ws => "WebSocket rooms",
            Surface::Files => "Exposed files (sandboxed)",
            Surface::Static => "Static assets and scripts",
            Surface::Ms => "Map tiles",
            Surface::Mcp => "Published MCP servers",
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
    /// A section's root (`/site-a`) on a `/` pages route or an `mcp` route
    /// (`addressing.md` §2c). Absent on every rootless project, and never
    /// written when absent, so a rootless file round-trips unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
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
    #[serde(default = "default_disabled", deserialize_with = "known_surfaces")]
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

/// The stored switch list, keeping only surfaces this build has: a word for a
/// surface that no longer exists (`fs`) is a switch for nothing.
fn known_surfaces<'de, D>(deserializer: D) -> Result<Vec<Surface>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let words: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(words.iter().filter_map(|word| Surface::parse(word)).collect())
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

    /// Hosts carrying a `files` route: the project's named file hosts, which
    /// answer exposed files and nothing else (`addressing.md` §2b).
    pub fn file_hosts(&self) -> Vec<&str> {
        self.routes
            .iter()
            .filter(|r| r.surface == Surface::Files)
            .map(|r| r.host.as_str())
            .collect()
    }

    /// The hosts a `public_execute` folder may name in `serve`: the dev host
    /// and every named host except a file host, where nothing runs.
    pub fn site_hosts(&self, dev_host: String) -> Vec<String> {
        let file_hosts = self.file_hosts();
        // A section host is never a site host (§2c); on a rootless project
        // this list is empty.
        let section_hosts = self.section_hosts();
        let mut hosts: Vec<String> = self
            .hosts
            .iter()
            .filter(|h| !file_hosts.contains(&h.as_str()) && !section_hosts.contains(&h.as_str()))
            .cloned()
            .collect();
        hosts.push(dev_host);
        hosts
    }

    /// Hosts whose `/` pages route carries a root (`addressing.md` §2c).
    pub fn section_hosts(&self) -> Vec<&str> {
        self.routes
            .iter()
            .filter(|r| r.root.is_some() && r.surface == Surface::Pages && r.path == "/")
            .map(|r| r.host.as_str())
            .collect()
    }

    /// Whether any route carries a root: a sectioned project.
    pub fn is_sectioned(&self) -> bool {
        self.routes.iter().any(|r| r.root.is_some())
    }

    /// Where a page's `zeb/rooms` opens room sockets: the path a room name is
    /// appended to, for the address the page was served on (`addressing.md`
    /// §2, `ws`). `page_host` is the project host the page arrived on; `None`
    /// is the platform form `/wh/{o}/{p}/…`.
    ///
    /// `None` when `ws` is switched off, or when the page's host carries
    /// routes and none of them is `ws` — `/_ws/` misses there, and the
    /// platform form would name the owner and project on the project's own
    /// address. A section host keeps the default mount (§2c).
    pub fn room_base(&self, owner: &str, project: &str, page_host: Option<&str>) -> Option<String> {
        if !self.is_enabled(Surface::Ws) {
            return None;
        }
        let Some(host) = page_host else {
            return Surface::Ws.platform_prefix(owner, project).map(|prefix| format!("{prefix}/rooms/"));
        };
        let host = normalize_host(host);
        if let Some(route) = self.routes.iter().find(|r| r.host == host && r.surface == Surface::Ws) {
            return Some(format!("{}/rooms/", route.path.trim_end_matches('/')));
        }
        let carries_routes = self.routes.iter().any(|r| r.host == host);
        if !carries_routes || self.section_hosts().contains(&host.as_str()) {
            return Some(format!("{}rooms/", Surface::Ws.default_path()));
        }
        None
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
    /// Whether this host is a named file host (`addressing.md` §2b).
    pub file_host: bool,
    /// The host carries routes and none matched: only this project's own
    /// platform forms pass; anything else is 404, never a platform route.
    pub miss: bool,
}

impl Resolution {
    /// The platform-form path this request is served as; none for `files`,
    /// which the gateway answers itself (`/_files/photos/a.webp` is the store
    /// key `photos/a.webp`, readable only when the exposure rules say so).
    pub fn platform_path(&self) -> Option<String> {
        let prefix = self.surface.platform_prefix(&self.owner, &self.project)?;
        Some(if self.rest == "/" {
            match self.surface {
                Surface::Pages => prefix,
                _ => format!("{prefix}/"),
            }
        } else {
            format!("{prefix}{}", self.rest)
        })
    }
}

#[derive(Default)]
struct Index {
    /// host → (owner, project). Custom hosts and section dev hosts; the
    /// two-label dev hosts are computed.
    hosts: HashMap<String, (String, String)>,
    /// (owner, project) → addressing, as last read.
    projects: HashMap<(String, String), ProjectAddressing>,
    /// Sectioned projects only: their prepared routing (§2c).
    sections: HashMap<(String, String), Arc<SectionTable>>,
    /// Every host of a sectioned project — named, dev, section dev — to it.
    section_hosts: HashMap<String, (String, String)>,
}

impl Index {
    /// Put a project's sections into the index, or take them out when it has
    /// none. Answers the section dev hosts skipped because another project
    /// holds the name.
    fn place_sections(&mut self, owner: &str, project: &str, table: Option<SectionTable>) -> Vec<String> {
        let key = (owner.to_string(), project.to_string());
        self.section_hosts.retain(|_, held| held != &key);
        if let Some(old) = self.sections.remove(&key) {
            for host in old.section_dev_hosts.values() {
                if self.hosts.get(host) == Some(&key) {
                    self.hosts.remove(host);
                }
            }
        }
        let Some(mut table) = table else {
            return Vec::new();
        };
        let mut skipped = Vec::new();
        let labels: Vec<(String, String)> = table.section_dev_hosts.iter().map(|(l, h)| (l.clone(), h.clone())).collect();
        for (label, host) in labels {
            match self.hosts.get(&host) {
                Some(held) if held != &key => {
                    table.section_dev_hosts.remove(&label);
                    table.hosts.remove(&host);
                    skipped.push(host);
                }
                _ => {
                    self.hosts.insert(host, key.clone());
                }
            }
        }
        for host in table.hosts.keys() {
            self.section_hosts.insert(host.clone(), key.clone());
        }
        self.sections.insert(key, Arc::new(table));
        skipped
    }
}

pub struct AddressingService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    index: RwLock<Index>,
    /// Whether any project on this instance has a root. A rootless instance
    /// pays one relaxed load for §2c and nothing else.
    any_sections: AtomicBool,
}

impl AddressingService {
    pub fn new(data: Arc<dyn DataAdapter>, file: Arc<dyn FileAdapter>) -> Self {
        Self { data, file, index: RwLock::new(Index::default()), any_sections: AtomicBool::new(false) }
    }

    /// The prepared routing of the sectioned project a host belongs to, and
    /// the host as the table knows it. `None` for every host of a rootless
    /// project, at the cost of one atomic load on an instance with none.
    pub fn section_of_host(&self, host: &str) -> Option<(Arc<SectionTable>, String)> {
        if !self.any_sections.load(Ordering::Relaxed) {
            return None;
        }
        let host = normalize_host(host);
        let index = self.index.read().expect("addressing index poisoned");
        let key = index.section_hosts.get(&host)?;
        Some((index.sections.get(key)?.clone(), host))
    }

    /// The prepared routing of a sectioned project; `None` when rootless.
    pub fn sections(&self, owner: &str, project: &str) -> Option<Arc<SectionTable>> {
        if !self.any_sections.load(Ordering::Relaxed) {
            return None;
        }
        let key = (slug_segment(owner), slug_segment(project));
        if let Some(found) = self.index.read().expect("addressing index poisoned").sections.get(&key) {
            return Some(found.clone());
        }
        None
    }

    /// Rebuild one sectioned project's routing after its ACL changed: the
    /// hosts a folder serves are part of it.
    pub fn refresh_sections(&self, owner: &str, project: &str) {
        if self.sections(owner, project).is_none() {
            return;
        }
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Ok(addressing) = self.read(&owner, &project) else { return };
        let table = self.section_table(&owner, &project, &addressing);
        self.index.write().expect("addressing index poisoned").place_sections(&owner, &project, table);
    }

    fn section_table(&self, owner: &str, project: &str, addressing: &ProjectAddressing) -> Option<SectionTable> {
        if !addressing.is_sectioned() {
            return None;
        }
        let serve = self
            .file
            .ensure_project_layout(owner, project)
            .map(|layout| sections::serve_hosts_of(&layout.data_store_dir()))
            .unwrap_or_default();
        self.any_sections.store(true, Ordering::Relaxed);
        SectionTable::build(owner, project, addressing, &serve)
    }

    /// The automatic host every project has: `<project>.<owner>.localhost`.
    pub fn dev_host(owner: &str, project: &str) -> String {
        format!("{}.{}{}", slug_segment(project), slug_segment(owner), DEV_HOST_SUFFIX)
    }

    /// The automatic file host every project has:
    /// `<project>.<owner>.fs.localhost`. Its own origin, never the Studio's or
    /// a page's, so nothing it serves shares their cookies
    /// (`kinds/zebfs-acl` §Resolution).
    pub fn dev_file_host(owner: &str, project: &str) -> String {
        format!(
            "{}.{}{}",
            slug_segment(project),
            slug_segment(owner),
            DEV_FILE_HOST_SUFFIX
        )
    }

    /// `(owner, project)` when the host is a dev file host.
    pub fn parse_dev_file_host(host: &str) -> Option<(String, String)> {
        let host = normalize_host(host);
        let stem = host.strip_suffix(DEV_FILE_HOST_SUFFIX)?;
        let mut parts = stem.split('.');
        let project = parts.next()?;
        let owner = parts.next()?;
        if parts.next().is_some() || project.is_empty() || owner.is_empty() {
            return None;
        }
        Some((owner.to_string(), project.to_string()))
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
        let mut sectioned = Vec::new();
        for user in self.data.list_users()? {
            for project in self.data.list_projects(&user.owner)? {
                let addressing = self.read_from_disk(&project.owner, &project.project)?;
                for host in &addressing.hosts {
                    if sections::is_platform_host(host) || is_localhost_name(host) {
                        continue;
                    }
                    index
                        .hosts
                        .insert(host.clone(), (project.owner.clone(), project.project.clone()));
                }
                if addressing.is_sectioned() {
                    sectioned.push((project.owner.clone(), project.project.clone(), addressing.clone()));
                }
                index
                    .projects
                    .insert((project.owner.clone(), project.project.clone()), addressing);
            }
        }
        // Section dev hosts after every named host: a collision drops only
        // the section dev host, never a rootless project's host (§2c).
        for (owner, project, addressing) in sectioned {
            let table = self.section_table(&owner, &project, &addressing);
            for host in index.place_sections(&owner, &project, table) {
                eprintln!("addressing: {owner}/{project}: section dev host `{host}` is another project's named host; it does not answer");
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
        if self.data.get_project(&owner, &project)?.is_none() {
            return Ok(ProjectAddressing::default());
        }
        let addressing = self.read_from_disk(&owner, &project)?;
        let table = self.section_table(&owner, &project, &addressing);
        let mut index = self.index.write().expect("addressing index poisoned");
        if table.is_some() {
            index.place_sections(&owner, &project, table);
        }
        index.projects.insert((owner, project), addressing.clone());
        Ok(addressing)
    }

    /// Validate, persist, and re-index. Refuses a host another project holds,
    /// a route on a host the project does not have, a pages route anywhere
    /// but `/`, and a path outside the surface grammar.
    pub fn write(
        &self,
        owner: &str,
        project: &str,
        addressing: ProjectAddressing,
    ) -> Result<ProjectAddressing, PlatformError> {
        self.write_checked(owner, project, addressing, |_| Ok(()))
    }

    /// [`Self::write`], with `check` run on a sectioned project's prepared
    /// routing before anything is stored: the save refuses what the caller
    /// can see and this service cannot (registered routes, `addressing.md`
    /// §2c). Never called for a rootless project.
    pub fn write_checked(
        &self,
        owner: &str,
        project: &str,
        mut addressing: ProjectAddressing,
        check: impl Fn(&SectionTable) -> Result<(), PlatformError>,
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
            // A section dev host of this project is implicit too (§2c).
            if addressing.routes.iter().filter_map(|r| r.root.as_deref()).filter_map(|r| sections::root_label(r).ok())
                .any(|label| sections::section_dev_host(&owner, &project, &label) == host)
            {
                continue;
            }
            if sections::is_platform_host(raw) {
                return Err(PlatformError::new(
                    "ADDRESSING_HOST_RESERVED",
                    format!("`{host}` is the platform's own address; a project cannot hold it"),
                ));
            }
            if !valid_host(&host) {
                return Err(PlatformError::new(
                    "ADDRESSING_HOST_INVALID",
                    format!("`{raw}` is not a hostname (letters, digits, `-`, dots; no port, no path)"),
                ));
            }
            // Every browser sends `*.localhost` to loopback: the names are the
            // dev hosts'. A project's own dev and section dev hosts were
            // skipped above as implicit; any other one is another project's,
            // or a squat on a name one could take later.
            if is_localhost_name(&host) {
                return Err(PlatformError::new(
                    "ADDRESSING_HOST_RESERVED",
                    format!("`{host}` is a `.localhost` name; only the project's own dev hosts answer there, and they need no saving"),
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
            let root = match route.root.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
                Some(raw) => Some(format!("/{}", sections::root_label(raw)?)),
                None => None,
            };
            // A pages route off `/` stands only beside a section's root (§2c);
            // `sections::validate` refuses it anywhere else.
            let shared_pages = route.surface == Surface::Pages
                && root.is_none()
                && addressing.routes.iter().any(|r| {
                    normalize_host(&r.host) == host && r.surface == Surface::Pages && r.root.is_some()
                });
            let path = if shared_pages {
                normalize_route_path(&route.path, Surface::Static)?
            } else {
                normalize_route_path(&route.path, route.surface)?
            };
            if routes.iter().any(|r: &AddressRoute| r.host == host && r.path == path) {
                return Err(PlatformError::new(
                    "ADDRESSING_ROUTE_DUPLICATE",
                    format!("two routes at `{host}{path}`"),
                ));
            }
            routes.push(AddressRoute { host, path, surface: route.surface, root });
        }
        // A file host answers exposed files at its root and nothing else: no
        // second route beside it, and never the dev host (`addressing.md` §2b).
        for route in routes.iter().filter(|r| r.surface == Surface::Files) {
            if route.host == dev_host || route.path != "/" || routes.iter().any(|r| r.host == route.host && r.surface != Surface::Files) {
                return Err(PlatformError::new(
                    "ADDRESSING_FILE_HOST",
                    format!("a files route makes `{}` a file host: it mounts at `/` on a named host of its own, with no other route", route.host),
                ));
            }
        }
        {
            let file_hosts: Vec<&str> = routes.iter().filter(|r| r.surface == Surface::Files).map(|r| r.host.as_str()).collect();
            sections::validate(&owner, &addressing.hosts, &routes, &file_hosts)?;
        }
        addressing.routes = routes;
        addressing.disabled.sort_by_key(|s| s.key());
        addressing.disabled.dedup();
        if let Some(table) = self.section_table(&owner, &project, &addressing) {
            if let Some((host, _)) = table.hosts.iter().find(|(_, h)| h.section.is_some() && h.serve_host) {
                return Err(PlatformError::new(
                    "ADDRESSING_ROOT",
                    format!("`{host}` serves a folder (Files → access); a section host cannot also be a `serve` host"),
                ));
            }
            check(&table)?;
        }

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

        // The section dev hosts are claimed under the index's own lock, in
        // the same hold that publishes the routing (§2c Dev host).
        let table = self.section_table(&owner, &project, &addressing);
        let mut index = self.index.write().expect("addressing index poisoned");
        if let Some(table) = &table {
            for host in table.section_dev_hosts.values() {
                if let Some((o, p)) = index.hosts.get(host)
                    && (o != &owner || p != &project)
                {
                    return Err(PlatformError::new(
                        "ADDRESSING_HOST_TAKEN",
                        format!("the section dev host `{host}` already answers for `{o}/{p}`"),
                    ));
                }
            }
        }
        index
            .hosts
            .retain(|_, (o, p)| !(o == &owner && p == &project));
        for host in &addressing.hosts {
            index.hosts.insert(host.clone(), (owner.clone(), project.clone()));
        }
        index.place_sections(&owner, &project, table);
        index.projects.insert((owner, project), addressing.clone());
        Ok(addressing)
    }

    /// Where a request for `host` + `path` goes, if the host is a project's.
    /// `None` means "not a project host — route as the platform".
    /// The project a host belongs to, whatever its routes: a dev host by its
    /// name, a named host by the index. `(owner, project, dev_host)`.
    pub fn project_of_host(&self, host: &str) -> Option<(String, String, bool)> {
        let host = normalize_host(host);
        if host.is_empty() {
            return None;
        }
        if let Some((o, p)) = Self::parse_dev_host(&host) {
            return Some((o, p, true));
        }
        let index = self.index.read().expect("addressing index poisoned");
        let (o, p) = index.hosts.get(&host)?.clone();
        Some((o, p, false))
    }

    pub fn resolve(&self, host: &str, path: &str) -> Option<Resolution> {
        let (owner, project, dev_host) = self.project_of_host(host)?;
        let host = normalize_host(host);
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
        // dedicated tiles host does not also serve the site at `/`, nor the
        // Studio's `/login`: a miss stays the project's.
        let miss = custom_hit.is_none() && !candidates.is_empty();
        let hit = match custom_hit {
            Some(hit) => Some(hit),
            None if miss => Some((Surface::Pages, "/".to_string())),
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
        // A file host answers whatever the `files` switch says: the switch
        // governs the `/_files/` mount on site hosts only.
        let file_host = !dev_host && addressing.file_hosts().contains(&host.as_str());
        if file_host {
            return Some(Resolution { owner, project, surface, rest, host, dev_host, file_host, miss });
        }
        if !miss && !addressing.is_enabled(surface) {
            return Some(Resolution {
                owner,
                project,
                surface,
                rest: String::new(), // the empty rest marks "disabled" for the gate
                host,
                dev_host,
                file_host,
                miss,
            });
        }
        Some(Resolution { owner, project, surface, rest, host, dev_host, file_host, miss })
    }

    /// Whether a platform-form request (`/files/{o}/{p}/…`) is for a surface
    /// the project switched off.
    pub fn platform_form_disabled(&self, path: &str) -> bool {
        let mut parts = path.trim_start_matches('/').splitn(4, '/');
        let (first, owner, project) = (parts.next(), parts.next(), parts.next());
        let (Some(first), Some(owner), Some(project)) = (first, owner, project) else {
            return false;
        };
        if first == "ws" && owner == "preview" {
            return false;
        }
        let surface = match first {
            "wh" => Surface::Pages,
            "ws" => Surface::Ws,
            "static" => Surface::Static,
            "ms" => Surface::Ms,
            "mcp" => Surface::Mcp,
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
        && host.parse::<std::net::IpAddr>().is_err()
        && !host
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .chars()
            .all(|c| c.is_ascii_digit())
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// Validate a custom project hostname before storing or probing it via DNS/HTTP.
pub fn validate_host(raw: &str) -> Result<String, PlatformError> {
    let host = normalize_host(raw);
    if host.is_empty() {
        return Err(PlatformError::new("ADDRESSING_HOST_INVALID", "host is required"));
    }
    if sections::is_platform_host(raw) {
        return Err(PlatformError::new(
            "ADDRESSING_HOST_RESERVED",
            format!("`{host}` is the platform's own address; a project cannot hold it"),
        ));
    }
    if !valid_host(&host) {
        return Err(PlatformError::new(
            "ADDRESSING_HOST_INVALID",
            format!("`{raw}` is not a public hostname (letters, digits, `-`, dots; no IP literal, port, or path)"),
        ));
    }
    if AddressingService::parse_dev_host(&host).is_some()
        || AddressingService::parse_dev_file_host(&host).is_some()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".cluster.local")
    {
        return Err(PlatformError::new(
            "ADDRESSING_HOST_RESERVED",
            format!("`{host}` is a reserved or internal hostname"),
        ));
    }
    Ok(host)
}

/// A route path: starts with `/`; a prefix ends with `/`; pages may only be
/// `/`.
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
    if surface != Surface::Pages && !path.ends_with('/') {
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

    /// A page learns where its rooms live from the address it was served on:
    /// the default mount or the host's own `ws` route on a project host, the
    /// platform form on the platform form, and nothing with `ws` off.
    #[test]
    fn a_page_room_base_follows_the_address_it_was_served_on() {
        let route = |host: &str, path: &str, surface: Surface, root: Option<&str>| AddressRoute {
            host: host.to_string(),
            path: path.to_string(),
            surface,
            root: root.map(str::to_string),
        };
        let mut on = ProjectAddressing { disabled: vec![Surface::Files, Surface::Ms, Surface::Mcp], ..Default::default() };
        assert_eq!(on.room_base("site-a", "demo", Some("demo.site-a.localhost:10610")).as_deref(), Some("/_ws/rooms/"));
        assert_eq!(on.room_base("site-a", "demo", None).as_deref(), Some("/ws/site-a/demo/rooms/"));

        on.hosts = vec!["play.example.com".into(), "www.example.com".into(), "a.example.net".into()];
        on.routes = vec![
            route("play.example.com", "/", Surface::Pages, None),
            route("play.example.com", "/live/", Surface::Ws, None),
            route("www.example.com", "/", Surface::Pages, None),
            route("www.example.com", "/api/", Surface::Pages, None),
            route("a.example.net", "/", Surface::Pages, Some("/site-a")),
        ];
        assert_eq!(on.room_base("site-a", "demo", Some("Play.Example.com")).as_deref(), Some("/live/rooms/"), "the host's own ws route");
        assert_eq!(
            on.room_base("site-a", "demo", Some("www.example.com")),
            None,
            "a host whose routes leave ws out has no rooms, and never names the owner"
        );
        assert_eq!(on.room_base("site-a", "demo", Some("a.example.net")).as_deref(), Some("/_ws/rooms/"), "a section host keeps the default mount");
        assert_eq!(on.room_base("site-a", "demo", Some("demo.site-a.localhost")).as_deref(), Some("/_ws/rooms/"));

        let off = ProjectAddressing::default();
        assert!(!off.is_enabled(Surface::Ws), "ws starts off");
        assert_eq!(off.room_base("site-a", "demo", Some("demo.site-a.localhost")), None);
        assert_eq!(off.room_base("site-a", "demo", None), None);
    }

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
        assert!(path_matches("/_mcp/shop", Surface::Mcp.default_path()));
        assert!(!path_matches("/_mcpx", Surface::Mcp.default_path()));
        assert_eq!(strip_prefix("/_mcp/shop", Surface::Mcp.default_path()), "/shop");
        assert_eq!(strip_prefix("/_files/a.jpg", "/_files/"), "/a.jpg");
        assert_eq!(strip_prefix("/_files", "/_files/"), "/");
        assert_eq!(strip_prefix("/service/tiles/1/2/3", "/service/"), "/service/".strip_prefix("/service").map(|_| "/tiles/1/2/3").unwrap());
    }

    #[test]
    fn a_pages_route_off_the_root_is_refused_and_prefixes_are_normalised() {
        assert!(normalize_route_path("/shop/", Surface::Pages).is_err());
        assert_eq!(normalize_route_path("service", Surface::Ms).unwrap(), "/service/");
        assert_eq!(normalize_route_path("", Surface::Files).unwrap(), "/");
        assert_eq!(normalize_route_path("/agents", Surface::Mcp).unwrap(), "/agents/");
    }

    #[test]
    fn a_fresh_project_has_tiles_and_files_off_on_its_hosts() {
        let fresh = ProjectAddressing::default();
        assert!(!fresh.is_enabled(Surface::Ws));
        assert!(!fresh.is_enabled(Surface::Ms));
        assert!(!fresh.is_enabled(Surface::Files));
        assert!(!fresh.is_enabled(Surface::Mcp));
        assert!(fresh.is_enabled(Surface::Pages));
        assert!(fresh.is_enabled(Surface::Static));
        let old: ProjectAddressing = serde_json::from_str(r#"{"disabled":["ms","fs","mcp"]}"#).unwrap();
        assert_eq!(old.disabled, vec![Surface::Ms, Surface::Mcp], "a word for a removed surface is dropped");
        let stored: ProjectAddressing = serde_json::from_str(r#"{"hosts":["a.example"]}"#).unwrap();
        assert_eq!(stored.disabled, fresh.disabled, "an old file without the key gets the defaults");
        let explicit: ProjectAddressing = serde_json::from_str(r#"{"disabled":[]}"#).unwrap();
        assert!(explicit.is_enabled(Surface::Ws), "an explicit empty list means everything on");
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
    fn the_dev_file_host_is_its_own_grammar() {
        assert_eq!(AddressingService::dev_file_host("o", "p"), "p.o.fs.localhost");
        assert_eq!(
            AddressingService::parse_dev_file_host("p.o.fs.localhost:10610"),
            Some(("o".to_string(), "p".to_string()))
        );
        assert_eq!(AddressingService::parse_dev_host("p.o.fs.localhost"), None);
        assert_eq!(AddressingService::parse_dev_file_host("p.o.localhost"), None);
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
            file_host: false,
            miss: false,
        };
        assert_eq!(r.platform_path(), None, "files has no platform form; the gateway answers it");
        let r = Resolution { surface: Surface::Pages, rest: "/".into(), ..r };
        assert_eq!(r.platform_path().as_deref(), Some("/wh/o/p"));
        let r = Resolution { surface: Surface::Pages, rest: "/book?x=1".into(), ..r };
        assert_eq!(r.platform_path().as_deref(), Some("/wh/o/p/book?x=1"));
    }

    #[test]
    fn the_platform_address_and_dev_file_hosts_are_refused_for_every_project() {
        let dir = tempfile::tempdir().expect("tempdir");
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: dir.path().to_path_buf(),
                ..crate::platform::model::PlatformConfig::default()
            },
        )
        .expect("platform");
        for host in ["127.0.0.1", "localhost", "[::1]", "default.superadmin.fs.localhost", "other.user.fs.localhost"] {
            let err = platform
                .addressing
                .write(
                    "superadmin",
                    "default",
                    ProjectAddressing { hosts: vec![host.to_string()], ..ProjectAddressing::default() },
                )
                .expect_err(host);
            assert_eq!(err.code, "ADDRESSING_HOST_RESERVED", "{host}: {}", err.message);
        }
        // Any other `.localhost` name is a squat on the dev namespace.
        for host in ["squat.localhost", "a.b.c.localhost", "site-b.default.superadmin.localhost", "other.superadmin.localhost"] {
            let err = platform
                .addressing
                .write(
                    "superadmin",
                    "default",
                    ProjectAddressing { hosts: vec![host.to_string()], ..ProjectAddressing::default() },
                )
                .expect_err(host);
            assert_eq!(err.code, "ADDRESSING_HOST_RESERVED", "{host}: {}", err.message);
        }
        // The project's own dev host is implicit: accepted and never stored.
        let saved = platform
            .addressing
            .write(
                "superadmin",
                "default",
                ProjectAddressing { hosts: vec!["default.superadmin.localhost".to_string()], ..ProjectAddressing::default() },
            )
            .expect("own dev host is implicit");
        assert!(saved.hosts.is_empty());
        // Normal project host still saves cleanly.
        let saved = platform
            .addressing
            .write(
                "superadmin",
                "default",
                ProjectAddressing { hosts: vec!["site-a.example.org".to_string()], ..ProjectAddressing::default() },
            )
            .expect("normal host saves");
        assert_eq!(saved.hosts, vec!["site-a.example.org".to_string()]);
    }

    #[test]
    fn anonymous_requests_do_not_create_project_layouts_for_missing_projects() {
        let dir = tempfile::tempdir().expect("tempdir");
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: dir.path().to_path_buf(),
                ..crate::platform::model::PlatformConfig::default()
            },
        )
        .expect("platform");
        let _ = platform.addressing.resolve("ghost-proj.ghost-owner.localhost", "/");
        let _ = platform.addressing.read("ghost-owner", "ghost-proj").expect("default addressing");
        // A project lives at `users/{owner}/{project}`.
        assert!(
            !dir.path().join("users/ghost-owner/ghost-proj").exists(),
            "missing project must not be created on disk by addressing lookups"
        );
        assert!(
            !dir.path().join("users/ghost-owner").exists(),
            "a missing project's owner folder must not appear either"
        );
        assert!(
            !platform
                .addressing
                .index
                .read()
                .expect("index")
                .projects
                .contains_key(&("ghost-owner".to_string(), "ghost-proj".to_string())),
            "missing project must not be cached in addressing index"
        );
        // A project that exists is read and indexed as before.
        platform.addressing.read("superadmin", "default").expect("existing project addressing");
        assert!(dir.path().join("users/superadmin/default").is_dir());
        assert!(
            platform
                .addressing
                .index
                .read()
                .expect("index")
                .projects
                .contains_key(&("superadmin".to_string(), "default".to_string())),
            "an existing project is cached in the addressing index"
        );
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
        "# Check: every host must reach Zebflow with its Host header intact.\n\
         #   curl -s -o /dev/null -w '%{{http_code}}\\n' https://{first}/_verify     → 204\n\
         #   Settings → Addressing → Verify then proves each host answers for this project\n\
         #   (a one-time probe; no response names the project)."
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
            assert!(text.contains("/_verify"), "{id} ends with its check");
            assert!(!text.contains("x-zebflow-project"), "{id}: no header names the project");
        }
        assert!(server_configs(&ProxyFacts { hosts: &[], ..facts }).is_empty());
    }

    #[test]
    fn addressing_check_validates_host_and_blocks_internal_dns_and_ip_literals() {
        for rejected in [
            "",
            "localhost",
            "127.0.0.1",
            "169.254.169.254",
            "203.0.113.10",
            "northside.superadmin.localhost",
            "northside.superadmin.fs.localhost",
            "redis.default.svc.cluster.local",
            "db.internal",
            "printer.local",
            "metadata.google.internal",
        ] {
            assert!(
                validate_host(rejected).is_err(),
                "expected '{rejected}' to be rejected by validate_host"
            );
        }
        assert_eq!(
            validate_host("Site-A.Example.com:443").expect("valid public host"),
            "site-a.example.com"
        );
    }
}
