//! File-based Kubernetes cluster layout manager for Zebflow.
//!
//! This module intentionally manages files on disk first. It does not apply anything to a live
//! cluster. The CLI mutates a small set of YAML manifests and only rewrites the blocks explicitly
//! marked as Zebflow-owned.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::version::APP_VERSION;

const MANAGEMENT_FILE: &str = "management.yaml";
const AUTO_UPDATE_FILE: &str = "auto-update.yaml";
const MANAGEMENT_SECRET_NAME: &str = "zebflow-management-secrets";
const MANAGEMENT_CONFIGMAP_NAME: &str = "zebflow-management";
const DEFAULT_NAMESPACE: &str = "zebflow";
const DEFAULT_CONTROLLER_ID: &str = "office-main";
const DEFAULT_SCHEDULE: &str = "* * * * *";

const BLOCK_MANAGEMENT_CONFIG: &str = "config";
const BLOCK_OFFICE_CONTAINER: &str = "container";
const BLOCK_AUTOUPDATE_RESOURCE_NAMES: &str = "resource-names";
const BLOCK_AUTOUPDATE_SUSPEND: &str = "suspend";
const BLOCK_AUTOUPDATE_TARGETS: &str = "target-workloads";
const BLOCK_AUTOUPDATE_IMAGE_REPO: &str = "image-repo";
const CLUSTER_LOCK_FILE: &str = ".zebflow.cluster.lock";
const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumePermissionStrategy {
    FsGroup,
    InitChmod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClusterConfig {
    cluster_name: String,
    namespace: String,
    controller_office_id: String,
    managed_image: String,
    managed_offices: Vec<String>,
    managed_workloads: Vec<String>,
    auto_update_enabled: bool,
    volume_permission_strategy: VolumePermissionStrategy,
    resource_suffix: String,
    management_secret_name: String,
    manage_secret: bool,
    replicas: u32,
    precreate_pvcs: bool,
}

pub fn run_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_k8s_help();
        return Ok(());
    }
    if args.len() == 1 && matches!(args[0].as_str(), "help" | "--help" | "-h") {
        print_k8s_help();
        return Ok(());
    }
    match args {
        [cluster, command, rest @ ..] if cluster == "cluster" => run_cluster_command(command, rest),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown zebflow k8s command\n\n{}", k8s_help_text()),
        )
        .into()),
    }
}

fn run_cluster_command(command: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(command, "help" | "--help" | "-h") {
        print_cluster_help();
        return Ok(());
    }
    match (command, args) {
        ("init", [help]) if matches!(help.as_str(), "help" | "--help" | "-h") => {
            print_cluster_help();
            Ok(())
        }
        ("init", [path]) => {
            init_cluster(Path::new(path))?;
            println!(
                "initialized k8s cluster folder at {}",
                Path::new(path).display()
            );
            Ok(())
        }
        ("add-office", [path, office_id]) => {
            add_office(Path::new(path), office_id)?;
            println!(
                "added office '{}' in {}",
                office_id,
                Path::new(path).display()
            );
            Ok(())
        }
        ("set-controller", [path, office_id]) => {
            set_controller(Path::new(path), office_id)?;
            println!(
                "set controller office '{}' in {}",
                office_id,
                Path::new(path).display()
            );
            Ok(())
        }
        ("set-namespace", [path, namespace]) => {
            set_namespace(Path::new(path), namespace)?;
            println!(
                "set namespace to '{}' in {}",
                namespace,
                Path::new(path).display()
            );
            Ok(())
        }
        ("set-resource-suffix", [path, suffix]) => {
            set_resource_suffix(Path::new(path), suffix)?;
            println!(
                "set resource suffix to '{}' in {}",
                suffix,
                Path::new(path).display()
            );
            Ok(())
        }
        ("set-image", [path, image]) => {
            set_image(Path::new(path), image)?;
            println!(
                "set managed image to '{}' in {}",
                image,
                Path::new(path).display()
            );
            Ok(())
        }
        ("use-secret", [path, secret_name]) => {
            use_external_secret(Path::new(path), secret_name)?;
            println!(
                "set external management secret '{}' in {}",
                secret_name,
                Path::new(path).display()
            );
            Ok(())
        }
        ("set-replicas", [path, replicas]) => {
            let replicas = replicas.parse::<u32>().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("replicas must be an unsigned integer: {err}"),
                )
            })?;
            set_replicas(Path::new(path), replicas)?;
            println!(
                "set replicas to {} in {}",
                replicas,
                Path::new(path).display()
            );
            Ok(())
        }
        ("enable-precreate-pvcs", [path]) => {
            set_precreate_pvcs(Path::new(path), true)?;
            println!("enabled precreated PVCs in {}", Path::new(path).display());
            Ok(())
        }
        ("disable-precreate-pvcs", [path]) => {
            set_precreate_pvcs(Path::new(path), false)?;
            println!("disabled precreated PVCs in {}", Path::new(path).display());
            Ok(())
        }
        ("enable-auto-update", [path]) => {
            set_auto_update(Path::new(path), true)?;
            println!("enabled auto-update in {}", Path::new(path).display());
            Ok(())
        }
        ("disable-auto-update", [path]) => {
            set_auto_update(Path::new(path), false)?;
            println!("disabled auto-update in {}", Path::new(path).display());
            Ok(())
        }
        ("describe", [path]) => {
            describe_cluster(Path::new(path))?;
            Ok(())
        }
        ("validate", [path]) => {
            validate_cluster(Path::new(path))?;
            println!("cluster folder is valid: {}", Path::new(path).display());
            Ok(())
        }
        ("render-copy-jobs", [source_path, target_path, output_file]) => {
            render_copy_jobs_file(
                Path::new(source_path),
                Path::new(target_path),
                Path::new(output_file),
            )?;
            println!(
                "rendered copy jobs from {} to {} at {}",
                Path::new(source_path).display(),
                Path::new(target_path).display(),
                Path::new(output_file).display()
            );
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown zebflow k8s cluster command\n\n{}",
                cluster_help_text()
            ),
        )
        .into()),
    }
}

fn k8s_help_text() -> String {
    "Zebflow Kubernetes file manager

Usage:
  zebflow k8s cluster <command> ...
  zebflow k8s --help

Commands:
  cluster init <path>
  cluster add-office <path> <office-id>
  cluster set-controller <path> <office-id>
  cluster set-namespace <path> <namespace>
  cluster set-resource-suffix <path> <suffix>
  cluster set-image <path> <image>
  cluster use-secret <path> <secret-name>
  cluster set-replicas <path> <replicas>
  cluster enable-precreate-pvcs <path>
  cluster disable-precreate-pvcs <path>
  cluster enable-auto-update <path>
  cluster disable-auto-update <path>
  cluster render-copy-jobs <source-path> <target-path> <output-file>
  cluster describe <path>
  cluster validate <path>

This surface only manages YAML files on disk. It does not apply anything to a live cluster."
        .to_string()
}

fn cluster_help_text() -> String {
    "Zebflow Kubernetes cluster file manager

Usage:
  zebflow k8s cluster init <path>
  zebflow k8s cluster add-office <path> <office-id>
  zebflow k8s cluster set-controller <path> <office-id>
  zebflow k8s cluster set-namespace <path> <namespace>
  zebflow k8s cluster set-resource-suffix <path> <suffix>
  zebflow k8s cluster set-image <path> <image>
  zebflow k8s cluster use-secret <path> <secret-name>
  zebflow k8s cluster set-replicas <path> <replicas>
  zebflow k8s cluster enable-precreate-pvcs <path>
  zebflow k8s cluster disable-precreate-pvcs <path>
  zebflow k8s cluster enable-auto-update <path>
  zebflow k8s cluster disable-auto-update <path>
  zebflow k8s cluster render-copy-jobs <source-path> <target-path> <output-file>
  zebflow k8s cluster describe <path>
  zebflow k8s cluster validate <path>

Files managed on disk:
  management.yaml
  office-main.yaml
  <office-id>.yaml
  auto-update.yaml

Zebflow only rewrites its managed blocks inside these files."
        .to_string()
}

