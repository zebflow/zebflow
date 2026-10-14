//! Sections — one project, several sites (`docs/contracts/addressing.md` §2c).
//!
//! A project whose routes carry a `root` is a sectioned project. Everything
//! here runs only for one: a project without a root never reaches this
//! module, so its answers, code path and cost stay what §1–§2b say.
//!
//! The routing of a sectioned project is prepared once, whenever its
//! addressing enters the index ([`SectionTable::build`]): each host's routes
//! in match order, the root set, the section dev hosts and the hosts a
//! `public_execute` folder serves. A request then costs a host lookup, a few
//! prefix compares and one set lookup on the first segment ([`route`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{AddressRoute, ProjectAddressing, Surface, normalize_host, DEV_HOST_SUFFIX};
use crate::platform::error::PlatformError;
use crate::platform::model::slug_segment;

/// One route as a host answers it: where it mounts, what it serves, and the
/// root it maps onto (`None`: shared, or a rootless host).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRoute {
    /// Visitor-path prefix: `/`, or segments ending in `/` (`/api/`).
    pub path: String,
    pub surface: Surface,
    /// The section label (`site-a`), never the stored `/site-a`.
    pub root: Option<String>,
}

/// What one host of a sectioned project answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostTable {
    /// The host's section, for a section host and a section dev host.
    pub section: Option<String>,
    /// Longest prefix first; `/` last.
    pub routes: Vec<HostRoute>,
    /// The project's dev host or a section dev host: http, the listen port.
    pub dev_host: bool,
    /// A named file host: the gateway answers it, as today.
    pub file_host: bool,
    /// A `public_execute` folder names this host in `serve`.
    pub serve_host: bool,
}

/// The prepared routing of one sectioned project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionTable {
    pub owner: String,
    pub project: String,
    /// Section labels.
    pub roots: BTreeSet<String>,
    /// Every host of the project, its dev host and its section dev hosts.
    pub hosts: HashMap<String, HostTable>,
    /// Section dev hosts, label → host.
    pub section_dev_hosts: BTreeMap<String, String>,
    /// The project's switches, as saved with this routing.
    pub disabled: Vec<Surface>,
    pub api_on_hosts: bool,
}

/// A stored root (`/site-a`) as its label, or why it is refused.
pub fn root_label(raw: &str) -> Result<String, PlatformError> {
    let refuse = || {
        PlatformError::new(
            "ADDRESSING_ROOT",
            format!("root `{raw}` must be `/` and one DNS label: letters, digits and inner `-`, at most 63"),
        )
    };
    let label = raw.trim().strip_prefix('/').ok_or_else(refuse)?;
    let ok = !label.is_empty()
        && label.len() <= 63
        && label.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !label.starts_with('-')
        && !label.ends_with('-');
    if ok { Ok(label.to_string()) } else { Err(refuse()) }
}

/// The automatic dev host of one section: `<label>.<project>.<owner>.localhost`.
pub fn section_dev_host(owner: &str, project: &str, label: &str) -> String {
    format!("{label}.{}.{}{DEV_HOST_SUFFIX}", slug_segment(project), slug_segment(owner))
}

/// The section a host serves: the root of its `/` pages route.
fn host_section(routes: &[AddressRoute], host: &str) -> Option<String> {
    routes
        .iter()
        .find(|r| r.host == host && r.surface == Surface::Pages && r.path == "/")
        .and_then(|r| r.root.as_deref())
        .and_then(|root| root_label(root).ok())
}

/// The routes a host answers, in match order (§2c mapping). A section host
/// keeps the default mount of every surface it has no route for, its
/// `/_mcp/` with its root; any other host answers as §2 says.
fn host_routes(routes: &[AddressRoute], host: &str, section: Option<&str>) -> Vec<HostRoute> {
    let mut out: Vec<HostRoute> = routes
        .iter()
        .filter(|r| r.host == host)
        .map(|r| HostRoute {
            path: r.path.clone(),
            surface: r.surface,
            root: r.root.as_deref().and_then(|root| root_label(root).ok()),
        })
        .collect();
    if out.is_empty() || section.is_some() {
        for surface in Surface::ALL {
            if out.iter().any(|r| r.surface == surface) {
                continue;
            }
            let root = (surface == Surface::Mcp).then(|| section.map(str::to_string)).flatten();
            out.push(HostRoute { path: surface.default_path().to_string(), surface, root });
        }
    }
    out.sort_by(|a, b| b.path.len().cmp(&a.path.len()).then(a.path.cmp(&b.path)));
    out
}

