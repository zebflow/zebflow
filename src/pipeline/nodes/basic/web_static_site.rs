//! Shared static-site artifact helpers for static-producing web nodes.
//!
//! This layer does not publish content. It only organizes generated artifacts
//! into one coherent site tree and records a manifest so multiple generators can
//! cooperate on the same static root.

use std::collections::{BTreeSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::pipeline::PipelineError;
use crate::platform::web::embedded::{PLATFORM_LIBRARY_ASSETS, platform_public_asset};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticTemplateGroup {
    pub template: String,
    pub asset_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticPageRecord {
    pub path: String,
    pub route: String,
    pub template: String,
    pub asset_group: String,
    pub generator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticAssetRecord {
    pub path: String,
    pub source_url: String,
    pub kind: String,
    pub content_hash: String,
    pub asset_group: String,
}

#[derive(Debug, Clone)]
pub struct StaticAssetSources<'a> {
    pub owner: Option<&'a str>,
    pub project: Option<&'a str>,
    pub project_asset_root_abs: Option<&'a Path>,
}

#[derive(Debug, Clone)]
pub struct LocalizedStaticHtml {
    pub html: String,
    pub assets: Vec<StaticAssetRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticSiteManifest {
    pub version: u32,
    pub site_root: String,
    pub deploy_base_url: Option<String>,
    pub deploy_base_path: String,
    pub updated_at_unix: u64,
    pub generators: Vec<String>,
    pub templates: Vec<StaticTemplateGroup>,
    pub pages: Vec<StaticPageRecord>,
    pub assets: Vec<StaticAssetRecord>,
}

impl StaticSiteManifest {
    fn new(site_root: String, deploy_base_url: Option<String>, deploy_base_path: String) -> Self {
        Self {
            version: 1,
            site_root,
            deploy_base_url,
            deploy_base_path,
            updated_at_unix: now_unix(),
            generators: Vec::new(),
            templates: Vec::new(),
            pages: Vec::new(),
            assets: Vec::new(),
        }
    }
}

pub fn normalize_site_root_rel_path(raw: &str) -> Result<String, PipelineError> {
    let mut parts = Vec::new();
    for part in raw.trim().replace('\\', "/").split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "WEB_STATIC_SITE_ROOT",
                "site_root must stay inside the project files directory",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_SITE_ROOT",
            "site_root must not be empty",
        ));
    }
    Ok(parts.join("/"))
}

pub fn normalize_page_output_path(raw: &str) -> Result<String, PipelineError> {
    let mut parts = Vec::new();
    for part in raw.trim().replace('\\', "/").split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "WEB_STATIC_OUTPUT_PATH",
                "output_path must stay inside the configured static site root",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_OUTPUT_PATH",
            "output_path must not be empty",
        ));
    }
    Ok(parts.join("/"))
}

pub fn page_rel_path_from_site_root(
    site_root_rel: &str,
    page_output_path: &str,
) -> Result<String, PipelineError> {
    let page_output_path = normalize_page_output_path(page_output_path)?;
    Ok(format!(
        "{}/{}",
        site_root_rel.trim_end_matches('/'),
        page_output_path
    ))
}

pub fn site_manifest_rel_path(site_root_rel: &str) -> String {
    format!(
        "{}/.zebflow-static-site.json",
        site_root_rel.trim_end_matches('/')
    )
}

pub fn normalize_deploy_base_url(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
}

pub fn normalize_deploy_base_path(
    raw: Option<&str>,
    default_base_path: &str,
) -> Result<String, PipelineError> {
    let source = raw.unwrap_or(default_base_path);
    let mut parts = Vec::new();
    for part in source
        .trim()
        .trim_matches('/')
        .replace('\\', "/")
        .split('/')
    {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }
        if trimmed == ".." || trimmed.contains('\0') {
            return Err(PipelineError::new(
                "WEB_STATIC_DEPLOY_BASE_PATH",
                "deploy_base_path must stay inside the generated site URL space",
            ));
        }
        parts.push(trimmed.to_string());
    }
    if parts.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", parts.join("/")))
    }
}

pub fn route_path_for_output_path(
    deploy_base_path: &str,
    page_output_path: &str,
) -> Result<String, PipelineError> {
    let page_output_path = normalize_page_output_path(page_output_path)?;
    let base = if deploy_base_path == "/" {
        String::new()
    } else {
        normalize_deploy_base_path(Some(deploy_base_path), "/")?
            .trim_end_matches('/')
            .to_string()
    };

    let route_suffix = if page_output_path == "index.html" {
        "/".to_string()
    } else if let Some(prefix) = page_output_path.strip_suffix("/index.html") {
        format!("/{prefix}/")
    } else {
        format!("/{}", page_output_path)
    };

    if base.is_empty() {
        Ok(route_suffix)
    } else if route_suffix == "/" {
        Ok(format!("{base}/"))
    } else {
        Ok(format!("{base}{route_suffix}"))
    }
}