fn print_k8s_help() {
    println!("{}", k8s_help_text());
}

fn print_cluster_help() {
    println!("{}", cluster_help_text());
}

fn init_cluster(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(root)?;
    with_cluster_lock(root, || {
        let management_path = root.join(MANAGEMENT_FILE);
        if management_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", management_path.display()),
            )
            .into());
        }

        let cluster_name = root
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("zebflow-cluster")
            .to_string();

        let cfg = ClusterConfig {
            cluster_name,
            namespace: DEFAULT_NAMESPACE.to_string(),
            controller_office_id: DEFAULT_CONTROLLER_ID.to_string(),
            managed_image: default_image(),
            managed_offices: vec![DEFAULT_CONTROLLER_ID.to_string()],
            managed_workloads: vec![managed_workload_ref(DEFAULT_CONTROLLER_ID)],
            auto_update_enabled: false,
            volume_permission_strategy: VolumePermissionStrategy::FsGroup,
            resource_suffix: String::new(),
            management_secret_name: MANAGEMENT_SECRET_NAME.to_string(),
            manage_secret: true,
            replicas: 1,
            precreate_pvcs: false,
        };

        write_text(&management_path, &render_management_yaml(&cfg))?;
        write_text(
            &root.join(DEFAULT_CONTROLLER_ID.to_string() + ".yaml"),
            &render_office_yaml(&cfg, DEFAULT_CONTROLLER_ID),
        )?;
        write_text(&root.join(AUTO_UPDATE_FILE), &render_auto_update_yaml(&cfg))?;
        Ok(())
    })
}

fn add_office(root: &Path, office_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_office_id(office_id)?;
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        if cfg.managed_offices.iter().any(|value| value == office_id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("office '{}' already exists", office_id),
            )
            .into());
        }

        let office_path = root.join(format!("{office_id}.yaml"));
        if office_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", office_path.display()),
            )
            .into());
        }

        cfg.managed_offices.push(office_id.to_string());
        cfg.managed_workloads
            .push(managed_workload_ref_for(&cfg, office_id));
        cfg.managed_offices.sort();
        cfg.managed_workloads.sort();

        write_text(&office_path, &render_office_yaml(&cfg, office_id))?;
        save_cluster_config(root, &cfg)?;
        refresh_auto_update(root, &cfg)?;
        Ok(())
    })
}

fn set_controller(root: &Path, office_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_office_id(office_id)?;
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        if !cfg.managed_offices.iter().any(|value| value == office_id) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "office '{}' is not managed in this cluster folder",
                    office_id
                ),
            )
            .into());
        }
        cfg.controller_office_id = office_id.to_string();
        save_cluster_config(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        Ok(())
    })
}

fn set_namespace(root: &Path, namespace: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_resource_name(namespace, "namespace")?;
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.namespace = namespace.trim().to_string();
        rewrite_management_manifest(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        rewrite_auto_update_manifest(root, &cfg)?;
        Ok(())
    })
}

fn set_resource_suffix(root: &Path, suffix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let suffix = suffix.trim();
    if !suffix.is_empty() {
        validate_resource_name(suffix, "resource suffix")?;
    }
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.resource_suffix = suffix.to_string();
        cfg.managed_workloads = cfg
            .managed_offices
            .iter()
            .map(|office_id| managed_workload_ref_for(&cfg, office_id))
            .collect();
        rewrite_management_manifest(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        rewrite_auto_update_manifest(root, &cfg)?;
        Ok(())
    })
}

fn set_image(root: &Path, image: &str) -> Result<(), Box<dyn std::error::Error>> {
    if image.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "image cannot be empty").into());
    }
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.managed_image = image.trim().to_string();
        save_cluster_config(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        refresh_auto_update(root, &cfg)?;
        Ok(())
    })
}

fn use_external_secret(root: &Path, secret_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_resource_name(secret_name, "secret name")?;
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.management_secret_name = secret_name.trim().to_string();
        cfg.manage_secret = false;
        rewrite_management_manifest(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        Ok(())
    })
}

fn set_replicas(root: &Path, replicas: u32) -> Result<(), Box<dyn std::error::Error>> {
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.replicas = replicas;
        save_cluster_config(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        Ok(())
    })
}

fn set_precreate_pvcs(root: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.precreate_pvcs = enabled;
        save_cluster_config(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        Ok(())
    })
}

fn set_auto_update(root: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    with_cluster_lock(root, || {
        let mut cfg = load_cluster_config(root)?;
        cfg.auto_update_enabled = enabled;
        save_cluster_config(root, &cfg)?;
        refresh_office_manifests(root, &cfg)?;
        refresh_auto_update(root, &cfg)?;
        Ok(())
    })
}

fn render_copy_jobs_file(
    source_root: &Path,
    target_root: &Path,
    output_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = load_cluster_config(source_root)?;
    let target = load_cluster_config(target_root)?;
    if source.namespace != target.namespace {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "copy jobs require source and target namespaces to match: '{}' != '{}'",
                source.namespace, target.namespace
            ),
        )
        .into());
    }
    let content = render_copy_jobs_yaml(&source, &target)?;
    write_text(output_file, &content)?;
    Ok(())
}

fn render_copy_jobs_yaml(
    source: &ClusterConfig,
    target: &ClusterConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut jobs = Vec::new();
    for office_id in &target.managed_offices {
        if !source
            .managed_offices
            .iter()
            .any(|value| value == office_id)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("source cluster has no office '{office_id}'"),
            )
            .into());
        }
        let source_workload = workload_name_for_office(source, office_id);
        let target_workload = workload_name_for_office(target, office_id);
        jobs.push(render_copy_job_yaml(
            &target.namespace,
            office_id,
            &source_workload,
            &target_workload,
        ));
    }
    Ok(jobs.join("\n---\n"))
}

fn render_copy_job_yaml(
    namespace: &str,
    office_id: &str,
    source_workload: &str,
    target_workload: &str,
) -> String {
    format!(
        "apiVersion: batch/v1\nkind: Job\nmetadata:\n  name: migrate-{source_workload}-to-{target_workload}\n  namespace: {namespace}\n  labels:\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  backoffLimit: 1\n  template:\n    metadata:\n      labels:\n        zebflow.io/managed-by: zebflow\n        zebflow.io/office-id: {office_id}\n    spec:\n      securityContext:\n        runAsNonRoot: true\n        runAsUser: 1000\n        runAsGroup: 1000\n        fsGroup: 1000\n        fsGroupChangePolicy: OnRootMismatch\n        seccompProfile:\n          type: RuntimeDefault\n      restartPolicy: Never\n      tolerations:\n        - key: node-role.kubernetes.io/control-plane\n          operator: Exists\n          effect: NoSchedule\n      containers:\n        - name: copy\n          image: busybox:1.36\n          imagePullPolicy: IfNotPresent\n          securityContext:\n            runAsNonRoot: true\n            runAsUser: 1000\n            runAsGroup: 1000\n            allowPrivilegeEscalation: false\n            capabilities:\n              drop:\n                - ALL\n          command:\n            - sh\n            - -c\n            - |\n              set -eu\n              echo \"initial copy {source_pvc} -> {target_pvc}\"\n              mkdir -p /new\n              rm -rf /new/lost+found\n              cd /old\n              tar cf - . | (cd /new && tar xf -)\n              sync\n              echo \"copy complete {office_id}\"\n          volumeMounts:\n            - name: old-data\n              mountPath: /old\n              readOnly: true\n            - name: new-data\n              mountPath: /new\n      volumes:\n        - name: old-data\n          persistentVolumeClaim:\n            claimName: {source_pvc}\n            readOnly: true\n        - name: new-data\n          persistentVolumeClaim:\n            claimName: {target_pvc}\n",
        namespace = namespace,
        office_id = office_id,
        source_workload = source_workload,
        target_workload = target_workload,
        source_pvc = data_pvc_name(source_workload),
        target_pvc = data_pvc_name(target_workload),
    )
}

