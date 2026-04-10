//! Project-sharing, membership, invite, and user-bound git identity services.
//!
//! This module is the product-facing access-control layer that sits above the
//! lower-level capability engine in [`crate::platform::services::authorization`].
//!
//! The existing authorization core already understands:
//!
//! - project-scoped capabilities
//! - policy bundles
//! - subject bindings for users, MCP sessions, and assistant profiles
//!
//! What it does **not** give the product by itself is a stable UX model for:
//!
//! - GitLab-style membership presets (`guest`, `reporter`, ...)
//! - project members and invites
//! - user-owned Git identity that should appear in commits
//! - per-member MCP ceilings and other future fine-grained sharing controls
//!
//! Those concerns belong here.
//!
//! Design rules:
//!
//! 1. Membership is the UX model. Policies remain the enforcement model.
//! 2. User profile settings should own commit identity whenever possible.
//! 3. Project sharing should be able to evolve without rewriting capability checks.
//! 4. `0.2.0` should establish the stable vocabulary for later UI and API work.

pub mod git_identity;
pub mod invite;
pub mod membership;
pub mod roles;

pub use git_identity::GitIdentityService;
pub use invite::ProjectInviteService;
pub use membership::ProjectMembershipService;