pub fn absolute_deploy_url(base_url: Option<&str>, route_path: &str) -> Option<String> {
    normalize_deploy_base_url(base_url).map(|base| format!("{base}{route_path}"))
}

pub fn localize_static_html_assets(
    site_root_abs: &Path,
    page_output_path: &str,
    html: &str,
    asset_sources: StaticAssetSources<'_>,
    asset_group: &str,
) -> Result<LocalizedStaticHtml, PipelineError> {
    let refs = collect_static_asset_refs(html, asset_sources.owner, asset_sources.project);
    if refs.is_empty() {
        return Ok(LocalizedStaticHtml {
            html: html.to_string(),
            assets: Vec::new(),
        });
    }

    let page_output_path = normalize_page_output_path(page_output_path)?;
    let mut rewritten = html.to_string();
    let mut asset_records = Vec::new();
    let mut written = BTreeSet::new();
    for asset_url in refs {
        let origin = AssetOrigin::from_url(&asset_url, asset_sources.owner, asset_sources.project)
            .ok_or_else(|| {
                PipelineError::new(
                    "WEB_STATIC_SITE_ASSET_URL",
                    format!("unsupported static asset reference '{asset_url}'"),
                )
            })?;
        let local_rel = origin.local_rel_path();
        materialize_asset(
            site_root_abs,
            &origin,
            &asset_sources,
            asset_group,
            &mut written,
            &mut asset_records,
        )?;
        let replacement = relative_href(&page_output_path, &local_rel);
        rewritten = rewritten.replace(&asset_url, &replacement);
    }

    asset_records.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(LocalizedStaticHtml {
        html: rewritten,
        assets: asset_records,
    })
}

pub fn asset_group_id(template_rel_path: &str, template_markup: &str) -> String {
    let mut hasher = DefaultHasher::new();
    template_rel_path.hash(&mut hasher);
    template_markup.hash(&mut hasher);
    format!("tpl-{:016x}", hasher.finish())
}