impl SectionTable {
    /// The prepared routing, or `None` for a project without a root.
    pub fn build(owner: &str, project: &str, addressing: &ProjectAddressing, serve_hosts: &[String]) -> Option<SectionTable> {
        if !addressing.routes.iter().any(|r| r.root.is_some()) {
            return None;
        }
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let dev_host = super::AddressingService::dev_host(&owner, &project);
        let file_hosts = addressing.file_hosts();
        let mut hosts = HashMap::new();
        let mut roots = BTreeSet::new();
        let mut section_dev_hosts = BTreeMap::new();
        let mut all: Vec<String> = addressing.hosts.clone();
        all.push(dev_host.clone());
        for host in &all {
            let section = host_section(&addressing.routes, host);
            if let Some(label) = &section {
                roots.insert(label.clone());
            }
            let table = HostTable {
                routes: host_routes(&addressing.routes, host, section.as_deref()),
                dev_host: *host == dev_host,
                file_host: file_hosts.contains(&host.as_str()),
                serve_host: serve_hosts.iter().any(|h| h == host),
                section,
            };
            hosts.insert(host.clone(), table);
        }
        // A section dev host mirrors its section's hosts (aliases carry
        // identical routes, so the first one stands for all).
        for label in &roots {
            let Some(source) = all.iter().find(|h| hosts.get(*h).and_then(|t| t.section.as_deref()) == Some(label.as_str())) else {
                continue;
            };
            let mut table = hosts[source].clone();
            table.dev_host = true;
            table.file_host = false;
            table.serve_host = false;
            let name = section_dev_host(&owner, &project, label);
            section_dev_hosts.insert(label.clone(), name.clone());
            hosts.insert(name, table);
        }
        Some(SectionTable {
            owner,
            project,
            roots,
            hosts,
            section_dev_hosts,
            disabled: addressing.disabled.clone(),
            api_on_hosts: addressing.api_on_hosts,
        })
    }

    pub fn is_enabled(&self, surface: Surface) -> bool {
        !self.disabled.contains(&surface)
    }

    /// Whether an app path (or a published MCP route) lies under a root.
    pub fn claims(&self, segments: &[String]) -> Option<&str> {
        let first = segments.first()?;
        self.roots.get(first.as_str()).map(String::as_str)
    }
}

