PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  INTEGER NOT NULL DEFAULT 0
);
INSERT INTO schema_migrations VALUES(1,'initial_catalog',0);
INSERT INTO schema_migrations VALUES(2,'pipeline_invocation_run_id',0);
INSERT INTO schema_migrations VALUES(3,'hub_columns_and_publishers',0);
INSERT INTO schema_migrations VALUES(4,'stable_ids_and_user_local_auth',0);
INSERT INTO schema_migrations VALUES(5,'authorities_offices_and_runtime_normalization',0);
INSERT INTO schema_migrations VALUES(6,'hub_and_operations_internal_ids',0);
INSERT INTO schema_migrations VALUES(7,'ownership_and_access_internal_ids',0);
INSERT INTO schema_migrations VALUES(8,'constraint_and_uniqueness_hardening',0);
INSERT INTO schema_migrations VALUES(9,'core_foreign_key_enforcement',0);
INSERT INTO schema_migrations VALUES(10,'project_access_normalization',0);
INSERT INTO schema_migrations VALUES(11,'hub_token_scope_flags',0);
INSERT INTO schema_migrations VALUES(12,'platform_service_instances',0);
INSERT INTO schema_migrations VALUES(13,'hub_publisher_limits',0);
INSERT INTO schema_migrations VALUES(14,'reserved_post_stable_hub_schema',0);
INSERT INTO schema_migrations VALUES(15,'hub_access_grants',0);
INSERT INTO schema_migrations VALUES(16,'user_credential_state',0);
INSERT INTO schema_migrations VALUES(17,'office_join_tokens',0);
INSERT INTO schema_migrations VALUES(18,'office_vouch_redemptions_and_identity_writes',0);
INSERT INTO schema_migrations VALUES(19,'office_local_authority',0);
INSERT INTO schema_migrations VALUES(20,'credential_keys',0);
INSERT INTO schema_migrations VALUES(21,'member_provenance_is_history_not_reference',0);
CREATE TABLE users (
    owner         TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL UNIQUE DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'owner',
    git_name      TEXT NOT NULL DEFAULT '',
    git_email     TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE project_credentials (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    kind          TEXT NOT NULL DEFAULT '',
    secret_json   TEXT NOT NULL DEFAULT 'null',
    notes         TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, credential_id)
);
CREATE TABLE project_db_connections (
    owner            TEXT NOT NULL,
    project          TEXT NOT NULL,
    connection_id    TEXT NOT NULL,
    connection_slug  TEXT NOT NULL DEFAULT '',
    connection_label TEXT NOT NULL DEFAULT '',
    database_kind    TEXT NOT NULL DEFAULT '',
    credential_id    TEXT,
    config_json      TEXT NOT NULL DEFAULT 'null',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, connection_id)
);
CREATE TABLE project_hub_repositories (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    base_url      TEXT NOT NULL DEFAULT '',
    remote_owner  TEXT NOT NULL DEFAULT '',
    remote_project TEXT NOT NULL DEFAULT '',
    read_token    TEXT NOT NULL DEFAULT '',
    visibility    TEXT NOT NULL DEFAULT 'public',
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, repository_id)
);
CREATE TABLE hub_access_grants (
    grant_id       TEXT PRIMARY KEY,
    source_owner   TEXT NOT NULL DEFAULT '',
    source_id      TEXT NOT NULL DEFAULT '',
    repository_id  TEXT NOT NULL DEFAULT '',
    grant_scope    TEXT NOT NULL DEFAULT 'selected_project',
    target_owner   TEXT NOT NULL DEFAULT '',
    target_project TEXT NOT NULL DEFAULT '',
    can_read       INTEGER NOT NULL DEFAULT 1,
    can_publish    INTEGER NOT NULL DEFAULT 0,
    can_manage     INTEGER NOT NULL DEFAULT 0,
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE pipeline_meta (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    file_rel_path TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    title         TEXT NOT NULL DEFAULT '',
    virtual_path  TEXT NOT NULL DEFAULT '',
    description   TEXT NOT NULL DEFAULT '',
    trigger_kind  TEXT NOT NULL DEFAULT '',
    hash          TEXT NOT NULL DEFAULT '',
    active_hash   TEXT,
    activated_at  INTEGER,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, file_rel_path)
);
CREATE TABLE project_invites (
    owner                  TEXT NOT NULL,
    project                TEXT NOT NULL,
    invite_id              TEXT NOT NULL,
    target_user            TEXT NOT NULL DEFAULT '',
    role_preset            TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json  TEXT NOT NULL DEFAULT '[]',
    note                   TEXT NOT NULL DEFAULT '',
    invited_by             TEXT NOT NULL DEFAULT '',
    status                 TEXT NOT NULL DEFAULT 'pending',
    expires_at             INTEGER,
    created_at             INTEGER NOT NULL DEFAULT 0,
    updated_at             INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, invite_id)
);
CREATE TABLE offices (
    office_id     TEXT PRIMARY KEY,
    office_slug   TEXT NOT NULL UNIQUE DEFAULT '',
    label         TEXT NOT NULL DEFAULT '',
    office_kind   TEXT NOT NULL DEFAULT 'office',
    base_url      TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE office_join_tokens (
    office_id     TEXT PRIMARY KEY,
    secret_digest TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'active',
    note          TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    last_used_at  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);
CREATE TABLE platform_service_instances (
    service_instance_id TEXT PRIMARY KEY,
    service_kind        TEXT NOT NULL DEFAULT '',
    display_label       TEXT NOT NULL DEFAULT '',
    host_office_id      TEXT NOT NULL DEFAULT '',
    state_office_id     TEXT NOT NULL DEFAULT '',
    public_base_url     TEXT NOT NULL DEFAULT '',
    enabled             INTEGER NOT NULL DEFAULT 0,
    status              TEXT NOT NULL DEFAULT '',
    placement_generation INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE worker_registry (
    office_id           TEXT NOT NULL DEFAULT '',
    office_slug         TEXT NOT NULL DEFAULT '',
    node_id             TEXT PRIMARY KEY,
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0);
CREATE TABLE mcp_sessions (
    token              TEXT PRIMARY KEY,
    owner              TEXT NOT NULL DEFAULT '',
    project            TEXT NOT NULL DEFAULT '',
    capabilities_json  TEXT NOT NULL DEFAULT '[]',
    created_at         INTEGER NOT NULL DEFAULT 0,
    auto_reset_seconds INTEGER,
    enabled            INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE pipeline_invocations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    file_rel_path TEXT NOT NULL,
    run_id        TEXT NOT NULL DEFAULT '',
    at            INTEGER NOT NULL DEFAULT 0,
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    status        TEXT NOT NULL DEFAULT '',
    trigger       TEXT NOT NULL DEFAULT '',
    error         TEXT,
    trace_json    TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE user_local_auth (
    user_id              TEXT PRIMARY KEY,
    password_hash        TEXT NOT NULL DEFAULT '',
    password_alg         TEXT NOT NULL DEFAULT 'sha256',
    password_updated_at  INTEGER NOT NULL DEFAULT 0, credential_state TEXT NOT NULL DEFAULT 'chosen',
    FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);
CREATE TABLE projects (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    project_id    TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE platform_hub_repositories (
    source_id      TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id  TEXT NOT NULL DEFAULT '',
    owner          TEXT NOT NULL,
    repository_id  TEXT NOT NULL,
    title          TEXT NOT NULL DEFAULT '',
    base_url       TEXT NOT NULL DEFAULT '',
    remote_owner   TEXT NOT NULL DEFAULT '',
    remote_project TEXT NOT NULL DEFAULT '',
    read_token     TEXT NOT NULL DEFAULT '',
    kind           TEXT NOT NULL DEFAULT 'api',
    priority       INTEGER NOT NULL DEFAULT 100,
    visibility     TEXT NOT NULL DEFAULT 'public',
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, repository_id),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE hub_authorities (
    authority_id     TEXT PRIMARY KEY,
    host_project_id  TEXT NOT NULL UNIQUE DEFAULT '',
    owner            TEXT NOT NULL DEFAULT '',
    project          TEXT NOT NULL DEFAULT '',
    enabled          INTEGER NOT NULL DEFAULT 0,
    public_base_url  TEXT NOT NULL DEFAULT '',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (host_project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE hub_publishers (
    authority_id    TEXT NOT NULL DEFAULT '',
    publisher_pk    TEXT NOT NULL UNIQUE DEFAULT '',
    owner           TEXT NOT NULL,
    project         TEXT NOT NULL,
    publisher_id    TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    publisher_url   TEXT NOT NULL DEFAULT '',
    email           TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    icon_url        TEXT NOT NULL DEFAULT '',
    website_url     TEXT NOT NULL DEFAULT '',
    enabled         INTEGER NOT NULL DEFAULT 1,
    can_read        INTEGER NOT NULL DEFAULT 1,
    can_publish     INTEGER NOT NULL DEFAULT 1,
    can_manage      INTEGER NOT NULL DEFAULT 0,
    max_packages       INTEGER NOT NULL DEFAULT 0,
    max_package_bytes  INTEGER NOT NULL DEFAULT 0,
    max_media_files    INTEGER NOT NULL DEFAULT 0,
    max_image_bytes    INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL DEFAULT 0,
    updated_at      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, publisher_id),
    FOREIGN KEY (authority_id) REFERENCES hub_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE hub_asset_packages (
    package_pk              TEXT NOT NULL UNIQUE DEFAULT '',
    authority_id            TEXT NOT NULL DEFAULT '',
    publisher_pk            TEXT NOT NULL DEFAULT '',
    package_id              TEXT PRIMARY KEY,
    authority_owner         TEXT NOT NULL DEFAULT '',
    authority_project       TEXT NOT NULL DEFAULT '',
    publisher_owner         TEXT NOT NULL DEFAULT '',
    publisher_id            TEXT NOT NULL DEFAULT '',
    publisher_display_name  TEXT NOT NULL DEFAULT '',
    publisher_url           TEXT NOT NULL DEFAULT '',
    publisher_email         TEXT NOT NULL DEFAULT '',
    asset_kind              TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    description             TEXT NOT NULL DEFAULT '',
    summary                 TEXT NOT NULL DEFAULT '',
    description_md          TEXT NOT NULL DEFAULT '',
    image_url               TEXT NOT NULL DEFAULT '',
    media_json              TEXT NOT NULL DEFAULT '[]',
    gallery_json            TEXT NOT NULL DEFAULT '{}',
    visibility              TEXT NOT NULL DEFAULT 'private',
    tags_json               TEXT NOT NULL DEFAULT '[]',
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES hub_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES hub_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE hub_asset_versions (
    package_pk         TEXT NOT NULL DEFAULT '',
    package_id         TEXT NOT NULL,
    version            TEXT NOT NULL,
    authority_owner    TEXT NOT NULL DEFAULT '',
    authority_project  TEXT NOT NULL DEFAULT '',
    publisher_owner    TEXT NOT NULL DEFAULT '',
    publisher_id       TEXT NOT NULL DEFAULT '',
    source_owner       TEXT NOT NULL DEFAULT '',
    source_project     TEXT NOT NULL DEFAULT '',
    source_kind        TEXT NOT NULL DEFAULT '',
    source_ref         TEXT NOT NULL DEFAULT '',
    artifact_rel_path  TEXT NOT NULL DEFAULT '',
    artifact_sha256    TEXT NOT NULL DEFAULT '',
    manifest_json      TEXT NOT NULL DEFAULT 'null',
    created_at         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (package_id, version),
    FOREIGN KEY (package_pk) REFERENCES hub_asset_packages(package_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE office_nodes (
    node_id             TEXT PRIMARY KEY,
    office_id           TEXT NOT NULL DEFAULT '',
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE project_runtime_placements (
    project_id         TEXT NOT NULL DEFAULT '',
    owner              TEXT NOT NULL,
    project            TEXT NOT NULL,
    mode               TEXT NOT NULL DEFAULT 'shared',
    target             TEXT NOT NULL DEFAULT 'local',
    target_office_id   TEXT,
    target_node_id     TEXT,
    worker_id          TEXT,
    resource_profile   TEXT NOT NULL DEFAULT '',
    desired_replicas   INTEGER NOT NULL DEFAULT 1,
    effective_state    TEXT NOT NULL DEFAULT '',
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_node_id) REFERENCES office_nodes(node_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE project_operations (
    project_id          TEXT NOT NULL DEFAULT '',
    owner               TEXT NOT NULL,
    project             TEXT NOT NULL,
    operation_id        TEXT NOT NULL,
    kind                TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT 'pending',
    current_step        TEXT NOT NULL DEFAULT '',
    source_office_id    TEXT,
    target_office_id    TEXT,
    artifact_rel_path   TEXT,
    artifact_sha256     TEXT,
    artifact_bytes      INTEGER,
    error_message       TEXT,
    retry_count         INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT 0,
    completed_at        INTEGER,
    PRIMARY KEY (owner, project, operation_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (source_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE project_policies (
    project_id        TEXT NOT NULL DEFAULT '',
    owner             TEXT NOT NULL,
    project           TEXT NOT NULL,
    policy_id         TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    managed           INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT 0,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, policy_id),
    UNIQUE (project_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE project_policy_bindings (
    project_id    TEXT NOT NULL DEFAULT '',
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    subject_kind  TEXT NOT NULL,
    subject_id    TEXT NOT NULL,
    policy_id     TEXT NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, subject_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (project_id, policy_id) REFERENCES project_policies(project_id, policy_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE hub_tokens (
    token_id                TEXT PRIMARY KEY,
    authority_id            TEXT NOT NULL DEFAULT '',
    publisher_pk            TEXT NOT NULL DEFAULT '',
    owner                   TEXT NOT NULL DEFAULT '',
    project                 TEXT NOT NULL DEFAULT '',
    publisher_id            TEXT NOT NULL DEFAULT '',
    publisher_display_name  TEXT NOT NULL DEFAULT '',
    publisher_url           TEXT NOT NULL DEFAULT '',
    publisher_email         TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    secret_hash             TEXT NOT NULL DEFAULT '',
    scope_read              INTEGER NOT NULL DEFAULT 0,
    scope_publish           INTEGER NOT NULL DEFAULT 0,
    scope_manage            INTEGER NOT NULL DEFAULT 0,
    expires_at              INTEGER,
    last_used_at            INTEGER,
    revoked_at              INTEGER,
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES hub_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES hub_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE office_vouch_redemptions (
                nonce       TEXT PRIMARY KEY,
                office_id   TEXT NOT NULL DEFAULT '',
                identity    TEXT NOT NULL DEFAULT '',
                expires_at  INTEGER NOT NULL DEFAULT 0,
                redeemed_at INTEGER NOT NULL DEFAULT 0
            );
CREATE TABLE office_identity_writes (
                write_id   TEXT PRIMARY KEY,
                office_id  TEXT NOT NULL DEFAULT '',
                owner      TEXT NOT NULL DEFAULT '',
                action     TEXT NOT NULL DEFAULT '',
                role       TEXT NOT NULL DEFAULT '',
                source     TEXT NOT NULL DEFAULT '',
                detail     TEXT NOT NULL DEFAULT '',
                written_at INTEGER NOT NULL DEFAULT 0
            );
CREATE TABLE office_local_authority (
                event_id         TEXT PRIMARY KEY,
                office_id        TEXT NOT NULL DEFAULT '',
                event            TEXT NOT NULL DEFAULT '',
                join_fingerprint TEXT NOT NULL DEFAULT '',
                owner            TEXT NOT NULL DEFAULT '',
                detail           TEXT NOT NULL DEFAULT '',
                acted_at         INTEGER NOT NULL DEFAULT 0,
                reported_at      INTEGER NOT NULL DEFAULT 0
            );
CREATE TABLE credential_keys (
                key_id      INTEGER PRIMARY KEY,
                wrapped_key TEXT NOT NULL,
                created_at  INTEGER NOT NULL DEFAULT 0
            );
CREATE TABLE project_members (
    project_id              TEXT NOT NULL DEFAULT '',
    owner                   TEXT NOT NULL,
    project                 TEXT NOT NULL,
    user_id                 TEXT NOT NULL,
    member_user_id          TEXT NOT NULL DEFAULT '',
    role_preset             TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json  TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json   TEXT NOT NULL DEFAULT '[]',
    created_by              TEXT NOT NULL DEFAULT '',
    created_by_user_id      TEXT NOT NULL DEFAULT '',
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, user_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (member_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE INDEX idx_pipeline_invocations_pipeline
    ON pipeline_invocations (owner, project, file_rel_path, at DESC);
CREATE UNIQUE INDEX idx_users_user_id ON users(user_id);
CREATE UNIQUE INDEX idx_project_db_connections_slug
             ON project_db_connections(owner, project, connection_slug);
CREATE INDEX idx_pipeline_meta_project_file
             ON pipeline_meta(owner, project, file_rel_path);
CREATE INDEX idx_office_nodes_office_id
             ON office_nodes(office_id);
CREATE INDEX idx_project_runtime_placements_project_id
             ON project_runtime_placements(project_id);
CREATE INDEX idx_project_operations_project
             ON project_operations(owner, project, updated_at DESC, operation_id ASC);
CREATE INDEX idx_project_operations_project_id
             ON project_operations(project_id, updated_at DESC, operation_id ASC);
CREATE UNIQUE INDEX idx_hub_publishers_publisher_pk
             ON hub_publishers(publisher_pk);
CREATE UNIQUE INDEX idx_hub_asset_packages_package_pk
             ON hub_asset_packages(package_pk);
CREATE INDEX idx_hub_asset_versions_package_pk
             ON hub_asset_versions(package_pk);
CREATE UNIQUE INDEX idx_hub_publishers_authority_publisher
             ON hub_publishers(authority_id, publisher_id);
CREATE UNIQUE INDEX idx_hub_publishers_authority_url
             ON hub_publishers(authority_id, publisher_url)
             WHERE publisher_url <> '';
CREATE UNIQUE INDEX idx_hub_asset_packages_authority_package
             ON hub_asset_packages(authority_id, package_id);
CREATE UNIQUE INDEX idx_hub_asset_versions_packagepk_version
             ON hub_asset_versions(package_pk, version);
CREATE INDEX idx_platform_hub_repositories_owner_user
             ON platform_hub_repositories(owner_user_id, enabled, title);
CREATE UNIQUE INDEX idx_project_policies_project_policy
             ON project_policies(project_id, policy_id);
CREATE UNIQUE INDEX idx_project_policy_bindings_project_subject_policy
             ON project_policy_bindings(project_id, subject_kind, subject_id, policy_id);
CREATE INDEX idx_hub_tokens_publisher_pk
             ON hub_tokens(publisher_pk);
CREATE INDEX idx_hub_tokens_scope_publish
             ON hub_tokens(scope_publish, revoked_at, expires_at);
CREATE INDEX idx_hub_tokens_scope_read
             ON hub_tokens(scope_read, revoked_at, expires_at);
CREATE INDEX idx_platform_service_instances_kind
    ON platform_service_instances(service_kind);
CREATE INDEX idx_platform_service_instances_host
    ON platform_service_instances(host_office_id);
CREATE UNIQUE INDEX idx_hub_access_grants_unique_target
    ON hub_access_grants(source_id, grant_scope, target_owner, target_project);
CREATE INDEX idx_hub_access_grants_source
    ON hub_access_grants(source_owner, repository_id, enabled);
CREATE INDEX idx_hub_access_grants_project
    ON hub_access_grants(target_owner, target_project, enabled);
CREATE INDEX idx_office_vouch_redemptions_expires_at
                ON office_vouch_redemptions (expires_at);
CREATE INDEX idx_office_identity_writes_written_at
                ON office_identity_writes (written_at);
CREATE INDEX idx_office_local_authority_acted_at
                ON office_local_authority (acted_at);
CREATE INDEX idx_project_members_project_id
                 ON project_members(project_id, user_id);
CREATE UNIQUE INDEX idx_project_members_project_user
                 ON project_members(project_id, user_id);
CREATE UNIQUE INDEX idx_project_members_project_member_user_id
                 ON project_members(project_id, member_user_id);
CREATE INDEX idx_project_members_member_user_id
                 ON project_members(member_user_id);
COMMIT;