pub fn update_site_manifest(
    manifest_abs_path: &Path,
    site_root_rel: &str,
    deploy_base_url: Option<&str>,
    deploy_base_path: &str,
    generator_kind: &str,
    template_rel_path: &str,
    asset_group: &str,
    pages: &[StaticPageRecord],
    assets: &[StaticAssetRecord],
    prune_full_group: bool,
) -> Result<StaticSiteManifest, PipelineError> {
    let deploy_base_url = normalize_deploy_base_url(deploy_base_url);
    let deploy_base_path = normalize_deploy_base_path(Some(deploy_base_path), "/")?;
    let mut manifest = if manifest_abs_path.is_file() {
        let raw = std::fs::read_to_string(manifest_abs_path).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_SITE_MANIFEST_READ",
                format!("failed reading '{}': {err}", manifest_abs_path.display()),
            )
        })?;
        serde_json::from_str::<StaticSiteManifest>(&raw).unwrap_or_else(|_| {
            StaticSiteManifest::new(
                site_root_rel.to_string(),
                deploy_base_url.clone(),
                deploy_base_path.clone(),
            )
        })
    } else {
        StaticSiteManifest::new(
            site_root_rel.to_string(),
            deploy_base_url.clone(),
            deploy_base_path.clone(),
        )
    };

    manifest.site_root = site_root_rel.to_string();
    manifest.deploy_base_url = deploy_base_url;
    manifest.deploy_base_path = deploy_base_path;
    manifest.updated_at_unix = now_unix();

    if !manifest
        .generators
        .iter()
        .any(|value| value == generator_kind)
    {
        manifest.generators.push(generator_kind.to_string());
        manifest.generators.sort();
    }

    let template_group = StaticTemplateGroup {
        template: template_rel_path.to_string(),
        asset_group: asset_group.to_string(),
    };
    if let Some(existing) = manifest
        .templates
        .iter_mut()
        .find(|item| item.template == template_group.template)
    {
        *existing = template_group;
    } else {
        manifest.templates.push(template_group);
        manifest
            .templates
            .sort_by(|a, b| a.template.cmp(&b.template));
    }

    if prune_full_group {
        let incoming_paths = pages
            .iter()
            .map(|page| page.path.as_str())
            .collect::<BTreeSet<_>>();
        manifest.pages.retain(|page| {
            page.asset_group != asset_group || incoming_paths.contains(page.path.as_str())
        });
    }

    for page in pages {
        if let Some(existing) = manifest
            .pages
            .iter_mut()
            .find(|item| item.path == page.path)
        {
            *existing = page.clone();
        } else {
            manifest.pages.push(page.clone());
        }
    }
    manifest.pages.sort_by(|a, b| a.path.cmp(&b.path));

    if prune_full_group {
        let incoming_asset_paths = assets
            .iter()
            .map(|asset| asset.path.as_str())
            .collect::<BTreeSet<_>>();
        let stale_asset_paths = manifest
            .assets
            .iter()
            .filter(|asset| {
                asset.asset_group == asset_group
                    && !incoming_asset_paths.contains(asset.path.as_str())
            })
            .map(|asset| asset.path.clone())
            .collect::<Vec<_>>();
        let site_root_abs = manifest_abs_path.parent().ok_or_else(|| {
            PipelineError::new(
                "WEB_STATIC_SITE_MANIFEST_PARENT",
                format!(
                    "manifest path '{}' has no parent directory",
                    manifest_abs_path.display()
                ),
            )
        })?;
        for stale_path in &stale_asset_paths {
            let stale_abs = site_root_abs.join(stale_path);
            if stale_abs.is_file() {
                std::fs::remove_file(&stale_abs).map_err(|err| {
                    PipelineError::new(
                        "WEB_STATIC_SITE_ASSET_DELETE",
                        format!(
                            "failed deleting stale asset '{}': {err}",
                            stale_abs.display()
                        ),
                    )
                })?;
            }
        }
        manifest.assets.retain(|asset| {
            asset.asset_group != asset_group || incoming_asset_paths.contains(asset.path.as_str())
        });
    }

    for asset in assets {
        if let Some(existing) = manifest
            .assets
            .iter_mut()
            .find(|item| item.path == asset.path)
        {
            *existing = asset.clone();
        } else {
            manifest.assets.push(asset.clone());
        }
    }
    manifest.assets.sort_by(|a, b| a.path.cmp(&b.path));

    if let Some(parent) = manifest_abs_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_SITE_MANIFEST_MKDIR",
                format!("failed creating '{}': {err}", parent.display()),
            )
        })?;
    }
    let payload = serde_json::to_vec_pretty(&manifest).map_err(|err| {
        PipelineError::new(
            "WEB_STATIC_SITE_MANIFEST_SERIALIZE",
            format!("failed serializing site manifest: {err}"),
        )
    })?;
    std::fs::write(manifest_abs_path, payload).map_err(|err| {
        PipelineError::new(
            "WEB_STATIC_SITE_MANIFEST_WRITE",
            format!("failed writing '{}': {err}", manifest_abs_path.display()),
        )
    })?;

    Ok(manifest)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
enum AssetOrigin {
    Library(String),
    Platform(String),
    Branding(String),
    Project(String),
}

impl AssetOrigin {
    fn from_url(url: &str, owner: Option<&str>, project: Option<&str>) -> Option<Self> {
        if let Some(path) = url.strip_prefix("/assets/libraries/") {
            return Some(Self::Library(path.to_string()));
        }
        if let Some(path) = url.strip_prefix("/assets/platform/") {
            return Some(Self::Platform(path.to_string()));
        }
        if let Some(path) = url.strip_prefix("/assets/branding/") {
            return Some(Self::Branding(path.to_string()));
        }
        if let (Some(owner), Some(project)) = (owner, project) {
            let prefix = format!("/assets/{owner}/{project}/");
            if let Some(path) = url.strip_prefix(&prefix) {
                return Some(Self::Project(path.to_string()));
            }
        }
        None
    }

    fn local_rel_path(&self) -> String {
        match self {
            Self::Library(path) => format!("_assets/libraries/{path}"),
            Self::Platform(path) => format!("_assets/platform/{path}"),
            Self::Branding(path) => format!("_assets/branding/{path}"),
            Self::Project(path) => format!("_assets/project/{path}"),
        }
    }

    fn source_url(&self, owner: Option<&str>, project: Option<&str>) -> String {
        match self {
            Self::Library(path) => format!("/assets/libraries/{path}"),
            Self::Platform(path) => format!("/assets/platform/{path}"),
            Self::Branding(path) => format!("/assets/branding/{path}"),
            Self::Project(path) => {
                let owner = owner.unwrap_or("owner");
                let project = project.unwrap_or("project");
                format!("/assets/{owner}/{project}/{path}")
            }
        }
    }

    fn kind(&self) -> String {
        asset_kind_for_path(match self {
            Self::Library(path)
            | Self::Platform(path)
            | Self::Branding(path)
            | Self::Project(path) => path,
        })
    }
}

