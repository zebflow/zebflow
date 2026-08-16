use std::collections::HashSet;

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::platform::sekejap::SekejapSchemaExport;

/// Canonical portable database schema stored in a project repository.
pub struct DatabaseSchemaContract;

impl PlatformContract for DatabaseSchemaContract {
    type Spec = SekejapSchemaExport;
    const KIND: ContractKind = ContractKind::DatabaseSchema;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != spec.connection_slug {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match database connection_slug '{}'",
                metadata.name, spec.connection_slug
            )));
        }
        if spec.database != "sekejap" {
            return Err(ContractError::invalid(format!(
                "unsupported database schema kind '{}'",
                spec.database
            )));
        }
        if spec.connection_slug.trim().is_empty() {
            return Err(ContractError::invalid(
                "database schema connection_slug must not be empty",
            ));
        }

        let mut tables = HashSet::new();
        let mut collections = HashSet::new();
        for table in &spec.tables {
            if table.table.trim().is_empty() || table.collection.trim().is_empty() {
                return Err(ContractError::invalid(
                    "database schema table and collection names must not be empty",
                ));
            }
            if !tables.insert(table.table.as_str()) {
                return Err(ContractError::invalid(format!(
                    "duplicate database schema table '{}'",
                    table.table
                )));
            }
            if !collections.insert(table.collection.as_str()) {
                return Err(ContractError::invalid(format!(
                    "duplicate database schema collection '{}'",
                    table.collection
                )));
            }
            let mut attributes = HashSet::new();
            for attribute in &table.attributes {
                if attribute.name.trim().is_empty() {
                    return Err(ContractError::invalid(
                        "database schema attribute names must not be empty",
                    ));
                }
                if !attributes.insert(attribute.name.as_str()) {
                    return Err(ContractError::invalid(format!(
                        "duplicate attribute '{}' in table '{}'",
                        attribute.name, table.table
                    )));
                }
            }
        }
        Ok(())
    }
}