fn describe_cluster(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_cluster_config(root)?;
    println!("Cluster: {}", cfg.cluster_name);
    println!("Namespace: {}", cfg.namespace);
    println!("Controller: {}", cfg.controller_office_id);
    println!("Image: {}", cfg.managed_image);
    println!(
        "Auto update: {}",
        if cfg.auto_update_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("Offices:");
    for office_id in &cfg.managed_offices {
        let role = if office_id == &cfg.controller_office_id {
            "controller"
        } else {
            "office"
        };
        println!(
            "  - {} ({}) [{}]",
            office_id,
            role,
            root.join(format!("{office_id}.yaml")).display()
        );
    }
    Ok(())
}

fn validate_cluster(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_cluster_config(root)?;
    if cfg.managed_offices.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidData, "managed offices list is empty").into(),
        );
    }
    if !cfg
        .managed_offices
        .iter()
        .any(|value| value == &cfg.controller_office_id)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "controller '{}' is not present in managed offices",
                cfg.controller_office_id
            ),
        )
        .into());
    }
    let unique_offices = cfg.managed_offices.iter().cloned().collect::<BTreeSet<_>>();
    if unique_offices.len() != cfg.managed_offices.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "managed offices contains duplicates",
        )
        .into());
    }
    for office_id in &cfg.managed_offices {
        validate_office_id(office_id)?;
        let path = root.join(format!("{office_id}.yaml"));
        let content = fs::read_to_string(&path)?;
        ensure_block_present(&content, BLOCK_OFFICE_CONTAINER)?;
        ensure_contains(&content, &format!("name: {office_id}"), &path)?;
        ensure_contains(&content, &format!("value: \"{office_id}\""), &path)?;
    }

    let management = fs::read_to_string(root.join(MANAGEMENT_FILE))?;
    ensure_block_present(&management, BLOCK_MANAGEMENT_CONFIG)?;

    let auto_update = fs::read_to_string(root.join(AUTO_UPDATE_FILE))?;
    ensure_block_present(&auto_update, BLOCK_AUTOUPDATE_RESOURCE_NAMES)?;
    ensure_block_present(&auto_update, BLOCK_AUTOUPDATE_SUSPEND)?;
    ensure_block_present(&auto_update, BLOCK_AUTOUPDATE_TARGETS)?;
    ensure_block_present(&auto_update, BLOCK_AUTOUPDATE_IMAGE_REPO)?;

    for workload in &cfg.managed_workloads {
        let workload_name = workload_name_from_ref(workload).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid managed workload ref '{workload}'"),
            )
        })?;
        ensure_contains(
            &auto_update,
            &format!("- {workload_name}"),
            &root.join(AUTO_UPDATE_FILE),
        )?;
    }
    ensure_contains(
        &auto_update,
        &format!("value: \"{}\"", cfg.managed_workloads.join(",")),
        &root.join(AUTO_UPDATE_FILE),
    )?;
    ensure_contains(
        &auto_update,
        &format!(
            "suspend: {}",
            if cfg.auto_update_enabled {
                "false"
            } else {
                "true"
            }
        ),
        &root.join(AUTO_UPDATE_FILE),
    )?;
    Ok(())
}

fn save_cluster_config(root: &Path, cfg: &ClusterConfig) -> Result<(), Box<dyn std::error::Error>> {
    let management_path = root.join(MANAGEMENT_FILE);
    let content = fs::read_to_string(&management_path)?;
    let next = replace_managed_block(
        &content,
        BLOCK_MANAGEMENT_CONFIG,
        &render_management_config_block(cfg),
    )?;
    write_text(&management_path, &next)?;
    Ok(())
}

fn rewrite_management_manifest(
    root: &Path,
    cfg: &ClusterConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    write_text(&root.join(MANAGEMENT_FILE), &render_management_yaml(cfg))?;
    Ok(())
}

fn refresh_auto_update(root: &Path, cfg: &ClusterConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(AUTO_UPDATE_FILE);
    let content = fs::read_to_string(&path)?;
    let next = replace_managed_block(
        &replace_managed_block(
            &replace_managed_block(
                &replace_managed_block(
                    &content,
                    BLOCK_AUTOUPDATE_RESOURCE_NAMES,
                    &render_auto_update_resource_names_block(cfg),
                )?,
                BLOCK_AUTOUPDATE_SUSPEND,
                &render_auto_update_suspend_block(cfg),
            )?,
            BLOCK_AUTOUPDATE_TARGETS,
            &render_auto_update_targets_block(cfg),
        )?,
        BLOCK_AUTOUPDATE_IMAGE_REPO,
        &render_auto_update_image_repo_block(cfg),
    )?;
    write_text(&path, &next)?;
    Ok(())
}

fn rewrite_auto_update_manifest(
    root: &Path,
    cfg: &ClusterConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    write_text(&root.join(AUTO_UPDATE_FILE), &render_auto_update_yaml(cfg))?;
    Ok(())
}

fn refresh_office_manifests(
    root: &Path,
    cfg: &ClusterConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    for office_id in &cfg.managed_offices {
        let path = root.join(format!("{office_id}.yaml"));
        write_text(&path, &render_office_yaml(cfg, office_id))?;
    }
    Ok(())
}

fn load_cluster_config(root: &Path) -> Result<ClusterConfig, Box<dyn std::error::Error>> {
    let management = fs::read_to_string(root.join(MANAGEMENT_FILE))?;
    let block = extract_managed_block(&management, BLOCK_MANAGEMENT_CONFIG)?;
    let values = parse_config_block(&block)?;

    let managed_offices = csv_values(values.get("MANAGED_OFFICES"));
    let managed_workloads = csv_values(values.get("MANAGED_WORKLOADS"))
        .into_iter()
        .map(|workload| normalize_workload_ref(&workload))
        .collect();
    let auto_update_enabled = values
        .get("AUTO_UPDATE_ENABLED")
        .map(|value| value == "true")
        .unwrap_or(false);

    Ok(ClusterConfig {
        cluster_name: values
            .get("CLUSTER_NAME")
            .cloned()
            .unwrap_or_else(|| "zebflow-cluster".to_string()),
        namespace: values
            .get("NAMESPACE")
            .cloned()
            .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
        controller_office_id: values
            .get("CONTROLLER_OFFICE_ID")
            .cloned()
            .unwrap_or_else(|| DEFAULT_CONTROLLER_ID.to_string()),
        managed_image: values
            .get("MANAGED_IMAGE")
            .cloned()
            .unwrap_or_else(default_image),
        managed_offices,
        managed_workloads,
        auto_update_enabled,
        volume_permission_strategy: values
            .get("VOLUME_PERMISSION_STRATEGY")
            .map(|value| parse_volume_permission_strategy(value))
            .transpose()?
            .unwrap_or(VolumePermissionStrategy::FsGroup),
        resource_suffix: values.get("RESOURCE_SUFFIX").cloned().unwrap_or_default(),
        management_secret_name: values
            .get("MANAGEMENT_SECRET_NAME")
            .cloned()
            .unwrap_or_else(|| MANAGEMENT_SECRET_NAME.to_string()),
        manage_secret: values
            .get("MANAGE_SECRET")
            .map(|value| value == "true")
            .unwrap_or(true),
        replicas: values
            .get("REPLICAS")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1),
        precreate_pvcs: values
            .get("PRECREATE_PVCS")
            .map(|value| value == "true")
            .unwrap_or(false),
    })
}

