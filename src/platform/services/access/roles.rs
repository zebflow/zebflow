//! Managed role presets for project membership.
//!
//! Zebflow keeps capability enforcement separate from the product-facing sharing
//! vocabulary. This module defines that vocabulary and the managed policy bundles
//! backing it.
//!
//! The baseline intentionally follows familiar GitLab-style names:
//!
//! - `guest`
//! - `reporter`
//! - `developer`
//! - `maintainer`
//! - `owner`
//!
//! Legacy aliases such as `viewer` and `editor` can still exist as managed
//! policies so older policy ids do not immediately become invalid during the
//! `0.2.0` baseline transition.

use crate::platform::model::{ProjectAccessRolePreset, ProjectCapability, ProjectPolicy};

/// What each rung adds to the one below it.
///
/// Cumulative on purpose. Every rung used to list its capabilities in full, so
/// Maintainer repeated all twenty of Developer's by hand. Adding one to
/// Developer and forgetting Maintainer left a Developer holding something a
/// Maintainer did not — silently, because nothing compared the two.
///
/// **To give a role a new capability, add it to that rung.** Every rung above
/// inherits it, and `the_ladder_never_loses_a_capability_going_up` proves it.
const fn role_additions(role: ProjectAccessRolePreset) -> &'static [ProjectCapability] {
    use ProjectCapability as C;
    match role {
        // Can look at the project and what is in it. Sees no data, runs nothing.
        ProjectAccessRolePreset::Guest => &[
            C::ProjectRead,
            C::MembersRead,
            C::TemplatesRead,
            C::PipelinesRead,
            C::FilesRead,
            C::LibrariesRead,
        ],
        // Adds the data and the settings, still read-only throughout.
        ProjectAccessRolePreset::Reporter => &[C::TablesRead, C::SettingsRead],
        // Builds and runs. Deliberately stops short of `TablesWrite`: someone
        // can write a pipeline that changes data without being able to change
        // rows by hand.
        ProjectAccessRolePreset::Developer => &[
            C::TemplatesWrite,
            C::TemplatesCreate,
            C::TemplatesDelete,
            C::TemplatesMove,
            C::TemplatesDiagnostics,
            C::PipelinesWrite,
            C::PipelinesCreate,
            C::PipelinesDelete,
            C::PipelinesMove,
            C::PipelinesExecute,
            C::FilesWrite,
            C::FilesDelete,
        ],
        // Runs the project: who is in it, what it connects to, what it installs.
        ProjectAccessRolePreset::Maintainer => &[
            C::MembersWrite,
            C::CredentialsRead,
            C::CredentialsWrite,
            C::TablesWrite,
            C::LibrariesInstall,
            C::LibrariesRemove,
            C::SettingsWrite,
            C::McpSessionCreate,
            C::McpSessionRevoke,
        ],
        // The project's own life. This is the whole difference between an
        // Owner and a Maintainer, and until it existed the two rungs resolved
        // to the same set.
        ProjectAccessRolePreset::Owner => &[C::ProjectDelete],
    }
}

/// The rungs, weakest first. The ladder's order is this array and nothing else.
pub const ROLE_LADDER: [ProjectAccessRolePreset; 5] = [
    ProjectAccessRolePreset::Guest,
    ProjectAccessRolePreset::Reporter,
    ProjectAccessRolePreset::Developer,
    ProjectAccessRolePreset::Maintainer,
    ProjectAccessRolePreset::Owner,
];

/// Return the managed capability bundle for one user-facing role preset.
///
/// A rung's bundle is every addition from the bottom of the ladder up to and
/// including it, so a capability is written down once.
pub fn role_capabilities(role: ProjectAccessRolePreset) -> Vec<ProjectCapability> {
    if role == ProjectAccessRolePreset::Owner {
        return ProjectCapability::all();
    }
    let mut out: Vec<ProjectCapability> = Vec::new();
    for rung in ROLE_LADDER {
        out.extend_from_slice(role_additions(rung));
        if rung == role {
            break;
        }
    }
    out.dedup();
    out
}

/// Whether `actor` may grant `granted`.
///
/// A maintainer cannot mint an owner. GitLab words it as "the maximum role you
/// can assign depends on your own permissions", and without it the members
/// screen is a way for anyone who can edit members to promote themselves.
pub fn can_grant(actor: ProjectAccessRolePreset, granted: ProjectAccessRolePreset) -> bool {
    let rank = |role| ROLE_LADDER.iter().position(|entry| *entry == role);
    match (rank(actor), rank(granted)) {
        (Some(actor_rank), Some(granted_rank)) => granted_rank <= actor_rank,
        _ => false,
    }
}

/// Build the managed policy rows for the core access presets.
pub fn managed_role_policies(owner: &str, project: &str, now: i64) -> Vec<ProjectPolicy> {
    [
        ProjectAccessRolePreset::Owner,
        ProjectAccessRolePreset::Guest,
        ProjectAccessRolePreset::Reporter,
        ProjectAccessRolePreset::Developer,
        ProjectAccessRolePreset::Maintainer,
    ]
    .into_iter()
    .map(|role| ProjectPolicy {
        project_id: String::new(),
        owner: owner.to_string(),
        project: project.to_string(),
        policy_id: role.policy_id().to_string(),
        title: role.title().to_string(),
        capabilities: role_capabilities(role),
        managed: true,
        created_at: now,
        updated_at: now,
    })
    .collect()
}