fn collect_static_asset_refs(
    input: &str,
    owner: Option<&str>,
    project: Option<&str>,
) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let mut prefixes = vec![
        "/assets/libraries/".to_string(),
        "/assets/platform/".to_string(),
        "/assets/branding/".to_string(),
    ];
    if let (Some(owner), Some(project)) = (owner, project) {
        prefixes.push(format!("/assets/{owner}/{project}/"));
    }
    for prefix in prefixes {
        let mut cursor = 0usize;
        while let Some(offset) = input[cursor..].find(&prefix) {
            let start = cursor + offset;
            let tail = &input[start..];
            let end = tail
                .find(|ch: char| {
                    ch.is_ascii_whitespace()
                        || matches!(ch, '"' | '\'' | '<' | '>' | ')' | '(' | '\\' | ',')
                })
                .unwrap_or(tail.len());
            let candidate = &tail[..end];
            if candidate.len() > prefix.len() {
                refs.insert(candidate.to_string());
            }
            cursor = start + prefix.len();
        }
    }
    refs
}

fn materialize_asset(
    site_root_abs: &Path,
    origin: &AssetOrigin,
    asset_sources: &StaticAssetSources<'_>,
    asset_group: &str,
    written: &mut BTreeSet<String>,
    asset_records: &mut Vec<StaticAssetRecord>,
) -> Result<(), PipelineError> {
    let local_rel = origin.local_rel_path();
    if !written.insert(local_rel.clone()) {
        return Ok(());
    }

    let raw_bytes = match origin {
        AssetOrigin::Library(path) => materialize_runtime_family(path)?,
        AssetOrigin::Platform(path) => platform_public_asset(&format!("platform/{path}"))
            .map(|bytes| bytes.to_vec())
            .ok_or_else(|| {
                PipelineError::new(
                    "WEB_STATIC_SITE_ASSET_MISSING",
                    format!("embedded platform asset 'platform/{path}' was not found"),
                )
            })?,
        AssetOrigin::Branding(path) => platform_public_asset(&format!("branding/{path}"))
            .map(|bytes| bytes.to_vec())
            .ok_or_else(|| {
                PipelineError::new(
                    "WEB_STATIC_SITE_ASSET_MISSING",
                    format!("embedded branding asset 'branding/{path}' was not found"),
                )
            })?,
        AssetOrigin::Project(path) => {
            let root = asset_sources.project_asset_root_abs.ok_or_else(|| {
                PipelineError::new(
                    "WEB_STATIC_SITE_PROJECT_ASSETS",
                    "project asset root is required to localize /assets/{owner}/{project}/ references",
                )
            })?;
            let abs = root.join(path);
            if !abs.starts_with(root) || !abs.is_file() {
                return Err(PipelineError::new(
                    "WEB_STATIC_SITE_PROJECT_ASSET_MISSING",
                    format!("project asset '{}' was not found", abs.display()),
                ));
            }
            std::fs::read(&abs).map_err(|err| {
                PipelineError::new(
                    "WEB_STATIC_SITE_PROJECT_ASSET_READ",
                    format!("failed reading '{}': {err}", abs.display()),
                )
            })?
        }
    };

    let local_abs = site_root_abs.join(&local_rel);
    if let Some(parent) = local_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_SITE_ASSET_MKDIR",
                format!("failed creating '{}': {err}", parent.display()),
            )
        })?;
    }

    let final_bytes = if local_rel.ends_with(".css") {
        localize_css_asset(
            String::from_utf8(raw_bytes).map_err(|err| {
                PipelineError::new(
                    "WEB_STATIC_SITE_ASSET_UTF8",
                    format!("failed decoding CSS asset '{}': {err}", local_rel),
                )
            })?,
            site_root_abs,
            &local_rel,
            origin,
            asset_sources,
            asset_group,
            written,
            asset_records,
        )?
        .into_bytes()
    } else {
        raw_bytes
    };

    std::fs::write(&local_abs, &final_bytes).map_err(|err| {
        PipelineError::new(
            "WEB_STATIC_SITE_ASSET_WRITE",
            format!("failed writing '{}': {err}", local_abs.display()),
        )
    })?;

    asset_records.push(StaticAssetRecord {
        path: local_rel,
        source_url: origin.source_url(asset_sources.owner, asset_sources.project),
        kind: origin.kind(),
        content_hash: content_hash(&final_bytes),
        asset_group: asset_group.to_string(),
    });

    if let AssetOrigin::Library(path) = origin {
        materialize_library_runtime_family_siblings(
            site_root_abs,
            path,
            asset_sources,
            asset_group,
            written,
            asset_records,
        )?;
    }

    Ok(())
}

