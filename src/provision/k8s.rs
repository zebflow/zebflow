//! File-based Kubernetes cluster layout manager for Zebflow.
//!
//! This module intentionally manages files on disk first. It does not apply anything to a live
//! cluster. The CLI mutates a small set of YAML manifests and only rewrites the blocks explicitly
//! marked as Zebflow-owned.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClusterConfig {
    cluster_name: String,
    namespace: String,
    controller_office_id: String,
    managed_image: String,
    managed_offices: Vec<String>,
    managed_workloads: Vec<String>,
    auto_update_enabled: bool,
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
        ("set-image", [path, image]) => {
            set_image(Path::new(path), image)?;
            println!(
                "set managed image to '{}' in {}",
                image,
                Path::new(path).display()
            );
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
  cluster set-image <path> <image>
  cluster enable-auto-update <path>
  cluster disable-auto-update <path>
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
  zebflow k8s cluster set-image <path> <image>
  zebflow k8s cluster enable-auto-update <path>
  zebflow k8s cluster disable-auto-update <path>
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
    };

    write_text(&management_path, &render_management_yaml(&cfg))?;
    write_text(
        &root.join(DEFAULT_CONTROLLER_ID.to_string() + ".yaml"),
        &render_office_yaml(&cfg, DEFAULT_CONTROLLER_ID),
    )?;
    write_text(&root.join(AUTO_UPDATE_FILE), &render_auto_update_yaml(&cfg))?;
    Ok(())
}

fn add_office(root: &Path, office_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_office_id(office_id)?;
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
    cfg.managed_workloads.push(managed_workload_ref(office_id));
    cfg.managed_offices.sort();
    cfg.managed_workloads.sort();

    write_text(&office_path, &render_office_yaml(&cfg, office_id))?;
    save_cluster_config(root, &cfg)?;
    refresh_auto_update(root, &cfg)?;
    Ok(())
}

fn set_controller(root: &Path, office_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_office_id(office_id)?;
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
    for item in &cfg.managed_offices {
        let path = root.join(format!("{item}.yaml"));
        let content = fs::read_to_string(&path)?;
        let next = replace_managed_block(
            &content,
            BLOCK_OFFICE_CONTAINER,
            &render_office_container_block(&cfg, item),
        )?;
        write_text(&path, &next)?;
    }
    Ok(())
}

fn set_image(root: &Path, image: &str) -> Result<(), Box<dyn std::error::Error>> {
    if image.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "image cannot be empty").into());
    }
    let mut cfg = load_cluster_config(root)?;
    cfg.managed_image = image.trim().to_string();
    save_cluster_config(root, &cfg)?;
    for item in &cfg.managed_offices {
        let path = root.join(format!("{item}.yaml"));
        let content = fs::read_to_string(&path)?;
        let next = replace_managed_block(
            &content,
            BLOCK_OFFICE_CONTAINER,
            &render_office_container_block(&cfg, item),
        )?;
        write_text(&path, &next)?;
    }
    refresh_auto_update(root, &cfg)?;
    Ok(())
}

fn set_auto_update(root: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_cluster_config(root)?;
    cfg.auto_update_enabled = enabled;
    save_cluster_config(root, &cfg)?;
    refresh_auto_update(root, &cfg)?;
    Ok(())
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

    for office_id in &cfg.managed_offices {
        ensure_contains(
            &auto_update,
            &format!("- {office_id}"),
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
    })
}