fn render_management_yaml(cfg: &ClusterConfig) -> String {
    let secret_yaml = if cfg.manage_secret {
        format!(
            "---\napiVersion: v1\nkind: Secret\nmetadata:\n  name: {secret}\n  namespace: {namespace}\ntype: Opaque\nstringData:\n  ZEBFLOW_PLATFORM_DEFAULT_PASSWORD: \"CHANGE_ME_TO_A_REAL_PASSWORD\"\n  ZEBFLOW_CLUSTER_JOIN_TOKEN: \"CHANGE_ME_TO_A_REAL_JOIN_TOKEN\"\n",
            secret = cfg.management_secret_name,
            namespace = cfg.namespace,
        )
    } else {
        String::new()
    };
    format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {namespace}\n{secret_yaml}---\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: {configmap}\n  namespace: {namespace}\n  annotations:\n    zebflow.io/managed-by: zebflow\ndata:\n  # zebflow:managed-begin {block}\n{config_block}  # zebflow:managed-end {block}\n",
        namespace = cfg.namespace,
        secret_yaml = secret_yaml,
        configmap = management_configmap_name(cfg),
        block = BLOCK_MANAGEMENT_CONFIG,
        config_block = render_management_config_block(cfg)
    )
}

fn render_management_config_block(cfg: &ClusterConfig) -> String {
    let mut lines = Vec::new();
    lines.push(format!("  CLUSTER_NAME: \"{}\"", cfg.cluster_name));
    lines.push(format!("  NAMESPACE: \"{}\"", cfg.namespace));
    lines.push(format!(
        "  CONTROLLER_OFFICE_ID: \"{}\"",
        cfg.controller_office_id
    ));
    lines.push(format!(
        "  CONTROLLER_WORKLOAD: \"{}\"",
        workload_ref_for_office(cfg, &cfg.controller_office_id)
    ));
    lines.push(format!(
        "  CONTROLLER_URL: \"{}\"",
        office_service_url(
            &service_name_for_office(cfg, &cfg.controller_office_id),
            &cfg.namespace
        )
    ));
    lines.push(format!("  MANAGED_IMAGE: \"{}\"", cfg.managed_image));
    lines.push(format!(
        "  MANAGED_OFFICES: \"{}\"",
        cfg.managed_offices.join(",")
    ));
    lines.push(format!(
        "  MANAGED_WORKLOADS: \"{}\"",
        cfg.managed_workloads.join(",")
    ));
    lines.push("  MARKETPLACE_DEFAULT_BASE_URL: \"https://market.zebflow.com/api\"".to_string());
    lines.push(format!(
        "  AUTO_UPDATE_ENABLED: \"{}\"",
        if cfg.auto_update_enabled {
            "true"
        } else {
            "false"
        }
    ));
    lines.push(format!(
        "  VOLUME_PERMISSION_STRATEGY: \"{}\"",
        cfg.volume_permission_strategy.as_str()
    ));
    lines.push(format!("  RESOURCE_SUFFIX: \"{}\"", cfg.resource_suffix));
    lines.push(format!(
        "  MANAGEMENT_SECRET_NAME: \"{}\"",
        cfg.management_secret_name
    ));
    lines.push(format!(
        "  MANAGE_SECRET: \"{}\"",
        if cfg.manage_secret { "true" } else { "false" }
    ));
    lines.push(format!("  REPLICAS: \"{}\"", cfg.replicas));
    lines.push(format!(
        "  PRECREATE_PVCS: \"{}\"",
        if cfg.precreate_pvcs { "true" } else { "false" }
    ));
    lines.join("\n") + "\n"
}

fn render_office_yaml(cfg: &ClusterConfig, office_id: &str) -> String {
    let pod_security_context = render_office_pod_security_context(cfg);
    let init_containers = render_office_init_containers(cfg);
    let service_name = service_name_for_office(cfg, office_id);
    let workload_name = workload_name_for_office(cfg, office_id);
    let storage = office_storage(office_id);
    let precreated_pvc = if cfg.precreate_pvcs {
        render_office_precreated_pvc(cfg, office_id, &workload_name, storage)
    } else {
        String::new()
    };
    format!(
        "apiVersion: v1\nkind: Service\nmetadata:\n  name: {service_name}\n  namespace: {namespace}\n  labels:\n    app: {workload_name}\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  selector:\n    app: {workload_name}\n  ports:\n    - name: http\n      port: 10610\n      targetPort: http\n{precreated_pvc}---\napiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: {workload_name}\n  namespace: {namespace}\n  labels:\n    app: {workload_name}\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  serviceName: {service_name}\n  replicas: {replicas}\n  selector:\n    matchLabels:\n      app: {workload_name}\n  template:\n    metadata:\n      labels:\n        app: {workload_name}\n        zebflow.io/managed-by: zebflow\n        zebflow.io/office-id: {office_id}\n    spec:\n      automountServiceAccountToken: false\n      securityContext:\n{pod_security_context}      tolerations:\n        - key: node-role.kubernetes.io/control-plane\n          operator: Exists\n          effect: NoSchedule\n{init_containers}      containers:\n        - name: zebflow\n          # zebflow:managed-begin {block}\n{container_block}          # zebflow:managed-end {block}\n          ports:\n            - name: http\n              containerPort: 10610\n              protocol: TCP\n          readinessProbe:\n            httpGet:\n              path: /ready\n              port: http\n            initialDelaySeconds: 10\n            periodSeconds: 5\n            timeoutSeconds: 3\n            failureThreshold: 6\n          livenessProbe:\n            httpGet:\n              path: /health\n              port: http\n            initialDelaySeconds: 30\n            periodSeconds: 10\n            timeoutSeconds: 5\n            failureThreshold: 6\n          resources:\n            requests:\n              cpu: \"100m\"\n              memory: \"256Mi\"\n            limits:\n              cpu: \"1000m\"\n              memory: \"1Gi\"\n          securityContext:\n            runAsNonRoot: true\n            runAsUser: 1000\n            runAsGroup: 1000\n            allowPrivilegeEscalation: false\n            capabilities:\n              drop:\n                - ALL\n            readOnlyRootFilesystem: false\n            seccompProfile:\n              type: RuntimeDefault\n          volumeMounts:\n            - name: data\n              mountPath: /var/lib/zebflow/data\n  volumeClaimTemplates:\n    - metadata:\n        name: data\n      spec:\n        accessModes:\n          - ReadWriteOnce\n        storageClassName: local-path\n        resources:\n          requests:\n            storage: {storage}\n",
        namespace = cfg.namespace,
        office_id = office_id,
        service_name = service_name,
        workload_name = workload_name,
        replicas = cfg.replicas,
        precreated_pvc = precreated_pvc,
        block = BLOCK_OFFICE_CONTAINER,
        pod_security_context = pod_security_context,
        init_containers = init_containers,
        container_block = render_office_container_block(cfg, office_id),
        storage = storage,
    )
}

fn render_office_precreated_pvc(
    cfg: &ClusterConfig,
    office_id: &str,
    workload_name: &str,
    storage: &str,
) -> String {
    format!(
        "---\napiVersion: v1\nkind: PersistentVolumeClaim\nmetadata:\n  name: {pvc_name}\n  namespace: {namespace}\n  labels:\n    app: {workload_name}\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  accessModes:\n    - ReadWriteOnce\n  storageClassName: local-path\n  resources:\n    requests:\n      storage: {storage}\n",
        pvc_name = data_pvc_name(workload_name),
        namespace = cfg.namespace,
        workload_name = workload_name,
        office_id = office_id,
        storage = storage,
    )
}

fn render_office_pod_security_context(cfg: &ClusterConfig) -> String {
    match cfg.volume_permission_strategy {
        VolumePermissionStrategy::FsGroup => {
            "        fsGroup: 1000\n        fsGroupChangePolicy: OnRootMismatch\n".to_string()
        }
        VolumePermissionStrategy::InitChmod => String::new(),
    }
}