fn materialize_library_runtime_family_siblings(
    site_root_abs: &Path,
    embedded_rel: &str,
    asset_sources: &StaticAssetSources<'_>,
    asset_group: &str,
    written: &mut BTreeSet<String>,
    asset_records: &mut Vec<StaticAssetRecord>,
) -> Result<(), PipelineError> {
    let runtime_dir = Path::new(embedded_rel)
        .parent()
        .ok_or_else(|| {
            PipelineError::new(
                "WEB_STATIC_SITE_ASSET_PATH",
                format!("asset path '{embedded_rel}' has no parent runtime directory"),
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");
    let runtime_prefix = format!("{runtime_dir}/");
    for asset in PLATFORM_LIBRARY_ASSETS {
        if asset.path == embedded_rel || !asset.path.starts_with(&runtime_prefix) {
            continue;
        }
        let sibling = AssetOrigin::Library(asset.path.to_string());
        let sibling_local_rel = sibling.local_rel_path();
        if !written.insert(sibling_local_rel.clone()) {
            continue;
        }
        let sibling_abs = site_root_abs.join(&sibling_local_rel);
        if let Some(parent) = sibling_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                PipelineError::new(
                    "WEB_STATIC_SITE_ASSET_MKDIR",
                    format!("failed creating '{}': {err}", parent.display()),
                )
            })?;
        }
        let final_bytes = if sibling_local_rel.ends_with(".css") {
            localize_css_asset(
                String::from_utf8(asset.bytes.to_vec()).map_err(|err| {
                    PipelineError::new(
                        "WEB_STATIC_SITE_ASSET_UTF8",
                        format!("failed decoding CSS asset '{}': {err}", sibling_local_rel),
                    )
                })?,
                site_root_abs,
                &sibling_local_rel,
                &sibling,
                asset_sources,
                asset_group,
                written,
                asset_records,
            )?
            .into_bytes()
        } else {
            asset.bytes.to_vec()
        };
        std::fs::write(&sibling_abs, &final_bytes).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_SITE_ASSET_WRITE",
                format!("failed writing '{}': {err}", sibling_abs.display()),
            )
        })?;
        asset_records.push(StaticAssetRecord {
            path: sibling_local_rel,
            source_url: sibling.source_url(asset_sources.owner, asset_sources.project),
            kind: sibling.kind(),
            content_hash: content_hash(&final_bytes),
            asset_group: asset_group.to_string(),
        });
    }
    Ok(())
}

fn localize_css_asset(
    css: String,
    site_root_abs: &Path,
    css_local_rel: &str,
    origin: &AssetOrigin,
    asset_sources: &StaticAssetSources<'_>,
    asset_group: &str,
    written: &mut BTreeSet<String>,
    asset_records: &mut Vec<StaticAssetRecord>,
) -> Result<String, PipelineError> {
    let url_refs = collect_css_url_refs(&css);
    let import_refs = collect_css_import_refs(&css);
    if url_refs.is_empty() && import_refs.is_empty() {
        return Ok(css);
    }

    let mut rewritten = css;
    for css_ref in import_refs {
        let Some(target_origin) = resolve_css_asset_reference(origin, &css_ref, asset_sources)
        else {
            continue;
        };
        let target_local_rel = target_origin.local_rel_path();
        materialize_asset(
            site_root_abs,
            &target_origin,
            asset_sources,
            asset_group,
            written,
            asset_records,
        )?;
        let replacement = relative_href(css_local_rel, &target_local_rel);
        rewritten = rewritten.replace(&css_ref, &replacement);
    }
    for css_ref in url_refs {
        let Some(target_origin) = resolve_css_asset_reference(origin, &css_ref, asset_sources)
        else {
            continue;
        };
        let target_local_rel = target_origin.local_rel_path();
        materialize_asset(
            site_root_abs,
            &target_origin,
            asset_sources,
            asset_group,
            written,
            asset_records,
        )?;
        let replacement = relative_href(css_local_rel, &target_local_rel);
        rewritten = rewritten.replace(&css_ref, &replacement);
    }
    Ok(rewritten)
}

fn resolve_css_asset_reference(
    base_origin: &AssetOrigin,
    css_ref: &str,
    asset_sources: &StaticAssetSources<'_>,
) -> Option<AssetOrigin> {
    let trimmed = css_ref.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty()
        || trimmed.starts_with("data:")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("//")
        || trimmed.starts_with('#')
    {
        return None;
    }
    if trimmed.starts_with("/assets/") {
        return AssetOrigin::from_url(trimmed, asset_sources.owner, asset_sources.project);
    }
    let joined = match base_origin {
        AssetOrigin::Library(path) => join_relative_asset_path(path, trimmed)
            .ok()
            .map(AssetOrigin::Library),
        AssetOrigin::Platform(path) => join_relative_asset_path(path, trimmed)
            .ok()
            .map(AssetOrigin::Platform),
        AssetOrigin::Branding(path) => join_relative_asset_path(path, trimmed)
            .ok()
            .map(AssetOrigin::Branding),
        AssetOrigin::Project(path) => join_relative_asset_path(path, trimmed)
            .ok()
            .map(AssetOrigin::Project),
    };
    joined
}

