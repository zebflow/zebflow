//! Table definition for SQL engines, built from the same attribute list the
//! studio collects for every engine.
//!
//! Identifiers reach the database as text rather than as bind parameters, so
//! every name is validated here before it is quoted. A name that is not a plain
//! identifier is refused rather than escaped.

use crate::platform::error::PlatformError;
use crate::platform::model::CollectionAttribute;

/// The SQL dialect a statement is being written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    Sqlite,
    Postgres,
    MySql,
}

impl SqlDialect {
    /// Wraps one already-validated identifier for this dialect.
    ///
    /// MySQL quotes with backticks by default; the other two use the SQL
    /// standard double quote. Names are validated before they reach here, so
    /// this only has to pick the character.
    pub fn quote(&self, name: &str) -> String {
        match self {
            SqlDialect::MySql => format!("`{name}`"),
            _ => format!("\"{name}\""),
        }
    }
}

/// The column every created table carries, so a row can be identified for
/// editing and deletion. Sekejap's own equivalent is `_key`.
pub const IDENTITY_COLUMN: &str = "id";

/// Refuses anything that is not a plain lowercase identifier.
///
/// Quoting would make a strange name safe to execute, but it would also let a
/// table exist that nothing else in Zebflow can address. Refusing keeps the
/// name space the same everywhere.
pub fn validate_identifier(raw: &str, what: &str) -> Result<String, PlatformError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_NAME",
            format!("{what} name is empty"),
        ));
    }
    if name.len() > 63 {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_NAME",
            format!("{what} name '{name}' is longer than 63 characters"),
        ));
    }
    let first_ok = name
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false);
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !first_ok || !rest_ok {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_NAME",
            format!(
                "{what} name '{name}' must start with a letter or underscore and use only letters, digits and underscores"
            ),
        ));
    }
    Ok(name.to_ascii_lowercase())
}

/// The column type one attribute kind becomes.
///
/// Kinds that need a database extension are refused by name rather than
/// silently stored as text, because a column that claims to hold geometry and
/// holds a string is worse than a rejected request.
fn column_type(kind: &str, dialect: SqlDialect) -> Result<&'static str, PlatformError> {
    let kind = kind.trim().to_ascii_lowercase();
    let ty = match (kind.as_str(), dialect) {
        // MySQL cannot index a TEXT column without a prefix length, so the
        // indexable kind is a bounded VARCHAR there and TEXT stays for prose.
        ("string", SqlDialect::MySql) => "VARCHAR(255)",
        ("string", _) | ("text", _) => "TEXT",
        ("number", SqlDialect::Sqlite) => "REAL",
        ("number", SqlDialect::Postgres) => "DOUBLE PRECISION",
        ("number", SqlDialect::MySql) => "DOUBLE",
        ("boolean", SqlDialect::Sqlite) => "INTEGER",
        ("boolean", SqlDialect::Postgres) => "BOOLEAN",
        ("boolean", SqlDialect::MySql) => "BOOLEAN",
        ("json", SqlDialect::Sqlite) => "TEXT",
        ("json", SqlDialect::Postgres) => "JSONB",
        ("json", SqlDialect::MySql) => "JSON",
        ("vector", _) => {
            return Err(PlatformError::new(
                "PLATFORM_DB_DDL_KIND",
                "vector columns need a database extension this driver does not manage",
            ));
        }
        ("geo", _) => {
            return Err(PlatformError::new(
                "PLATFORM_DB_DDL_KIND",
                "geometry columns need a database extension this driver does not manage",
            ));
        }
        (other, _) => {
            return Err(PlatformError::new(
                "PLATFORM_DB_DDL_KIND",
                format!("unknown attribute kind '{other}'"),
            ));
        }
    };
    Ok(ty)
}