/// Every refusal §2c puts on saving a sectioned project's addressing, given
/// its routes already normalised. Nothing here runs for a rootless project.
pub fn validate(owner: &str, hosts: &[String], routes: &[AddressRoute], file_hosts: &[&str]) -> Result<(), PlatformError> {
    if !routes.iter().any(|r| r.root.is_some()) {
        return Ok(());
    }
    let refuse = |message: String| Err(PlatformError::new("ADDRESSING_ROOT", message));
    if let Some(host) = hosts.iter().find(|h| is_platform_host(h)) {
        return refuse(format!("`{host}` is the platform's own address; a project with sections cannot hold it"));
    }
    if slug_segment(owner) == "fs" {
        return refuse("an owner named `fs` cannot have sections: `<project>.fs.fs.localhost` would also be a dev file host".to_string());
    }
    let mut sections: BTreeMap<String, Vec<&AddressRoute>> = BTreeMap::new();
    for route in routes {
        let Some(root) = route.root.as_deref() else { continue };
        let label = root_label(root)?;
        let section = host_section(routes, &route.host);
        match route.surface {
            Surface::Pages if route.path == "/" => {
                sections.entry(label).or_default();
            }
            Surface::Mcp if section.as_deref() == Some(label.as_str()) => {}
            Surface::Mcp => {
                return refuse(format!("an mcp root stands only on a section host with the same root; `{}` has none or another", route.host));
            }
            _ => {
                return refuse(format!(
                    "a root stands only on a host's `/` pages route and its mcp route; `{}{}` ({}) cannot carry one",
                    route.host,
                    route.path,
                    route.surface.key()
                ));
            }
        }
    }
    for route in routes {
        let section = host_section(routes, &route.host);
        if route.surface == Surface::Pages && route.path != "/" {
            if section.is_none() {
                return refuse(format!("pages answer at `/` of a host; `{}{}` is allowed only on a section host", route.host, route.path));
            }
            if route.path.starts_with(super::RESERVED_PREFIX) {
                return refuse(format!("`{}{}`: `/_` is the platform's on a project host", route.host, route.path));
            }
        }
        if route.path.contains('%') {
            return refuse(format!("`{}{}`: a route path of a sectioned project holds no `%`", route.host, route.path));
        }
        if let Some(label) = &section
            && route.surface == Surface::Pages
            && route.path != "/"
            && route.path.trim_matches('/').split('/').next() == Some(label.as_str())
        {
            return refuse(format!("`{}{}` lies under its own root", route.host, route.path));
        }
    }
    // A root is never the first segment of an unrooted pages route.
    for route in routes.iter().filter(|r| r.surface == Surface::Pages && r.root.is_none() && r.path != "/") {
        let first = route.path.trim_matches('/').split('/').next().unwrap_or_default();
        if sections.contains_key(first) {
            return refuse(format!("`{}{}` is a shared route under the root `/{first}`", route.host, route.path));
        }
    }
    // Hosts sharing a root (aliases) carry identical routes.
    let mut by_label: BTreeMap<String, Vec<(String, Vec<HostRoute>)>> = BTreeMap::new();
    let mut hosts: Vec<&str> = routes.iter().map(|r| r.host.as_str()).collect();
    hosts.sort();
    hosts.dedup();
    for host in hosts {
        let Some(label) = host_section(routes, host) else { continue };
        if file_hosts.contains(&host) {
            return refuse(format!("`{host}` is a file host; it cannot also be a section host"));
        }
        by_label.entry(label.clone()).or_default().push((host.to_string(), host_routes(routes, host, Some(&label))));
    }
    for (label, hosts) in &by_label {
        if let Some((first, first_routes)) = hosts.first()
            && let Some((other, _)) = hosts.iter().find(|(_, r)| r != first_routes)
        {
            return refuse(format!("`{first}` and `{other}` share the root `/{label}` but not their routes; aliases carry identical routes"));
        }
    }
    Ok(())
}

/// The platform address (§2c Claim): the configured base URL's host, and
/// always the loopback names. Any port: a tunnel names its own.
pub fn is_platform_host(raw: &str) -> bool {
    let raw = raw.trim();
    let host = match raw.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or_default().to_ascii_lowercase(),
        None => normalize_host(raw),
    };
    if matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return true;
    }
    std::env::var("ZEBFLOW_PLATFORM_BASE_URL")
        .ok()
        .and_then(|base| reqwest::Url::parse(base.trim()).ok())
        .and_then(|url| url.host_str().map(|h| h.trim_matches(['[', ']']).to_ascii_lowercase()))
        .is_some_and(|base| base == host)
}

// ── One reading of the path ─────────────────────────────────────────────────

/// The path decoded once with empty segments dropped, or `None` when it must
/// answer 400: an encoded `/`, `\` or NUL, a `.` or `..` segment, or bytes
/// that are not UTF-8. The gate and the matchers read it alike: the webhook
/// matcher decodes its tail once and drops empty segments.
pub fn read_path(raw: &str) -> Option<Vec<String>> {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("%2f") || lower.contains("%5c") || lower.contains("%00") || raw.contains('\\') {
        return None;
    }
    let mut segments = Vec::new();
    for part in raw.split('/') {
        if part.is_empty() {
            continue;
        }
        let decoded = percent_decode(part)?;
        if decoded == "." || decoded == ".." || decoded.contains('\0') || decoded.contains('\\') || decoded.contains('/') {
            return None;
        }
        segments.push(decoded);
    }
    Some(segments)
}