fn join_relative_asset_path(base_file_path: &str, relative: &str) -> Result<String, PipelineError> {
    let base_dir = Path::new(base_file_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let joined = base_dir.join(relative);
    normalize_relative_asset_path(&joined)
}

fn normalize_relative_asset_path(path: &Path) -> Result<String, PipelineError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => {
                let value = part.to_string_lossy().trim().to_string();
                if !value.is_empty() {
                    parts.push(value);
                }
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(PipelineError::new(
                        "WEB_STATIC_SITE_ASSET_PATH",
                        "relative asset path escapes the static asset root",
                    ));
                }
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_SITE_ASSET_PATH",
            "relative asset path must not be empty",
        ));
    }
    Ok(parts.join("/"))
}

fn collect_css_url_refs(input: &str) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let mut cursor = 0usize;
    while let Some(offset) = input[cursor..].find("url(") {
        let start = cursor + offset + 4;
        let tail = &input[start..];
        let Some(end_rel) = tail.find(')') else {
            break;
        };
        let candidate = tail[..end_rel].trim().trim_matches('"').trim_matches('\'');
        if !candidate.is_empty() {
            refs.insert(candidate.to_string());
        }
        cursor = start + end_rel + 1;
    }
    refs
}

fn collect_css_import_refs(input: &str) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let mut cursor = 0usize;
    while let Some(offset) = input[cursor..].find("@import") {
        let start = cursor + offset + "@import".len();
        let tail = input[start..].trim_start();
        let consumed = input[start..].len() - tail.len();
        if let Some(rest) = tail.strip_prefix("url(") {
            let Some(end_rel) = rest.find(')') else {
                break;
            };
            let candidate = rest[..end_rel].trim().trim_matches('"').trim_matches('\'');
            if !candidate.is_empty() {
                refs.insert(candidate.to_string());
            }
            cursor = start + consumed + 4 + end_rel + 1;
            continue;
        }
        if let Some(quote) = tail.chars().next().filter(|ch| *ch == '"' || *ch == '\'') {
            let quote_len = quote.len_utf8();
            let rest = &tail[quote_len..];
            let Some(end_rel) = rest.find(quote) else {
                break;
            };
            let candidate = rest[..end_rel].trim();
            if !candidate.is_empty() {
                refs.insert(candidate.to_string());
            }
            cursor = start + consumed + quote_len + end_rel + quote_len;
            continue;
        }
        cursor = start + consumed + 1;
    }
    refs
}