/// Transitional aliases that keep older policy ids readable while the product
/// moves toward GitLab-style role language.
pub fn managed_role_alias_policies(owner: &str, project: &str, now: i64) -> Vec<ProjectPolicy> {
    vec![
        ProjectPolicy {
            project_id: String::new(),
            owner: owner.to_string(),
            project: project.to_string(),
            policy_id: "viewer".to_string(),
            title: "Viewer".to_string(),
            capabilities: role_capabilities(ProjectAccessRolePreset::Reporter),
            managed: true,
            created_at: now,
            updated_at: now,
        },
        ProjectPolicy {
            project_id: String::new(),
            owner: owner.to_string(),
            project: project.to_string(),
            policy_id: "editor".to_string(),
            title: "Editor".to_string(),
            capabilities: role_capabilities(ProjectAccessRolePreset::Developer),
            managed: true,
            created_at: now,
            updated_at: now,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ladder is a ladder: nothing is lost climbing it.
    ///
    /// This is the guarantee the cumulative shape exists to give. When every
    /// rung listed its capabilities in full, adding one to Developer and
    /// forgetting Maintainer left a Developer able to do something a Maintainer
    /// could not, and no test would have said so.
    #[test]
    fn the_ladder_never_loses_a_capability_going_up() {
        for pair in ROLE_LADDER.windows(2) {
            let (lower, upper) = (pair[0], pair[1]);
            let lower_caps = role_capabilities(lower);
            let upper_caps = role_capabilities(upper);
            for capability in &lower_caps {
                assert!(
                    upper_caps.contains(capability),
                    "{} has '{}' and {} does not",
                    lower.title(),
                    capability.key(),
                    upper.title()
                );
            }
        }
    }

    /// A capability no role grants is a route nobody can reach.
    ///
    /// Adding one to the enum and wiring it to a handler is half the change;
    /// without a rung that carries it, the handler answers 403 for everyone
    /// including the owner — and the owner case is only covered because Owner
    /// is `all()` rather than a list.
    #[test]
    fn every_capability_is_reachable_by_some_role() {
        for capability in ProjectCapability::all() {
            assert!(
                ROLE_LADDER
                    .iter()
                    .any(|role| role_capabilities(*role).contains(&capability)),
                "no role grants '{}', so nothing guarded by it can be used",
                capability.key()
            );
        }
    }

    /// A rung grants strictly more than the one below, never merely the same.
    #[test]
    fn each_rung_is_worth_climbing() {
        for pair in ROLE_LADDER.windows(2) {
            let (lower, upper) = (pair[0], pair[1]);
            assert!(
                role_capabilities(upper).len() > role_capabilities(lower).len(),
                "{} grants no more than {}, so one of them has no reason to exist",
                upper.title(),
                lower.title()
            );
        }
    }

    /// Nobody hands out more than they hold.
    #[test]
    fn a_role_cannot_grant_above_itself() {
        use ProjectAccessRolePreset::*;
        assert!(can_grant(Owner, Owner));
        assert!(can_grant(Owner, Maintainer));
        assert!(can_grant(Maintainer, Developer));
        assert!(can_grant(Maintainer, Maintainer));
        assert!(!can_grant(Maintainer, Owner), "a maintainer must not mint an owner");
        assert!(!can_grant(Developer, Maintainer));
        assert!(!can_grant(Guest, Reporter));
    }

    #[test]
    fn owner_role_covers_all_capabilities() {
        assert_eq!(
            role_capabilities(ProjectAccessRolePreset::Owner),
            ProjectCapability::all()
        );
    }

    #[test]
    fn role_aliases_point_at_stable_capability_bundles() {
        let aliases = managed_role_alias_policies("superadmin", "default", 1);
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].policy_id, "viewer");
        assert_eq!(
            aliases[0].capabilities,
            role_capabilities(ProjectAccessRolePreset::Reporter)
        );
        assert_eq!(aliases[1].policy_id, "editor");
        assert_eq!(
            aliases[1].capabilities,
            role_capabilities(ProjectAccessRolePreset::Developer)
        );
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// The counts written into `docs/contracts/kinds/project-access/README.md`.
    ///
    /// A contract quoting numbers nobody checks drifts from the code the first
    /// time a rung gains a capability, and then reads as authoritative while
    /// being wrong.
    #[test]
    fn the_contract_quotes_the_real_role_sizes() {
        let sizes: Vec<usize> = ROLE_LADDER
            .iter()
            .map(|role| role_capabilities(*role).len())
            .collect();
        assert_eq!(
            sizes,
            vec![6, 8, 20, 29, 30],
            "guest/reporter/developer/maintainer/owner sizes changed — update \
             docs/contracts/kinds/project-access/README.md to match"
        );
    }
}