fn percent_decode(part: &str) -> Option<String> {
    let bytes = part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The one serializer: one leading `/`, each segment escaped, never `//`,
/// `/\` or a dot segment (segments come from [`read_path`] or a written
/// path, and a dot segment is refused).
pub fn serialize(segments: &[String]) -> Option<String> {
    let mut out = String::new();
    for segment in segments {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        out.push('/');
        for byte in segment.bytes() {
            let keep = byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte);
            if keep {
                out.push(byte as char);
            } else {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    Some(out)
}

/// A written app path (`/site-a/login`) as segments; `None` for anything but
/// a root-relative path (an absolute URL is the author's and passes as is).
pub fn written_segments(path: &str) -> Option<Vec<String>> {
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    read_path(path)
}

// ── Routing one request ─────────────────────────────────────────────────────

/// Where a request on a host of a sectioned project goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    /// 400 — the path has no single reading.
    BadPath,
    /// The project's own 404: a claimed path, or no route of the host.
    NotFound,
    /// A surface switched off for the project.
    Disabled(Surface),
    /// Served by `surface` at the path under it (`app` for pages, the
    /// published route for mcp, the rest for every other surface).
    Surface {
        surface: Surface,
        /// The visitor-path prefix the request came through (`/_mcp`).
        mount: String,
        /// For pages: the app path's segments; for mcp: the route's; for the
        /// others: the path under the mount.
        segments: Vec<String>,
        /// The root this arrival carries.
        root: Option<String>,
    },
}

/// Match a path already read ([`read_path`]) against one host's routes.
pub fn route(table: &SectionTable, host: &HostTable, segments: &[String], enabled: impl Fn(Surface) -> bool) -> Routed {
    for candidate in &host.routes {
        let prefix: Vec<&str> = candidate.path.split('/').filter(|s| !s.is_empty()).collect();
        if prefix.len() > segments.len() || prefix.iter().zip(segments).any(|(p, s)| *p != s.as_str()) {
            continue;
        }
        if !enabled(candidate.surface) {
            return Routed::Disabled(candidate.surface);
        }
        let mount = if prefix.is_empty() { String::new() } else { format!("/{}", prefix.join("/")) };
        let rest = &segments[prefix.len()..];
        let (mapped, claim_from): (Vec<String>, &[String]) = match candidate.surface {
            // Pages never strip: the route path stays part of the app path.
            Surface::Pages => (segments.to_vec(), segments),
            _ => (rest.to_vec(), rest),
        };
        let applies_root = matches!(candidate.surface, Surface::Pages | Surface::Mcp);
        if !applies_root {
            return Routed::Surface { surface: candidate.surface, mount, segments: mapped, root: None };
        }
        // Within mcp, the protocol suffix is split off before any rule.
        let route_part: Vec<String> = match candidate.surface {
            Surface::Mcp => claim_from.iter().take_while(|s| *s != "_oauth").cloned().collect(),
            _ => claim_from.to_vec(),
        };
        return match &candidate.root {
            Some(label) => {
                // A segment right after a root that starts with `_` matches nothing.
                if route_part.first().is_some_and(|s| s.starts_with('_')) {
                    return Routed::NotFound;
                }
                let mut with_root = vec![label.clone()];
                with_root.extend(mapped);
                Routed::Surface { surface: candidate.surface, mount, segments: with_root, root: Some(label.clone()) }
            }
            None => {
                if table.claims(&route_part).is_some() {
                    return Routed::NotFound;
                }
                Routed::Surface { surface: candidate.surface, mount, segments: mapped, root: None }
            }
        };
    }
    Routed::NotFound
}

/// The visitor path that reaches the pages app path `target` on `host`, if
/// one does and routes back to it (§2c Visitor form): under the host's root
/// the root is removed, under a shared route it stays.
pub fn pages_visitor_path(table: &SectionTable, host: &HostTable, target: &[String]) -> Option<Vec<String>> {
    let candidate: Vec<String> = match &host.section {
        Some(label) if target.first().map(String::as_str) == Some(label.as_str()) => target[1..].to_vec(),
        _ => target.to_vec(),
    };
    match route(table, host, &candidate, |_| true) {
        Routed::Surface { surface: Surface::Pages, segments, .. } if segments == target => Some(candidate),
        _ => None,
    }
}

/// The URL to emit on `host` for a written pages path (`/site-a/login`,
/// query allowed): the visitor path that routes back to it, or `None` —
/// nothing is emitted. An absolute URL is the author's and passes as written.
pub fn pages_url(table: &SectionTable, host: &str, written: &str) -> Option<String> {
    if written.starts_with("http://") || written.starts_with("https://") {
        return Some(written.to_string());
    }
    let (path, query) = match written.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (written, None),
    };
    let target = written_segments(path)?;
    let visitor = pages_visitor_path(table, table.hosts.get(host)?, &target)?;
    let mut url = serialize(&visitor)?;
    if let Some(query) = query {
        url.push('?');
        url.push_str(query);
    }
    Some(url)
}

/// The visitor path of a published MCP route on `host` through `mount`
/// with `root` (§2c Visitor form: the mount restored, the root removed).
pub fn mcp_visitor_route(route: &[String], root: Option<&str>) -> Option<Vec<String>> {
    match root {
        Some(label) if route.first().map(String::as_str) == Some(label) => Some(route[1..].to_vec()),
        Some(_) => None,
        None => Some(route.to_vec()),
    }
}

/// The project's registered routes under a root that its section hosts
/// cannot reach as written (§2c Saving refuses): `pages` are webhook routes
/// and `--login` pages, `mcp` published MCP routes, each written whole.
pub fn check_routes(table: &SectionTable, pages: &[String], mcp: &[String]) -> Result<(), PlatformError> {
    let section_host = |label: &str| {
        table.section_dev_hosts.get(label).and_then(|host| table.hosts.get(host))
    };
    for (surface, written) in pages.iter().map(|p| (Surface::Pages, p)).chain(mcp.iter().map(|p| (Surface::Mcp, p))) {
        let segments: Vec<String> = written.split('/').filter(|s| !s.is_empty()).map(str::to_string).collect();
        let Some(label) = table.claims(&segments).map(str::to_string) else { continue };
        if segments.get(1).is_some_and(|s| s.starts_with('_')) {
            return Err(PlatformError::new(
                "PIPELINE_ROUTE_RESERVED",
                format!("`{written}`: a segment right after the root `/{label}` may not start with `_` — the platform's on the section's host"),
            ));
        }
        let Some(host) = section_host(&label) else { continue };
        let reaches = match surface {
            Surface::Pages => pages_visitor_path(table, host, &segments).is_some(),
            _ => host.routes.iter().filter(|r| r.surface == Surface::Mcp && r.root.as_deref() == Some(label.as_str())).any(|r| {
                let mut visitor: Vec<String> = r.path.split('/').filter(|s| !s.is_empty()).map(str::to_string).collect();
                visitor.extend(segments[1..].iter().cloned());
                matches!(route(table, host, &visitor, |_| true), Routed::Surface { surface: Surface::Mcp, segments: ref got, .. } if *got == segments)
            }),
        };
        if !reaches {
            return Err(PlatformError::new(
                "PIPELINE_ROUTE_SHADOWED",
                format!("`{written}` lies under the root `/{label}`, but its section's host sends that address elsewhere (another route or a platform path)"),
            ));
        }
    }
    Ok(())
}

/// The hosts a project's ACL serves, read from its `serve` origins.
pub fn serve_hosts_of(store_dir: &std::path::Path) -> Vec<String> {
    let Ok(manifest) = crate::platform::services::zebfs_acl::read(store_dir) else {
        return Vec::new();
    };
    let mut hosts: Vec<String> = manifest
        .rules
        .values()
        .flat_map(|rule| rule.serve.iter())
        .map(|origin| normalize_host(crate::zebfs::acl::serve_origin_host(origin)))
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts
}

#[cfg(test)]
mod tests;