fn materialize_runtime_family(embedded_rel: &str) -> Result<Vec<u8>, PipelineError> {
    let runtime_dir = Path::new(embedded_rel)
        .parent()
        .ok_or_else(|| {
            PipelineError::new(
                "WEB_STATIC_SITE_ASSET_PATH",
                format!("asset path '{embedded_rel}' has no parent runtime directory"),
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");
    let runtime_prefix = format!("{runtime_dir}/");

    let mut target_bytes = None;
    for asset in PLATFORM_LIBRARY_ASSETS {
        if asset.path == embedded_rel {
            target_bytes = Some(asset.bytes.to_vec());
            break;
        }
        if asset.path.starts_with(&runtime_prefix) {
            // family scan intentionally keeps working for sibling asset discovery
        }
    }

    target_bytes.ok_or_else(|| {
        PipelineError::new(
            "WEB_STATIC_SITE_ASSET_MISSING",
            format!("embedded library asset '{embedded_rel}' was not found"),
        )
    })
}

fn asset_kind_for_path(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".css") {
        "css".to_string()
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "js".to_string()
    } else if lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".ico")
        || lower.ends_with(".avif")
    {
        "image".to_string()
    } else if lower.ends_with(".woff")
        || lower.ends_with(".woff2")
        || lower.ends_with(".ttf")
        || lower.ends_with(".otf")
    {
        "font".to_string()
    } else {
        "other".to_string()
    }
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn relative_href(from_page_output_path: &str, to_rel_path: &str) -> String {
    let from_parts = path_segments(from_page_output_path);
    let to_parts = path_segments(to_rel_path);
    let from_dirs = &from_parts[..from_parts.len().saturating_sub(1)];

    let mut common = 0usize;
    while common < from_dirs.len()
        && common < to_parts.len()
        && from_dirs[common] == to_parts[common]
    {
        common += 1;
    }

    let mut parts = Vec::new();
    parts.extend(std::iter::repeat_n(
        "..".to_string(),
        from_dirs.len().saturating_sub(common),
    ));
    parts.extend(to_parts.into_iter().skip(common));
    if parts.is_empty() {
        ".".to_string()
    } else if parts[0] == ".." || parts[0].starts_with("./") || parts[0].starts_with('/') {
        parts.join("/")
    } else {
        format!("./{}", parts.join("/"))
    }
}

fn path_segments(raw: &str) -> Vec<String> {
    raw.replace('\\', "/")
        .split('/')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() || trimmed == "." {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        LocalizedStaticHtml, StaticAssetRecord, StaticAssetSources, StaticPageRecord,
        absolute_deploy_url, asset_group_id, localize_static_html_assets,
        normalize_deploy_base_path, normalize_page_output_path, normalize_site_root_rel_path,
        page_rel_path_from_site_root, route_path_for_output_path, site_manifest_rel_path,
        update_site_manifest,
    };
    use tempfile::tempdir;

    #[test]
    fn normalizes_site_root_and_page_paths() {
        assert_eq!(
            normalize_site_root_rel_path("static/musiklib").unwrap(),
            "static/musiklib"
        );
        assert_eq!(
            page_rel_path_from_site_root("static/musiklib", "a/aurora/index.html").unwrap(),
            "static/musiklib/a/aurora/index.html"
        );
        assert!(normalize_page_output_path("../escape.html").is_err());
        assert_eq!(
            site_manifest_rel_path("static/musiklib"),
            "static/musiklib/.zebflow-static-site.json"
        );
    }

    #[test]
    fn asset_group_hash_is_stable_for_same_template() {
        let a = asset_group_id("pages/lyrics.tsx", "<Page>Hello</Page>");
        let b = asset_group_id("pages/lyrics.tsx", "<Page>Hello</Page>");
        let c = asset_group_id("pages/lyrics.tsx", "<Page>World</Page>");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn normalizes_deploy_paths_and_routes() {
        assert_eq!(
            normalize_deploy_base_path(Some("/docs"), "/fallback").unwrap(),
            "/docs"
        );
        assert_eq!(
            route_path_for_output_path("/docs", "index.html").unwrap(),
            "/docs/"
        );
        assert_eq!(
            route_path_for_output_path("/docs", "basic/query/index.html").unwrap(),
            "/docs/basic/query/"
        );
        assert_eq!(
            route_path_for_output_path("/docs", "songs/let-it-be/lyrics.html").unwrap(),
            "/docs/songs/let-it-be/lyrics.html"
        );
        assert_eq!(
            absolute_deploy_url(Some("https://db.sekejap.life/"), "/docs/basic/query/"),
            Some("https://db.sekejap.life/docs/basic/query/".to_string())
        );
    }

    #[test]
    fn localizes_static_assets_into_site_assets() {
        let temp = tempdir().expect("tempdir");
        let project_assets = temp.path().join("project-assets");
        std::fs::create_dir_all(project_assets.join("icons")).expect("project assets dir");
        std::fs::write(project_assets.join("icons").join("favicon.ico"), b"ico").expect("favicon");
        std::fs::create_dir_all(project_assets.join("images")).expect("images dir");
        std::fs::write(project_assets.join("images").join("cover.png"), b"cover").expect("cover");
        std::fs::write(
            project_assets.join("images").join("cover@2x.png"),
            b"cover2x",
        )
        .expect("cover2x");
        std::fs::create_dir_all(project_assets.join("styles")).expect("styles dir");
        std::fs::create_dir_all(project_assets.join("theme")).expect("theme dir");
        std::fs::create_dir_all(project_assets.join("fonts")).expect("fonts dir");
        std::fs::write(
            project_assets.join("styles").join("base.css"),
            "@import \"../theme/theme.css\";\n.hero{background:url('../icons/favicon.ico');}\n",
        )
        .expect("base css");
        std::fs::write(
            project_assets.join("theme").join("theme.css"),
            "@font-face{src:url('../fonts/demo.woff2');}\n",
        )
        .expect("theme css");
        std::fs::write(project_assets.join("fonts").join("demo.woff2"), b"font").expect("font");
        let html = concat!(
            "<link rel=\"icon\" href=\"/assets/superadmin/default/icons/favicon.ico\">",
            "<link rel=\"stylesheet\" href=\"/assets/superadmin/default/styles/base.css\">",
            "<script type=\"module\">",
            "import { h } from '/assets/libraries/zeb/preact/0.1/runtime/preact.bundle.mjs';",
            "const mod = await import('/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs');",
            "</script>",
            "<link rel=\"stylesheet\" href=\"/assets/libraries/zeb/icons/0.1/runtime/devicons.css\">",
            "<img srcset=\"/assets/superadmin/default/images/cover.png 1x, /assets/superadmin/default/images/cover@2x.png 2x\">"
        );

        let LocalizedStaticHtml {
            html: rewritten,
            assets,
        } = localize_static_html_assets(
            temp.path(),
            "basic/query/index.html",
            html,
            StaticAssetSources {
                owner: Some("superadmin"),
                project: Some("default"),
                project_asset_root_abs: Some(project_assets.as_path()),
            },
            "tpl-test",
        )
        .expect("localize assets");

        assert!(
            rewritten.contains("../../_assets/libraries/zeb/preact/0.1/runtime/preact.bundle.mjs")
        );
        assert!(rewritten.contains("../../_assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs"));
        assert!(rewritten.contains("../../_assets/libraries/zeb/icons/0.1/runtime/devicons.css"));
        assert!(rewritten.contains("../../_assets/project/icons/favicon.ico"));
        assert!(rewritten.contains("../../_assets/project/styles/base.css"));
        assert!(rewritten.contains("../../_assets/project/images/cover.png 1x"));
        assert!(rewritten.contains("../../_assets/project/images/cover@2x.png 2x"));
        assert!(
            temp.path()
                .join("_assets/libraries/zeb/preact/0.1/runtime/preact.bundle.mjs")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/libraries/zeb/icons/0.1/runtime/devicons.css")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/project/icons/favicon.ico")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/project/styles/base.css")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/project/theme/theme.css")
                .is_file()
        );
        assert!(
            temp.path()
                .join("_assets/project/fonts/demo.woff2")
                .is_file()
        );
        let localized_css =
            std::fs::read_to_string(temp.path().join("_assets/project/styles/base.css"))
                .expect("localized base css");
        assert!(localized_css.contains("../theme/theme.css"));
        assert!(localized_css.contains("../icons/favicon.ico"));
        let localized_theme_css =
            std::fs::read_to_string(temp.path().join("_assets/project/theme/theme.css"))
                .expect("localized theme css");
        assert!(localized_theme_css.contains("../fonts/demo.woff2"));
        assert!(
            assets
                .iter()
                .any(|asset| asset.path == "_assets/project/icons/favicon.ico")
        );
        assert!(
            assets
                .iter()
                .any(|asset| asset.path == "_assets/project/fonts/demo.woff2")
        );
        assert!(assets.iter().all(|asset| asset.asset_group == "tpl-test"));
    }

    #[test]
    fn manifest_prunes_full_group_assets_and_pages() {
        let temp = tempdir().expect("tempdir");
        let manifest_abs = temp.path().join(".zebflow-static-site.json");
        let stale_asset_abs = temp.path().join("_assets/project/old.png");
        std::fs::create_dir_all(stale_asset_abs.parent().expect("asset parent"))
            .expect("create stale parent");
        std::fs::write(&stale_asset_abs, b"old").expect("write stale asset");

        let _ = update_site_manifest(
            &manifest_abs,
            "docs",
            Some("https://db.sekejap.life"),
            "/docs",
            "web.docs.generate",
            "pages/docs/docs.template.tsx",
            "tpl-docs",
            &[StaticPageRecord {
                path: "old/index.html".to_string(),
                route: "/docs/old/".to_string(),
                template: "pages/docs/docs.template.tsx".to_string(),
                asset_group: "tpl-docs".to_string(),
                generator: "web.docs.generate".to_string(),
            }],
            &[StaticAssetRecord {
                path: "_assets/project/old.png".to_string(),
                source_url: "/assets/superadmin/default/old.png".to_string(),
                kind: "image".to_string(),
                content_hash: "old".to_string(),
                asset_group: "tpl-docs".to_string(),
            }],
            false,
        )
        .expect("initial manifest");

        let manifest = update_site_manifest(
            &manifest_abs,
            "docs",
            Some("https://db.sekejap.life"),
            "/docs",
            "web.docs.generate",
            "pages/docs/docs.template.tsx",
            "tpl-docs",
            &[StaticPageRecord {
                path: "index.html".to_string(),
                route: "/docs/".to_string(),
                template: "pages/docs/docs.template.tsx".to_string(),
                asset_group: "tpl-docs".to_string(),
                generator: "web.docs.generate".to_string(),
            }],
            &[StaticAssetRecord {
                path: "_assets/project/new.png".to_string(),
                source_url: "/assets/superadmin/default/new.png".to_string(),
                kind: "image".to_string(),
                content_hash: "new".to_string(),
                asset_group: "tpl-docs".to_string(),
            }],
            true,
        )
        .expect("pruned manifest");

        assert!(!stale_asset_abs.exists());
        assert_eq!(manifest.pages.len(), 1);
        assert_eq!(manifest.pages[0].path, "index.html");
        assert_eq!(manifest.assets.len(), 1);
        assert_eq!(manifest.assets[0].path, "_assets/project/new.png");
    }
}