fn render_management_yaml(cfg: &ClusterConfig) -> String {
    format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {namespace}\n---\napiVersion: v1\nkind: Secret\nmetadata:\n  name: {secret}\n  namespace: {namespace}\ntype: Opaque\nstringData:\n  ZEBFLOW_PLATFORM_DEFAULT_PASSWORD: \"CHANGE_ME_TO_A_REAL_PASSWORD\"\n  ZEBFLOW_CLUSTER_JOIN_TOKEN: \"CHANGE_ME_TO_A_REAL_JOIN_TOKEN\"\n---\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: {configmap}\n  namespace: {namespace}\n  annotations:\n    zebflow.io/managed-by: zebflow\ndata:\n  # zebflow:managed-begin {block}\n{config_block}  # zebflow:managed-end {block}\n",
        namespace = cfg.namespace,
        secret = MANAGEMENT_SECRET_NAME,
        configmap = MANAGEMENT_CONFIGMAP_NAME,
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
        managed_workload_ref(&cfg.controller_office_id)
    ));
    lines.push(format!(
        "  CONTROLLER_URL: \"{}\"",
        office_service_url(&cfg.controller_office_id, &cfg.namespace)
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
    lines.push(format!(
        "  AUTO_UPDATE_ENABLED: \"{}\"",
        if cfg.auto_update_enabled {
            "true"
        } else {
            "false"
        }
    ));
    lines.join("\n") + "\n"
}

fn render_office_yaml(cfg: &ClusterConfig, office_id: &str) -> String {
    format!(
        "apiVersion: v1\nkind: Service\nmetadata:\n  name: {office_id}\n  namespace: {namespace}\n  labels:\n    app: {office_id}\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  selector:\n    app: {office_id}\n  ports:\n    - name: http\n      port: 10610\n      targetPort: http\n---\napiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: {office_id}\n  namespace: {namespace}\n  labels:\n    app: {office_id}\n    zebflow.io/managed-by: zebflow\n    zebflow.io/office-id: {office_id}\nspec:\n  serviceName: {office_id}\n  replicas: 1\n  selector:\n    matchLabels:\n      app: {office_id}\n  template:\n    metadata:\n      labels:\n        app: {office_id}\n        zebflow.io/managed-by: zebflow\n        zebflow.io/office-id: {office_id}\n    spec:\n      automountServiceAccountToken: false\n      tolerations:\n        - key: node-role.kubernetes.io/control-plane\n          operator: Exists\n          effect: NoSchedule\n      initContainers:\n        - name: volume-permissions\n          image: busybox:1.36\n          imagePullPolicy: IfNotPresent\n          command:\n            - sh\n            - -c\n            - mkdir -p /var/lib/zebflow/data && chmod -R 0777 /var/lib/zebflow/data\n          securityContext:\n            runAsUser: 0\n          volumeMounts:\n            - name: data\n              mountPath: /var/lib/zebflow/data\n      containers:\n        - name: zebflow\n          # zebflow:managed-begin {block}\n{container_block}          # zebflow:managed-end {block}\n          ports:\n            - name: http\n              containerPort: 10610\n              protocol: TCP\n          readinessProbe:\n            httpGet:\n              path: /login\n              port: http\n            initialDelaySeconds: 10\n            periodSeconds: 5\n            timeoutSeconds: 3\n            failureThreshold: 6\n          livenessProbe:\n            httpGet:\n              path: /login\n              port: http\n            initialDelaySeconds: 30\n            periodSeconds: 10\n            timeoutSeconds: 5\n            failureThreshold: 6\n          resources:\n            requests:\n              cpu: \"100m\"\n              memory: \"256Mi\"\n            limits:\n              cpu: \"1000m\"\n              memory: \"1Gi\"\n          securityContext:\n            allowPrivilegeEscalation: false\n            capabilities:\n              drop:\n                - ALL\n            readOnlyRootFilesystem: false\n            seccompProfile:\n              type: RuntimeDefault\n          volumeMounts:\n            - name: data\n              mountPath: /var/lib/zebflow/data\n  volumeClaimTemplates:\n    - metadata:\n        name: data\n      spec:\n        accessModes:\n          - ReadWriteOnce\n        storageClassName: local-path\n        resources:\n          requests:\n            storage: {storage}\n",
        namespace = cfg.namespace,
        office_id = office_id,
        block = BLOCK_OFFICE_CONTAINER,
        container_block = render_office_container_block(cfg, office_id),
        storage = if office_id == DEFAULT_CONTROLLER_ID {
            "50Gi"
        } else {
            "10Gi"
        }
    )
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
        "            - name: ZEBFLOW_PLATFORM_DEFAULT_PASSWORD".to_string(),
        "              valueFrom:".to_string(),
        "                secretKeyRef:".to_string(),
        format!("                  name: {}", MANAGEMENT_SECRET_NAME),
        "                  key: ZEBFLOW_PLATFORM_DEFAULT_PASSWORD".to_string(),
        "            - name: ZEBFLOW_CLUSTER_NODE_ID".to_string(),
        format!("              value: \"{}\"", office_id),
        "            - name: ZEBFLOW_CLUSTER_NODE_LABEL".to_string(),
        format!("              value: \"{}\"", office_label(office_id)),
        "            - name: ZEBFLOW_CLUSTER_ADVERTISE_URL".to_string(),
        format!(
            "              value: \"{}\"",
            office_service_url(office_id, &cfg.namespace)
        ),
        "            - name: ZEBFLOW_CLUSTER_JOIN_TOKEN".to_string(),
        "              valueFrom:".to_string(),
        "                secretKeyRef:".to_string(),
        format!("                  name: {}", MANAGEMENT_SECRET_NAME),
        "                  key: ZEBFLOW_CLUSTER_JOIN_TOKEN".to_string(),
    ];
    if office_id != cfg.controller_office_id {
        lines.extend_from_slice(&[
            "            - name: ZEBFLOW_CLUSTER_MASTER_URL".to_string(),
            "              valueFrom:".to_string(),
            "                configMapKeyRef:".to_string(),
            format!("                  name: {}", MANAGEMENT_CONFIGMAP_NAME),
            "                  key: CONTROLLER_URL".to_string(),
        ]);
    }
    lines.join("\n") + "\n"
}