fn render_office_init_containers(cfg: &ClusterConfig) -> String {
    match cfg.volume_permission_strategy {
        VolumePermissionStrategy::FsGroup => String::new(),
        VolumePermissionStrategy::InitChmod => "      initContainers:\n        - name: volume-permissions\n          image: busybox:1.36\n          imagePullPolicy: IfNotPresent\n          command:\n            - sh\n            - -c\n            - mkdir -p /var/lib/zebflow/data && chmod -R 0777 /var/lib/zebflow/data\n          securityContext:\n            runAsUser: 0\n          volumeMounts:\n            - name: data\n              mountPath: /var/lib/zebflow/data\n".to_string(),
    }
}

fn render_office_container_block(cfg: &ClusterConfig, office_id: &str) -> String {
    let mut lines = vec![
        format!("          image: {}", cfg.managed_image),
        "          imagePullPolicy: Always".to_string(),
        "          args:".to_string(),
        format!(
            "            - {}",
            if office_id == cfg.controller_office_id {
                "controller"
            } else {
                "office"
            }
        ),
        "          env:".to_string(),
        "            - name: ZEBFLOW_PLATFORM_HOST".to_string(),
        "              value: \"0.0.0.0\"".to_string(),
        "            - name: ZEBFLOW_PLATFORM_PORT".to_string(),
        "              value: \"10610\"".to_string(),
        "            - name: ZEBFLOW_PLATFORM_DATA_DIR".to_string(),
        "              value: /var/lib/zebflow/data".to_string(),
        "            - name: ZEBFLOW_MARKETPLACE_DEFAULT_BASE_URL".to_string(),
        "              valueFrom:".to_string(),
        "                configMapKeyRef:".to_string(),
        format!("                  name: {}", management_configmap_name(cfg)),
        "                  key: MARKETPLACE_DEFAULT_BASE_URL".to_string(),
        "            - name: ZEBFLOW_PLATFORM_DEFAULT_PASSWORD".to_string(),
        "              valueFrom:".to_string(),
        "                secretKeyRef:".to_string(),
        format!("                  name: {}", cfg.management_secret_name),
        "                  key: ZEBFLOW_PLATFORM_DEFAULT_PASSWORD".to_string(),
        "            - name: ZEBFLOW_CLUSTER_NODE_ID".to_string(),
        format!("              value: \"{}\"", office_id),
        "            - name: ZEBFLOW_CLUSTER_NODE_LABEL".to_string(),
        format!("              value: \"{}\"", office_label(office_id)),
        "            - name: ZEBFLOW_CLUSTER_ADVERTISE_URL".to_string(),
        format!(
            "              value: \"{}\"",
            office_service_url(&service_name_for_office(cfg, office_id), &cfg.namespace)
        ),
        "            - name: ZEBFLOW_CLUSTER_JOIN_TOKEN".to_string(),
        "              valueFrom:".to_string(),
        "                secretKeyRef:".to_string(),
        format!("                  name: {}", cfg.management_secret_name),
        "                  key: ZEBFLOW_CLUSTER_JOIN_TOKEN".to_string(),
    ];
    if office_id != cfg.controller_office_id {
        lines.extend_from_slice(&[
            "            - name: ZEBFLOW_CLUSTER_MASTER_URL".to_string(),
            "              valueFrom:".to_string(),
            "                configMapKeyRef:".to_string(),
            format!("                  name: {}", management_configmap_name(cfg)),
            "                  key: CONTROLLER_URL".to_string(),
        ]);
    }
    lines.join("\n") + "\n"
}