/// The statements that create one table and its indexes.
///
/// Returned as a list because an index is its own statement in both dialects.
pub fn create_table_statements(
    table: &str,
    attributes: &[CollectionAttribute],
    dialect: SqlDialect,
) -> Result<Vec<String>, PlatformError> {
    let table = validate_identifier(table, "table")?;

    let id = dialect.quote(IDENTITY_COLUMN);
    let identity = match dialect {
        SqlDialect::Sqlite => format!("{id} INTEGER PRIMARY KEY AUTOINCREMENT"),
        SqlDialect::Postgres => format!("{id} SERIAL PRIMARY KEY"),
        SqlDialect::MySql => format!("{id} INT AUTO_INCREMENT PRIMARY KEY"),
    };
    let mut columns = vec![identity];
    // Name and column type, because MySQL needs the type to decide whether an
    // index needs a prefix length.
    let mut indexed: Vec<(String, String)> = Vec::new();

    for attr in attributes {
        let name = validate_identifier(&attr.name, "column")?;
        if name == IDENTITY_COLUMN {
            // The identity column is added above; a second one would be a
            // duplicate the database would reject with a worse message.
            continue;
        }
        let ty = column_type(&attr.kind, dialect)?;
        columns.push(format!("{} {ty}", dialect.quote(&name)));

        for index in &attr.index_types {
            match index.trim().to_ascii_lowercase().as_str() {
                // Both are ordinary btree indexes in a SQL engine; the
                // distinction sekejap draws does not exist here.
                "hash" | "range" => {
                    if !indexed.contains(&(name.clone(), ty.to_string())) {
                        indexed.push((name.clone(), ty.to_string()));
                    }
                }
                "fulltext" | "vector" | "spatial" => {
                    return Err(PlatformError::new(
                        "PLATFORM_DB_DDL_INDEX",
                        format!(
                            "'{index}' indexes need a database extension this driver does not manage"
                        ),
                    ));
                }
                other if other.is_empty() => {}
                other => {
                    return Err(PlatformError::new(
                        "PLATFORM_DB_DDL_INDEX",
                        format!("unknown index type '{other}'"),
                    ));
                }
            }
        }
    }

    let quoted_table = dialect.quote(&table);
    let mut statements = vec![format!("CREATE TABLE {quoted_table} ({})", columns.join(", "))];
    for (column, ty) in indexed {
        let index_name = dialect.quote(&format!("idx_{table}_{column}"));
        // A MySQL index over TEXT must state how many characters to index;
        // 191 is the largest prefix that fits utf8mb4 in a legacy key length.
        let target = if dialect == SqlDialect::MySql && ty == "TEXT" {
            format!("{}(191)", dialect.quote(&column))
        } else {
            dialect.quote(&column)
        };
        statements.push(format!(
            "CREATE INDEX {index_name} ON {quoted_table} ({target})"
        ));
    }
    Ok(statements)
}

/// The statement that drops one table.
pub fn drop_table_statement(table: &str, dialect: SqlDialect) -> Result<String, PlatformError> {
    let table = validate_identifier(table, "table")?;
    Ok(format!("DROP TABLE {}", dialect.quote(&table)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(name: &str, kind: &str, indexes: &[&str]) -> CollectionAttribute {
        CollectionAttribute {
            name: name.to_string(),
            kind: kind.to_string(),
            index_types: indexes.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn a_created_table_carries_an_identity_column_and_its_indexes() {
        let statements = create_table_statements(
            "orders",
            &[attr("label", "string", &["hash"]), attr("total", "number", &[])],
            SqlDialect::Postgres,
        )
        .expect("statements");

        assert_eq!(
            statements[0],
            "CREATE TABLE \"orders\" (\"id\" SERIAL PRIMARY KEY, \"label\" TEXT, \"total\" DOUBLE PRECISION)"
        );
        assert_eq!(
            statements[1],
            "CREATE INDEX \"idx_orders_label\" ON \"orders\" (\"label\")"
        );
    }

    #[test]
    fn each_dialect_names_its_own_types() {
        let sqlite = create_table_statements("t", &[attr("flag", "boolean", &[])], SqlDialect::Sqlite)
            .expect("sqlite");
        assert!(sqlite[0].contains("\"flag\" INTEGER"));
        assert!(sqlite[0].contains("AUTOINCREMENT"));

        let postgres =
            create_table_statements("t", &[attr("flag", "boolean", &[])], SqlDialect::Postgres)
                .expect("postgres");
        assert!(postgres[0].contains("\"flag\" BOOLEAN"));
    }

    #[test]
    fn mysql_quotes_with_backticks_and_indexes_text_by_prefix() {
        let statements = create_table_statements(
            "orders",
            &[attr("label", "string", &["hash"]), attr("body", "text", &["range"])],
            SqlDialect::MySql,
        )
        .expect("statements");

        assert!(statements[0].contains("`id` INT AUTO_INCREMENT PRIMARY KEY"));
        // string is bounded so it can be indexed without a prefix
        assert!(statements[0].contains("`label` VARCHAR(255)"));
        assert!(statements[1].contains("ON `orders` (`label`)"));
        // TEXT cannot be indexed whole in MySQL
        assert!(statements[2].contains("(`body`(191))"), "got {}", statements[2]);
    }

    #[test]
    fn a_name_that_is_not_a_plain_identifier_is_refused() {
        for bad in ["", "has space", "drop\"quote", "1leading", "semi;colon"] {
            assert!(
                validate_identifier(bad, "table").is_err(),
                "expected '{bad}' to be refused"
            );
        }
        assert_eq!(validate_identifier("Orders_2", "table").unwrap(), "orders_2");
    }

    #[test]
    fn a_kind_needing_an_extension_is_refused_rather_than_stored_as_text() {
        for kind in ["vector", "geo"] {
            let err = create_table_statements("t", &[attr("c", kind, &[])], SqlDialect::Postgres)
                .expect_err("should refuse");
            assert_eq!(err.code, "PLATFORM_DB_DDL_KIND");
        }
        let err = create_table_statements("t", &[attr("c", "string", &["fulltext"])], SqlDialect::Sqlite)
            .expect_err("should refuse");
        assert_eq!(err.code, "PLATFORM_DB_DDL_INDEX");
    }
}