fn render_auto_update_yaml(cfg: &ClusterConfig) -> String {
    format!(
        "apiVersion: v1\nkind: ServiceAccount\nmetadata:\n  name: zebflow-auto-updater\n  namespace: {namespace}\n---\napiVersion: rbac.authorization.k8s.io/v1\nkind: Role\nmetadata:\n  name: zebflow-auto-updater\n  namespace: {namespace}\nrules:\n  - apiGroups:\n      - apps\n    resources:\n      - statefulsets\n    resourceNames:\n      # zebflow:managed-begin {resource_block}\n{resource_names}      # zebflow:managed-end {resource_block}\n    verbs:\n      - get\n      - patch\n---\napiVersion: rbac.authorization.k8s.io/v1\nkind: RoleBinding\nmetadata:\n  name: zebflow-auto-updater\n  namespace: {namespace}\nsubjects:\n  - kind: ServiceAccount\n    name: zebflow-auto-updater\n    namespace: {namespace}\nroleRef:\n  apiGroup: rbac.authorization.k8s.io\n  kind: Role\n  name: zebflow-auto-updater\n---\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: zebflow-auto-updater-script\n  namespace: {namespace}\ndata:\n  update.py: |\n    import json\n    import os\n    import re\n    import ssl\n    import sys\n    import urllib.request\n    from datetime import datetime, timezone\n\n\n    IMAGE_REPO = os.environ.get(\"IMAGE_REPO\", \"insanalamin/zebflow\")\n    TAG_REGEX = re.compile(os.environ.get(\"TAG_REGEX\", r\"^\\d+\\.\\d+\\.\\d+\\.\\d+$\"))\n    TARGET_NAMESPACE = os.environ.get(\"TARGET_NAMESPACE\", \"zebflow\")\n    TARGET_WORKLOADS = [\n        item.strip()\n        for item in os.environ.get(\"TARGET_WORKLOADS\", \"statefulsets/office-main\").split(\",\")\n        if item.strip()\n    ]\n    TARGET_CONTAINER = os.environ.get(\"TARGET_CONTAINER\", \"zebflow\")\n    PAGE_SIZE = int(os.environ.get(\"PAGE_SIZE\", \"25\"))\n    MAX_PAGES = int(os.environ.get(\"MAX_PAGES\", \"4\"))\n\n\n    def fetch_json(url: str, headers: dict | None = None, cafile: str | None = None) -> dict:\n        req = urllib.request.Request(url, headers=headers or {{}})\n        context = ssl.create_default_context(cafile=cafile) if cafile else None\n        with urllib.request.urlopen(req, timeout=20, context=context) as resp:\n            return json.load(resp)\n\n\n    def docker_hub_latest_numbered_tag() -> str:\n        url = (\n            f\"https://hub.docker.com/v2/repositories/{{IMAGE_REPO}}/tags/\"\n            f\"?page_size={{PAGE_SIZE}}&ordering=last_updated\"\n        )\n        pages = 0\n        while url and pages < MAX_PAGES:\n            payload = fetch_json(url)\n            for item in payload.get(\"results\", []):\n                name = str(item.get(\"name\", \"\")).strip()\n                if name == \"latest\":\n                    continue\n                if TAG_REGEX.match(name):\n                    return name\n            url = payload.get(\"next\")\n            pages += 1\n        raise RuntimeError(\"no numbered Docker Hub tag matched the configured regex\")\n\n\n    def kube_api_headers(token: str, content_type: str | None = None) -> dict:\n        headers = {{\"Authorization\": f\"Bearer {{token}}\"}}\n        if content_type:\n            headers[\"Content-Type\"] = content_type\n        return headers\n\n\n    def read_service_account_token() -> str:\n        with open(\"/var/run/secrets/kubernetes.io/serviceaccount/token\", \"r\", encoding=\"utf-8\") as fh:\n            return fh.read().strip()\n\n\n    def kubernetes_ca_file() -> str:\n        return \"/var/run/secrets/kubernetes.io/serviceaccount/ca.crt\"\n\n\n    def normalize_workload_kind(kind: str) -> str:\n        normalized = kind.strip()\n        if normalized == \"statefulset\":\n            return \"statefulsets\"\n        if normalized == \"deployment\":\n            return \"deployments\"\n        if normalized == \"daemonset\":\n            return \"daemonsets\"\n        if normalized == \"replicaset\":\n            return \"replicasets\"\n        return normalized\n\n\n    def normalize_workload_ref(workload: str) -> tuple[str, str]:\n        kind, sep, name = workload.strip().partition(\"/\")\n        if not sep or not name.strip():\n            raise RuntimeError(f\"invalid workload ref '{{workload}}'; expected <kind>/<name>\")\n        return normalize_workload_kind(kind), name.strip()\n\n\n    def workload_url(kind: str, name: str) -> str:\n        host = os.environ.get(\"KUBERNETES_SERVICE_HOST\", \"kubernetes.default.svc\")\n        port = os.environ.get(\"KUBERNETES_SERVICE_PORT_HTTPS\", \"443\")\n        return (\n            f\"https://{{host}}:{{port}}/apis/apps/v1/namespaces/\"\n            f\"{{TARGET_NAMESPACE}}/{{kind}}/{{name}}\"\n        )\n\n\n    def current_image(payload: dict) -> str:\n        containers = payload.get(\"spec\", {{}}).get(\"template\", {{}}).get(\"spec\", {{}}).get(\"containers\", [])\n        for container in containers:\n            if container.get(\"name\") == TARGET_CONTAINER:\n                return str(container.get(\"image\", \"\")).strip()\n        raise RuntimeError(f\"container {{TARGET_CONTAINER!r}} not found in workload\")\n\n\n    def patch_workload(token: str, kind: str, name: str, image_ref: str) -> None:\n        patch = {{\n            \"spec\": {{\n                \"template\": {{\n                    \"metadata\": {{\n                        \"annotations\": {{\n                            \"zebflow.auto-update/last-applied\": datetime.now(timezone.utc).isoformat()\n                        }}\n                    }},\n                    \"spec\": {{\n                        \"containers\": [\n                            {{\n                                \"name\": TARGET_CONTAINER,\n                                \"image\": image_ref,\n                            }}\n                        ]\n                    }},\n                }}\n            }}\n        }}\n        req = urllib.request.Request(\n            workload_url(kind, name),\n            data=json.dumps(patch).encode(\"utf-8\"),\n            headers=kube_api_headers(token, \"application/strategic-merge-patch+json\"),\n            method=\"PATCH\",\n        )\n        context = ssl.create_default_context(cafile=kubernetes_ca_file())\n        with urllib.request.urlopen(req, timeout=20, context=context) as resp:\n            if resp.status < 200 or resp.status >= 300:\n                raise RuntimeError(f\"workload patch failed with HTTP {{resp.status}}\")\n\n\n    def main() -> int:\n        latest_tag = docker_hub_latest_numbered_tag()\n        desired_image = f\"{{IMAGE_REPO}}:{{latest_tag}}\"\n        token = read_service_account_token()\n        changed = False\n\n        for workload in TARGET_WORKLOADS:\n            kind, name = normalize_workload_ref(workload)\n            normalized_workload = f\"{{kind}}/{{name}}\"\n            payload = fetch_json(workload_url(kind, name), kube_api_headers(token), cafile=kubernetes_ca_file())\n            current = current_image(payload)\n            print(f\"{{normalized_workload}}: current image {{current}}\")\n            if current == desired_image:\n                continue\n            patch_workload(token, kind, name, desired_image)\n            print(f\"{{normalized_workload}}: patched to {{desired_image}}\")\n            changed = True\n\n        if not changed:\n            print(\"all managed Zebflow workloads are already up to date\")\n        return 0\n\n\n    if __name__ == \"__main__\":\n        try:\n            raise SystemExit(main())\n        except Exception as exc:\n            print(f\"auto-update failed: {{exc}}\", file=sys.stderr)\n            raise\n---\napiVersion: batch/v1\nkind: CronJob\nmetadata:\n  name: zebflow-auto-updater\n  namespace: {namespace}\nspec:\n  schedule: \"{schedule}\"\n  # zebflow:managed-begin {suspend_block}\n{suspend_value}  # zebflow:managed-end {suspend_block}\n  concurrencyPolicy: Forbid\n  successfulJobsHistoryLimit: 2\n  failedJobsHistoryLimit: 2\n  jobTemplate:\n    spec:\n      template:\n        spec:\n          serviceAccountName: zebflow-auto-updater\n          restartPolicy: OnFailure\n          tolerations:\n            - effect: NoSchedule\n              key: node-role.kubernetes.io/control-plane\n              operator: Exists\n          securityContext:\n            runAsNonRoot: true\n            runAsUser: 65532\n            runAsGroup: 65532\n            seccompProfile:\n              type: RuntimeDefault\n          containers:\n            - name: updater\n              image: python:3.12-slim\n              imagePullPolicy: IfNotPresent\n              command:\n                - python\n                - /opt/zebflow-auto-update/update.py\n              env:\n                - name: PYTHONDONTWRITEBYTECODE\n                  value: \"1\"\n                - name: IMAGE_REPO\n                  # zebflow:managed-begin {image_block}\n{image_repo_value}                  # zebflow:managed-end {image_block}\n                - name: TAG_REGEX\n                  value: \"^[0-9]+\\\\.[0-9]+\\\\.[0-9]+\\\\.[0-9]{{12}}$\"\n                - name: TARGET_NAMESPACE\n                  value: \"{namespace}\"\n                - name: TARGET_WORKLOADS\n                  # zebflow:managed-begin {targets_block}\n{targets_value}                  # zebflow:managed-end {targets_block}\n                - name: TARGET_CONTAINER\n                  value: \"zebflow\"\n              volumeMounts:\n                - name: script\n                  mountPath: /opt/zebflow-auto-update\n                  readOnly: true\n              resources:\n                requests:\n                  cpu: \"25m\"\n                  memory: \"64Mi\"\n                limits:\n                  cpu: \"100m\"\n                  memory: \"128Mi\"\n              securityContext:\n                allowPrivilegeEscalation: false\n                capabilities:\n                  drop:\n                    - ALL\n          volumes:\n            - name: script\n              configMap:\n                name: zebflow-auto-updater-script\n                defaultMode: 0555\n",
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
    cfg.managed_offices
        .iter()
        .map(|office_id| format!("      - {office_id}"))
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
    fs::write(path, content)
}

fn default_image() -> String {
    format!("insanalamin/zebflow:{APP_VERSION}")
}

fn managed_workload_ref(office_id: &str) -> String {
    format!("statefulsets/{office_id}")
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
}