fn render_auto_update_yaml(cfg: &ClusterConfig) -> String {
    let updater_name = resource_name("zebflow-auto-updater", &cfg.resource_suffix);
    let script_name = resource_name("zebflow-auto-updater-script", &cfg.resource_suffix);
    format!(
        "apiVersion: v1\nkind: ServiceAccount\nmetadata:\n  name: {updater_name}\n  namespace: {namespace}\n---\napiVersion: rbac.authorization.k8s.io/v1\nkind: Role\nmetadata:\n  name: {updater_name}\n  namespace: {namespace}\nrules:\n  - apiGroups:\n      - apps\n    resources:\n      - statefulsets\n    resourceNames:\n      # zebflow:managed-begin {resource_block}\n{resource_names}      # zebflow:managed-end {resource_block}\n    verbs:\n      - get\n      - patch\n---\napiVersion: rbac.authorization.k8s.io/v1\nkind: RoleBinding\nmetadata:\n  name: {updater_name}\n  namespace: {namespace}\nsubjects:\n  - kind: ServiceAccount\n    name: {updater_name}\n    namespace: {namespace}\nroleRef:\n  apiGroup: rbac.authorization.k8s.io\n  kind: Role\n  name: {updater_name}\n---\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: {script_name}\n  namespace: {namespace}\ndata:\n  update.py: |\n    import json\n    import os\n    import re\n    import ssl\n    import sys\n    import urllib.request\n    from datetime import datetime, timezone\n\n\n    IMAGE_REPO = os.environ.get(\"IMAGE_REPO\", \"insanalamin/zebflow\")\n    TAG_REGEX = re.compile(os.environ.get(\"TAG_REGEX\", r\"^\\d+\\.\\d+\\.\\d+\\.\\d+$\"))\n    TARGET_NAMESPACE = os.environ.get(\"TARGET_NAMESPACE\", \"zebflow\")\n    TARGET_WORKLOADS = [\n        item.strip()\n        for item in os.environ.get(\"TARGET_WORKLOADS\", \"statefulsets/office-main\").split(\",\")\n        if item.strip()\n    ]\n    TARGET_CONTAINER = os.environ.get(\"TARGET_CONTAINER\", \"zebflow\")\n    PAGE_SIZE = int(os.environ.get(\"PAGE_SIZE\", \"25\"))\n    MAX_PAGES = int(os.environ.get(\"MAX_PAGES\", \"4\"))\n\n\n    def fetch_json(url: str, headers: dict | None = None, cafile: str | None = None) -> dict:\n        req = urllib.request.Request(url, headers=headers or {{}})\n        context = ssl.create_default_context(cafile=cafile) if cafile else None\n        with urllib.request.urlopen(req, timeout=20, context=context) as resp:\n            return json.load(resp)\n\n\n    def docker_hub_latest_numbered_tag() -> str:\n        url = (\n            f\"https://hub.docker.com/v2/repositories/{{IMAGE_REPO}}/tags/\"\n            f\"?page_size={{PAGE_SIZE}}&ordering=last_updated\"\n        )\n        pages = 0\n        while url and pages < MAX_PAGES:\n            payload = fetch_json(url)\n            for item in payload.get(\"results\", []):\n                name = str(item.get(\"name\", \"\")).strip()\n                if name == \"latest\":\n                    continue\n                if TAG_REGEX.match(name):\n                    return name\n            url = payload.get(\"next\")\n            pages += 1\n        raise RuntimeError(\"no numbered Docker Hub tag matched the configured regex\")\n\n\n    def kube_api_headers(token: str, content_type: str | None = None) -> dict:\n        headers = {{\"Authorization\": f\"Bearer {{token}}\"}}\n        if content_type:\n            headers[\"Content-Type\"] = content_type\n        return headers\n\n\n    def read_service_account_token() -> str:\n        with open(\"/var/run/secrets/kubernetes.io/serviceaccount/token\", \"r\", encoding=\"utf-8\") as fh:\n            return fh.read().strip()\n\n\n    def kubernetes_ca_file() -> str:\n        return \"/var/run/secrets/kubernetes.io/serviceaccount/ca.crt\"\n\n\n    def normalize_workload_kind(kind: str) -> str:\n        normalized = kind.strip()\n        if normalized == \"statefulset\":\n            return \"statefulsets\"\n        if normalized == \"deployment\":\n            return \"deployments\"\n        if normalized == \"daemonset\":\n            return \"daemonsets\"\n        if normalized == \"replicaset\":\n            return \"replicasets\"\n        return normalized\n\n\n    def normalize_workload_ref(workload: str) -> tuple[str, str]:\n        kind, sep, name = workload.strip().partition(\"/\")\n        if not sep or not name.strip():\n            raise RuntimeError(f\"invalid workload ref '{{workload}}'; expected <kind>/<name>\")\n        return normalize_workload_kind(kind), name.strip()\n\n\n    def workload_url(kind: str, name: str) -> str:\n        host = os.environ.get(\"KUBERNETES_SERVICE_HOST\", \"kubernetes.default.svc\")\n        port = os.environ.get(\"KUBERNETES_SERVICE_PORT_HTTPS\", \"443\")\n        return (\n            f\"https://{{host}}:{{port}}/apis/apps/v1/namespaces/\"\n            f\"{{TARGET_NAMESPACE}}/{{kind}}/{{name}}\"\n        )\n\n\n    def current_image(payload: dict) -> str:\n        containers = payload.get(\"spec\", {{}}).get(\"template\", {{}}).get(\"spec\", {{}}).get(\"containers\", [])\n        for container in containers:\n            if container.get(\"name\") == TARGET_CONTAINER:\n                return str(container.get(\"image\", \"\")).strip()\n        raise RuntimeError(f\"container {{TARGET_CONTAINER!r}} not found in workload\")\n\n\n    def patch_workload(token: str, kind: str, name: str, image_ref: str) -> None:\n        patch = {{\n            \"spec\": {{\n                \"template\": {{\n                    \"metadata\": {{\n                        \"annotations\": {{\n                            \"zebflow.auto-update/last-applied\": datetime.now(timezone.utc).isoformat()\n                        }}\n                    }},\n                    \"spec\": {{\n                        \"containers\": [\n                            {{\n                                \"name\": TARGET_CONTAINER,\n                                \"image\": image_ref,\n                            }}\n                        ]\n                    }},\n                }}\n            }}\n        }}\n        req = urllib.request.Request(\n            workload_url(kind, name),\n            data=json.dumps(patch).encode(\"utf-8\"),\n            headers=kube_api_headers(token, \"application/strategic-merge-patch+json\"),\n            method=\"PATCH\",\n        )\n        context = ssl.create_default_context(cafile=kubernetes_ca_file())\n        with urllib.request.urlopen(req, timeout=20, context=context) as resp:\n            if resp.status < 200 or resp.status >= 300:\n                raise RuntimeError(f\"workload patch failed with HTTP {{resp.status}}\")\n\n\n    def main() -> int:\n        latest_tag = docker_hub_latest_numbered_tag()\n        desired_image = f\"{{IMAGE_REPO}}:{{latest_tag}}\"\n        token = read_service_account_token()\n        changed = False\n\n        for workload in TARGET_WORKLOADS:\n            kind, name = normalize_workload_ref(workload)\n            normalized_workload = f\"{{kind}}/{{name}}\"\n            payload = fetch_json(workload_url(kind, name), kube_api_headers(token), cafile=kubernetes_ca_file())\n            current = current_image(payload)\n            print(f\"{{normalized_workload}}: current image {{current}}\")\n            if current == desired_image:\n                continue\n            patch_workload(token, kind, name, desired_image)\n            print(f\"{{normalized_workload}}: patched to {{desired_image}}\")\n            changed = True\n\n        if not changed:\n            print(\"all managed Zebflow workloads are already up to date\")\n        return 0\n\n\n    if __name__ == \"__main__\":\n        try:\n            raise SystemExit(main())\n        except Exception as exc:\n            print(f\"auto-update failed: {{exc}}\", file=sys.stderr)\n            raise\n---\napiVersion: batch/v1\nkind: CronJob\nmetadata:\n  name: {updater_name}\n  namespace: {namespace}\nspec:\n  schedule: \"{schedule}\"\n  # zebflow:managed-begin {suspend_block}\n{suspend_value}  # zebflow:managed-end {suspend_block}\n  concurrencyPolicy: Forbid\n  successfulJobsHistoryLimit: 2\n  failedJobsHistoryLimit: 2\n  jobTemplate:\n    spec:\n      template:\n        spec:\n          serviceAccountName: {updater_name}\n          restartPolicy: OnFailure\n          tolerations:\n            - effect: NoSchedule\n              key: node-role.kubernetes.io/control-plane\n              operator: Exists\n          securityContext:\n            runAsNonRoot: true\n            runAsUser: 65532\n            runAsGroup: 65532\n            seccompProfile:\n              type: RuntimeDefault\n          containers:\n            - name: updater\n              image: python:3.12-slim\n              imagePullPolicy: IfNotPresent\n              command:\n                - python\n                - /opt/zebflow-auto-update/update.py\n              env:\n                - name: PYTHONDONTWRITEBYTECODE\n                  value: \"1\"\n                - name: IMAGE_REPO\n                  # zebflow:managed-begin {image_block}\n{image_repo_value}                  # zebflow:managed-end {image_block}\n                - name: TAG_REGEX\n                  value: \"^[0-9]+\\\\.[0-9]+\\\\.[0-9]+\\\\.[0-9]{{12}}$\"\n                - name: TARGET_NAMESPACE\n                  value: \"{namespace}\"\n                - name: TARGET_WORKLOADS\n                  # zebflow:managed-begin {targets_block}\n{targets_value}                  # zebflow:managed-end {targets_block}\n                - name: TARGET_CONTAINER\n                  value: \"zebflow\"\n              volumeMounts:\n                - name: script\n                  mountPath: /opt/zebflow-auto-update\n                  readOnly: true\n              resources:\n                requests:\n                  cpu: \"25m\"\n                  memory: \"64Mi\"\n                limits:\n                  cpu: \"100m\"\n                  memory: \"128Mi\"\n              securityContext:\n                allowPrivilegeEscalation: false\n                capabilities:\n                  drop:\n                    - ALL\n          volumes:\n            - name: script\n              configMap:\n                name: {script_name}\n                defaultMode: 0555\n",
        updater_name = updater_name,
        script_name = script_name,
        namespace = cfg.namespace,
        resource_block = BLOCK_AUTOUPDATE_RESOURCE_NAMES,
        resource_names = render_auto_update_resource_names_block(cfg),
        schedule = DEFAULT_SCHEDULE,
        suspend_block = BLOCK_AUTOUPDATE_SUSPEND,
        suspend_value = render_auto_update_suspend_block(cfg),
        image_block = BLOCK_AUTOUPDATE_IMAGE_REPO,
        image_repo_value = render_auto_update_image_repo_block(cfg),
        targets_block = BLOCK_AUTOUPDATE_TARGETS,
        targets_value = render_auto_update_targets_block(cfg),
    )
}

fn render_auto_update_resource_names_block(cfg: &ClusterConfig) -> String {
    cfg.managed_workloads
        .iter()
        .filter_map(|workload| workload_name_from_ref(workload))
        .map(|workload_name| format!("      - {workload_name}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn render_auto_update_suspend_block(cfg: &ClusterConfig) -> String {
    format!(
        "  suspend: {}\n",
        if cfg.auto_update_enabled {
            "false"
        } else {
            "true"
        }
    )
}

fn render_auto_update_targets_block(cfg: &ClusterConfig) -> String {
    format!(
        "                  value: \"{}\"\n",
        cfg.managed_workloads.join(",")
    )
}

fn render_auto_update_image_repo_block(cfg: &ClusterConfig) -> String {
    format!(
        "                  value: \"{}\"\n",
        image_repo(&cfg.managed_image)
    )
}

fn write_text(path: &Path, content: &str) -> Result<(), io::Error> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        )
    })?;
    let temp_path = parent.join(format!(".{file_name}.tmp"));
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

fn default_image() -> String {
    format!("insanalamin/zebflow:{APP_VERSION}")
}

fn managed_workload_ref(office_id: &str) -> String {
    format!("statefulsets/{office_id}")
}

fn managed_workload_ref_for(cfg: &ClusterConfig, office_id: &str) -> String {
    format!(
        "statefulsets/{}",
        resource_name(office_id, &cfg.resource_suffix)
    )
}

fn workload_ref_for_office(cfg: &ClusterConfig, office_id: &str) -> String {
    cfg.managed_offices
        .iter()
        .position(|value| value == office_id)
        .and_then(|index| cfg.managed_workloads.get(index))
        .cloned()
        .unwrap_or_else(|| managed_workload_ref_for(cfg, office_id))
}

fn workload_name_for_office(cfg: &ClusterConfig, office_id: &str) -> String {
    workload_name_from_ref(&workload_ref_for_office(cfg, office_id))
        .unwrap_or_else(|| resource_name(office_id, &cfg.resource_suffix))
}

fn service_name_for_office(cfg: &ClusterConfig, office_id: &str) -> String {
    resource_name(office_id, &cfg.resource_suffix)
}

fn management_configmap_name(cfg: &ClusterConfig) -> String {
    resource_name(MANAGEMENT_CONFIGMAP_NAME, &cfg.resource_suffix)
}

fn resource_name(base: &str, suffix: &str) -> String {
    let suffix = suffix.trim();
    if suffix.is_empty() {
        base.to_string()
    } else {
        format!("{base}-{suffix}")
    }
}

fn workload_name_from_ref(workload: &str) -> Option<String> {
    workload
        .split_once('/')
        .map(|(_, name)| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

fn data_pvc_name(workload_name: &str) -> String {
    format!("data-{workload_name}-0")
}

fn office_storage(office_id: &str) -> &'static str {
    if office_id == DEFAULT_CONTROLLER_ID {
        "50Gi"
    } else {
        "10Gi"
    }
}

fn normalize_workload_ref(workload: &str) -> String {
    let trimmed = workload.trim();
    let Some((kind, name)) = trimmed.split_once('/') else {
        return trimmed.to_string();
    };
    format!("{}/{}", normalize_workload_kind(kind), name.trim())
}

fn normalize_workload_kind(kind: &str) -> String {
    match kind.trim() {
        "statefulset" | "statefulsets" => "statefulsets".to_string(),
        "deployment" | "deployments" => "deployments".to_string(),
        "daemonset" | "daemonsets" => "daemonsets".to_string(),
        "replicaset" | "replicasets" => "replicasets".to_string(),
        other => other.to_string(),
    }
}

fn office_service_url(office_id: &str, namespace: &str) -> String {
    format!("http://{office_id}.{namespace}.svc.cluster.local:10610")
}

fn image_repo(image: &str) -> String {
    if let Some((repo, _)) = image.rsplit_once(':') {
        if !repo.contains('/') && !image.contains('/') {
            return image.to_string();
        }
        if repo.contains('@') {
            image.to_string()
        } else {
            repo.to_string()
        }
    } else if let Some((repo, _)) = image.rsplit_once('@') {
        repo.to_string()
    } else {
        image.to_string()
    }
}

fn office_label(office_id: &str) -> String {
    office_id
        .split(['-', '_'])
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_volume_permission_strategy(
    value: &str,
) -> Result<VolumePermissionStrategy, Box<dyn std::error::Error>> {
    match value.trim() {
        "fs-group" => Ok(VolumePermissionStrategy::FsGroup),
        "init-chmod" => Ok(VolumePermissionStrategy::InitChmod),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported volume permission strategy '{}'", other),
        )
        .into()),
    }
}

impl VolumePermissionStrategy {
    fn as_str(self) -> &'static str {
        match self {
            VolumePermissionStrategy::FsGroup => "fs-group",
            VolumePermissionStrategy::InitChmod => "init-chmod",
        }
    }
}

struct ClusterLockGuard {
    path: PathBuf,
}

impl Drop for ClusterLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn with_cluster_lock<T, F>(root: &Path, action: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let _guard = acquire_cluster_lock(root)?;
    action()
}

fn acquire_cluster_lock(root: &Path) -> Result<ClusterLockGuard, Box<dyn std::error::Error>> {
    let lock_path = root.join(CLUSTER_LOCK_FILE);
    let deadline = Instant::now() + LOCK_WAIT_TIMEOUT;
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => return Ok(ClusterLockGuard { path: lock_path }),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("timed out waiting for cluster lock {}", lock_path.display()),
                    )
                    .into());
                }
                thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(err) => return Err(err.into()),
        }
    }
}

fn validate_office_id(office_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = office_id.trim();
    if trimmed.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "office id cannot be empty").into(),
        );
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "office id '{}' is invalid; use lowercase letters, digits, and '-' only",
                office_id
            ),
        )
        .into());
    }
    Ok(())
}

fn validate_resource_name(value: &str, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} cannot be empty"),
        )
        .into());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} '{value}' is invalid; use lowercase letters, digits, and '-' only"),
        )
        .into());
    }
    Ok(())
}

fn replace_managed_block(
    content: &str,
    block_name: &str,
    replacement: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let begin_marker = format!("# zebflow:managed-begin {block_name}");
    let end_marker = format!("# zebflow:managed-end {block_name}");
    let mut output = Vec::new();
    let mut in_block = false;
    let mut replaced = false;

    for line in content.lines() {
        if line.contains(&begin_marker) {
            in_block = true;
            replaced = true;
            output.push(line.to_string());
            output.extend(replacement.lines().map(str::to_string));
            continue;
        }
        if line.contains(&end_marker) {
            in_block = false;
            output.push(line.to_string());
            continue;
        }
        if !in_block {
            output.push(line.to_string());
        }
    }

    if !replaced {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("managed block '{}' not found", block_name),
        )
        .into());
    }

    Ok(output.join("\n") + "\n")
}

fn extract_managed_block(
    content: &str,
    block_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let begin_marker = format!("# zebflow:managed-begin {block_name}");
    let end_marker = format!("# zebflow:managed-end {block_name}");
    let mut lines = Vec::new();
    let mut in_block = false;
    let mut found = false;

    for line in content.lines() {
        if line.contains(&begin_marker) {
            in_block = true;
            found = true;
            continue;
        }
        if line.contains(&end_marker) {
            break;
        }
        if in_block {
            lines.push(line.to_string());
        }
    }

    if !found {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("managed block '{}' not found", block_name),
        )
        .into());
    }
    Ok(lines.join("\n"))
}

fn ensure_block_present(content: &str, block_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let begin_marker = format!("# zebflow:managed-begin {block_name}");
    let end_marker = format!("# zebflow:managed-end {block_name}");
    if !content.contains(&begin_marker) || !content.contains(&end_marker) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("managed block '{}' is missing", block_name),
        )
        .into());
    }
    Ok(())
}

fn parse_config_block(
    block: &str,
) -> Result<std::collections::BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut values = std::collections::BTreeMap::new();
    for raw in block.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid management config line '{}'", raw),
            )
            .into());
        };
        values.insert(
            key.trim().to_string(),
            value.trim().trim_matches('"').to_string(),
        );
    }
    Ok(values)
}

fn csv_values(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn ensure_contains(
    content: &str,
    needle: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !content.contains(needle) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} is missing required content '{}'",
                path.display(),
                needle
            ),
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_cluster_dir() -> PathBuf {
        let root = std::env::temp_dir().join(format!("zebflow-k8s-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp dir");
        root
    }

    #[test]
    fn init_and_add_office_updates_management_and_auto_update() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");
        add_office(&root, "office-a").expect("add office");

        let management = fs::read_to_string(root.join(MANAGEMENT_FILE)).expect("management");
        assert!(
            management.contains("MANAGED_OFFICES: \"office-a,office-main\"")
                || management.contains("MANAGED_OFFICES: \"office-main,office-a\"")
        );
        let auto_update = fs::read_to_string(root.join(AUTO_UPDATE_FILE)).expect("auto-update");
        assert!(auto_update.contains("- office-a"));
        assert!(
            auto_update.contains("value: \"statefulsets/office-main,statefulsets/office-a\"")
                || auto_update
                    .contains("value: \"statefulsets/office-a,statefulsets/office-main\"")
        );
    }

    #[test]
    fn set_controller_rewrites_office_modes() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");
        add_office(&root, "office-a").expect("add office");
        set_controller(&root, "office-a").expect("set controller");

        let main = fs::read_to_string(root.join("office-main.yaml")).expect("main");
        let office_a = fs::read_to_string(root.join("office-a.yaml")).expect("office-a");
        assert!(main.contains("args:\n            - office"));
        assert!(main.contains("name: ZEBFLOW_CLUSTER_MASTER_URL"));
        assert!(office_a.contains("args:\n            - controller"));
        assert!(!office_a.contains("name: ZEBFLOW_CLUSTER_MASTER_URL"));
    }

    #[test]
    fn set_image_and_disable_auto_update_touch_disk_state() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");
        set_image(&root, "example.com/zebflow:test").expect("set image");
        set_auto_update(&root, false).expect("disable");

        let office = fs::read_to_string(root.join("office-main.yaml")).expect("office");
        assert!(office.contains("image: example.com/zebflow:test"));
        let auto_update = fs::read_to_string(root.join(AUTO_UPDATE_FILE)).expect("auto-update");
        assert!(auto_update.contains("suspend: true"));
        assert!(auto_update.contains("value: \"example.com/zebflow\""));
    }

    #[test]
    fn legacy_singular_workload_refs_are_normalized_on_write() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");

        let management_path = root.join(MANAGEMENT_FILE);
        let management = fs::read_to_string(&management_path).expect("management");
        fs::write(
            &management_path,
            management.replace("statefulsets/office-main", "statefulset/office-main"),
        )
        .expect("rewrite legacy singular ref");

        set_auto_update(&root, true).expect("enable");

        let management = fs::read_to_string(&management_path).expect("management");
        assert!(management.contains("MANAGED_WORKLOADS: \"statefulsets/office-main\""));
        assert!(!management.contains("MANAGED_WORKLOADS: \"statefulset/office-main\""));

        let auto_update = fs::read_to_string(root.join(AUTO_UPDATE_FILE)).expect("auto-update");
        assert!(auto_update.contains("value: \"statefulsets/office-main\""));
        assert!(!auto_update.contains("value: \"statefulset/office-main\""));
    }

    #[test]
    fn office_manifests_use_health_and_ready_probes() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");

        let office = fs::read_to_string(root.join("office-main.yaml")).expect("office");
        assert!(office.contains("path: /ready"));
        assert!(office.contains("path: /health"));
        assert!(!office.contains("path: /login"));
    }

    #[test]
    fn office_manifests_default_to_fs_group_without_root_chmod_init() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");

        let office = fs::read_to_string(root.join("office-main.yaml")).expect("office");
        assert!(office.contains("fsGroup: 1000"));
        assert!(office.contains("fsGroupChangePolicy: OnRootMismatch"));
        assert!(!office.contains("name: volume-permissions"));
        assert!(!office.contains("chmod -R 0777"));

        let management = fs::read_to_string(root.join(MANAGEMENT_FILE)).expect("management");
        assert!(management.contains("VOLUME_PERMISSION_STRATEGY: \"fs-group\""));
    }

    #[test]
    fn legacy_init_chmod_strategy_is_preserved_when_present() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");

        let management_path = root.join(MANAGEMENT_FILE);
        let management = fs::read_to_string(&management_path).expect("management");
        fs::write(
            &management_path,
            management.replace(
                "VOLUME_PERMISSION_STRATEGY: \"fs-group\"",
                "VOLUME_PERMISSION_STRATEGY: \"init-chmod\"",
            ),
        )
        .expect("rewrite strategy");

        set_auto_update(&root, true).expect("rewrite manifests");

        let office = fs::read_to_string(root.join("office-main.yaml")).expect("office");
        assert!(office.contains("name: volume-permissions"));
        assert!(office.contains("chmod -R 0777"));
        assert!(!office.contains("fsGroupChangePolicy: OnRootMismatch"));
    }

    #[test]
    fn resource_suffix_renders_parallel_workloads_without_managed_secret() {
        let root = temp_cluster_dir();
        init_cluster(&root).expect("init");
        add_office(&root, "hadaf-id").expect("add office");
        set_namespace(&root, "main-app-cluster").expect("namespace");
        set_resource_suffix(&root, "v6").expect("suffix");
        use_external_secret(&root, "zebflow-management-secrets").expect("external secret");
        set_replicas(&root, 0).expect("replicas");
        set_precreate_pvcs(&root, true).expect("precreate pvcs");
        validate_cluster(&root).expect("valid");

        let management = fs::read_to_string(root.join(MANAGEMENT_FILE)).expect("management");
        assert!(management.contains("name: zebflow-management-v6"));
        assert!(!management.contains("kind: Secret"));
        assert!(management.contains("MANAGEMENT_SECRET_NAME: \"zebflow-management-secrets\""));
        assert!(management.contains("MANAGE_SECRET: \"false\""));
        assert!(management.contains("REPLICAS: \"0\""));
        assert!(management.contains("PRECREATE_PVCS: \"true\""));
        assert!(management.contains(
            "MANAGED_WORKLOADS: \"statefulsets/hadaf-id-v6,statefulsets/office-main-v6\""
        ));

        let office = fs::read_to_string(root.join("hadaf-id.yaml")).expect("office");
        assert!(office.contains("name: hadaf-id-v6"));
        assert!(office.contains("name: data-hadaf-id-v6-0"));
        assert!(office.contains("replicas: 0"));
        assert!(office.contains("name: zebflow-management-v6"));
        assert!(office.contains("name: zebflow-management-secrets"));
        assert!(office.contains("value: \"hadaf-id\""));

        let auto_update = fs::read_to_string(root.join(AUTO_UPDATE_FILE)).expect("auto-update");
        assert!(auto_update.contains("name: zebflow-auto-updater-v6"));
        assert!(auto_update.contains("- hadaf-id-v6"));
        assert!(
            auto_update.contains("value: \"statefulsets/hadaf-id-v6,statefulsets/office-main-v6\"")
        );
    }

    #[test]
    fn copy_jobs_use_source_and_target_workload_pvc_names() {
        let source = temp_cluster_dir();
        init_cluster(&source).expect("source init");
        add_office(&source, "zebflow-site-v4").expect("source office");
        set_namespace(&source, "main-app-cluster").expect("source namespace");
        let source_management_path = source.join(MANAGEMENT_FILE);
        let source_management =
            fs::read_to_string(&source_management_path).expect("source management");
        fs::write(
            &source_management_path,
            source_management.replace(
                "MANAGED_WORKLOADS: \"statefulsets/office-main,statefulsets/zebflow-site-v4\"",
                "MANAGED_WORKLOADS: \"statefulsets/office-main-v4,statefulsets/zebflow-site-v4\"",
            ),
        )
        .expect("rewrite source workload names");

        let target = temp_cluster_dir();
        init_cluster(&target).expect("target init");
        add_office(&target, "zebflow-site-v4").expect("target office");
        set_namespace(&target, "main-app-cluster").expect("target namespace");
        set_resource_suffix(&target, "v6").expect("target suffix");

        let jobs = render_copy_jobs_yaml(
            &load_cluster_config(&source).expect("source config"),
            &load_cluster_config(&target).expect("target config"),
        )
        .expect("copy jobs");

        assert!(jobs.contains("name: migrate-zebflow-site-v4-to-zebflow-site-v4-v6"));
        assert!(jobs.contains("claimName: data-zebflow-site-v4-0"));
        assert!(jobs.contains("claimName: data-zebflow-site-v4-v6-0"));
    }
}
