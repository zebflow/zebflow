//! Table definition for SQL engines, built from the same attribute list the
//! studio collects for every engine.
//!
//! Identifiers reach the database as text rather than as bind parameters, so
//! every name is validated here before it is quoted. A name that is not a plain
//! identifier is refused rather than escaped.

use crate::platform::error::PlatformError;
use crate::platform::model::{CollectionAttribute, DbTypeDef, DbTypeFamily};

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

/// The types one SQL dialect offers, in the order the picker shows them.
///
/// Written in the engine's own words, because that is what its DDL takes and
/// what its catalog reports back. A generic set would have to be translated on
/// the way in and guessed at on the way out, and `timestamptz` has no generic
/// equivalent worth the loss.
pub fn type_catalog(dialect: SqlDialect) -> Vec<DbTypeDef> {
    let def = |name: &str, family: DbTypeFamily, parameterized: bool, note: &str| DbTypeDef {
        name: name.to_string(),
        family,
        parameterized,
        note: note.to_string(),
    };
    match dialect {
        SqlDialect::Postgres => vec![
            def("text", DbTypeFamily::Text, false, "unbounded"),
            def("varchar", DbTypeFamily::Text, true, "bounded"),
            def("char", DbTypeFamily::Text, true, "blank padded"),
            def("int2", DbTypeFamily::Number, false, "small integer"),
            def("int4", DbTypeFamily::Number, false, "integer"),
            def("int8", DbTypeFamily::Number, false, "big integer"),
            def("numeric", DbTypeFamily::Number, true, "exact decimal"),
            def("float4", DbTypeFamily::Number, false, "real"),
            def("float8", DbTypeFamily::Number, false, "double precision"),
            def("bool", DbTypeFamily::Boolean, false, ""),
            def("jsonb", DbTypeFamily::Json, false, "indexed json"),
            def("json", DbTypeFamily::Json, false, "stored as text"),
            def("uuid", DbTypeFamily::Uuid, false, ""),
            def("date", DbTypeFamily::DateTime, false, ""),
            def("time", DbTypeFamily::DateTime, false, ""),
            def("timestamp", DbTypeFamily::DateTime, false, "without time zone"),
            def("timestamptz", DbTypeFamily::DateTime, false, "with time zone"),
            def("interval", DbTypeFamily::DateTime, false, ""),
            def("bytea", DbTypeFamily::Binary, false, ""),
        ],
        SqlDialect::MySql => vec![
            def("varchar", DbTypeFamily::Text, true, "bounded"),
            def("char", DbTypeFamily::Text, true, "fixed width"),
            def("text", DbTypeFamily::Text, false, "needs a prefix to index"),
            def("mediumtext", DbTypeFamily::Text, false, ""),
            def("longtext", DbTypeFamily::Text, false, ""),
            def("tinyint", DbTypeFamily::Number, false, "also holds a boolean"),
            def("smallint", DbTypeFamily::Number, false, ""),
            def("int", DbTypeFamily::Number, false, ""),
            def("bigint", DbTypeFamily::Number, false, ""),
            def("decimal", DbTypeFamily::Number, true, "exact decimal"),
            def("float", DbTypeFamily::Number, false, ""),
            def("double", DbTypeFamily::Number, false, ""),
            def("boolean", DbTypeFamily::Boolean, false, "stored as tinyint(1)"),
            def("json", DbTypeFamily::Json, false, ""),
            def("date", DbTypeFamily::DateTime, false, ""),
            def("time", DbTypeFamily::DateTime, false, ""),
            def("datetime", DbTypeFamily::DateTime, false, ""),
            def("timestamp", DbTypeFamily::DateTime, false, ""),
            def("blob", DbTypeFamily::Binary, false, ""),
            def("varbinary", DbTypeFamily::Binary, true, ""),
        ],
        SqlDialect::Sqlite => vec![
            // SQLite has affinities rather than types; these are the five it
            // recognises plus the spellings it accepts and stores as one of them.
            def("TEXT", DbTypeFamily::Text, false, ""),
            def("INTEGER", DbTypeFamily::Number, false, "also holds a boolean"),
            def("REAL", DbTypeFamily::Number, false, ""),
            def("NUMERIC", DbTypeFamily::Number, false, ""),
            def("BLOB", DbTypeFamily::Binary, false, ""),
        ],
    }
}

/// Whether one requested type is in the dialect's catalog.
///
/// A parameterized type may carry its arguments — `varchar(255)`,
/// `numeric(10,2)` — which are checked for shape and passed through.
pub fn resolve_column_type(
    requested: &str,
    dialect: SqlDialect,
) -> Result<String, PlatformError> {
    let raw = requested.trim();
    if raw.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_KIND",
            "column type is empty",
        ));
    }
    let (base, args) = match raw.find('(') {
        Some(open) => {
            let close = raw.rfind(')').ok_or_else(|| {
                PlatformError::new(
                    "PLATFORM_DB_DDL_KIND",
                    format!("type '{raw}' is missing its closing bracket"),
                )
            })?;
            // Anything after the arguments is refused rather than dropped.
            // Discarding it quietly would accept `varchar(20); DROP TABLE x`
            // and report success for a request nobody made.
            if !raw[close + 1..].trim().is_empty() {
                return Err(PlatformError::new(
                    "PLATFORM_DB_DDL_KIND",
                    format!("type '{raw}' has trailing text after its arguments"),
                ));
            }
            (&raw[..open], Some(raw[open + 1..close].trim().to_string()))
        }
        None => (raw, None),
    };
    let base = base.trim();

    let catalog = type_catalog(dialect);
    let found = catalog
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(base))
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_DDL_KIND",
                format!("'{base}' is not a type this engine offers"),
            )
        })?;

    let Some(args) = args else {
        return Ok(found.name.clone());
    };
    if !found.parameterized {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_KIND",
            format!("type '{}' takes no arguments", found.name),
        ));
    }
    // Arguments reach the database as text, so only digits and one comma pass.
    if args.is_empty()
        || !args
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c.is_whitespace())
    {
        return Err(PlatformError::new(
            "PLATFORM_DB_DDL_KIND",
            format!("type arguments '{args}' must be numbers"),
        ));
    }
    let cleaned: String = args.chars().filter(|c| !c.is_whitespace()).collect();
    Ok(format!("{}({cleaned})", found.name))
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
fn column_type(kind: &str, dialect: SqlDialect) -> Result<String, PlatformError> {
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
        // Not one of the portable kinds, so it is read as the engine's own
        // type name and checked against that engine's catalog.
        (other, _) => return resolve_column_type(other, dialect),
    };
    Ok(ty.to_string())
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
                    if !indexed.contains(&(name.clone(), ty.clone())) {
                        indexed.push((name.clone(), ty.clone()));
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
        let target = if dialect == SqlDialect::MySql && ty.eq_ignore_ascii_case("text") {
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

/// The family one column type belongs to.
///
/// Types are compared by family rather than by spelling, because an engine
/// answers in its own words: MySQL reports `varchar(255)` where this platform
/// wrote `VARCHAR(255)`, and SQLite reports `INTEGER` for a column created as
/// boolean. Comparing the exact text would call those a type change and refuse
/// an edit that changes nothing.
///
/// SQLite has no boolean type at all — it stores one as an integer — so there
/// the two are one family. Refusing still happens across families, which is
/// where data would actually be at risk.
fn type_family(raw: &str, dialect: SqlDialect) -> &'static str {
    let value = raw.to_ascii_lowercase();
    if value.contains("json") {
        return "json";
    }
    let boolean = value.contains("bool");
    let numeric = value.contains("int")
        || value.contains("real")
        || value.contains("double")
        || value.contains("float")
        || value.contains("numeric")
        || value.contains("decimal");
    if boolean {
        return if dialect == SqlDialect::Sqlite {
            "number"
        } else {
            "boolean"
        };
    }
    if numeric {
        return "number";
    }
    "text"
}

/// One column as it exists in the database today.
pub struct ExistingColumn {
    pub name: String,
    /// The engine's own type name, used only to notice a change.
    pub column_type: String,
    /// Whether this platform's index for the column is present.
    pub indexed: bool,
}

/// The index name this platform gives one column.
pub fn index_name(table: &str, column: &str) -> String {
    format!("idx_{table}_{column}")
}

/// The statements that change one table to match a wanted set of attributes.
///
/// Columns are added and dropped, and indexes created and removed. A column
/// whose type changed is **refused**: rewriting a populated column can lose
/// what is in it, and silently keeping the old type would be a worse lie than
/// saying no.
pub fn alter_table_statements(
    table: &str,
    existing: &[ExistingColumn],
    desired: &[CollectionAttribute],
    dialect: SqlDialect,
) -> Result<Vec<String>, PlatformError> {
    let table = validate_identifier(table, "table")?;
    let quoted_table = dialect.quote(&table);
    let mut statements = Vec::new();

    let mut wanted: Vec<(String, String, bool)> = Vec::new();
    for attr in desired {
        let name = validate_identifier(&attr.name, "column")?;
        if name == IDENTITY_COLUMN {
            // The identity column is this platform's, not the author's.
            continue;
        }
        let ty = column_type(&attr.kind, dialect)?;
        let indexed = attr.index_types.iter().any(|index| {
            matches!(index.trim().to_ascii_lowercase().as_str(), "hash" | "range")
        });
        for index in &attr.index_types {
            match index.trim().to_ascii_lowercase().as_str() {
                "hash" | "range" | "" => {}
                other => {
                    return Err(PlatformError::new(
                        "PLATFORM_DB_DDL_INDEX",
                        format!(
                            "'{other}' indexes need a database extension this driver does not manage"
                        ),
                    ));
                }
            }
        }
        wanted.push((name, ty, indexed));
    }

    // Added and changed columns.
    for (name, ty, _) in &wanted {
        match existing.iter().find(|col| &col.name == name) {
            None => statements.push(format!(
                "ALTER TABLE {quoted_table} ADD COLUMN {} {ty}",
                dialect.quote(name)
            )),
            Some(current)
                if type_family(&current.column_type, dialect) != type_family(ty, dialect) =>
            {
                return Err(PlatformError::new(
                    "PLATFORM_DB_DDL_TYPE_CHANGE",
                    format!(
                        "column '{name}' is {} and cannot be changed to {ty} without risking its contents",
                        current.column_type
                    ),
                ));
            }
            Some(_) => {}
        }
    }

    // Columns the author removed.
    for current in existing {
        if current.name == IDENTITY_COLUMN {
            continue;
        }
        if !wanted.iter().any(|(name, _, _)| name == &current.name) {
            statements.push(format!(
                "ALTER TABLE {quoted_table} DROP COLUMN {}",
                dialect.quote(&current.name)
            ));
        }
    }

    // Index changes, by this platform's naming convention.
    for (name, ty, indexed) in &wanted {
        let already = existing
            .iter()
            .find(|col| &col.name == name)
            .map(|col| col.indexed)
            .unwrap_or(false);
        let index = dialect.quote(&index_name(&table, name));
        if *indexed && !already {
            let target = if dialect == SqlDialect::MySql && ty.eq_ignore_ascii_case("text") {
                format!("{}(191)", dialect.quote(name))
            } else {
                dialect.quote(name)
            };
            statements.push(format!(
                "CREATE INDEX {index} ON {quoted_table} ({target})"
            ));
        } else if !*indexed && already {
            statements.push(match dialect {
                // MySQL names the table when dropping an index; the others do not.
                SqlDialect::MySql => format!("DROP INDEX {index} ON {quoted_table}"),
                _ => format!("DROP INDEX {index}"),
            });
        }
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

    fn existing(name: &str, ty: &str, indexed: bool) -> ExistingColumn {
        ExistingColumn {
            name: name.to_string(),
            column_type: ty.to_string(),
            indexed,
        }
    }

    #[test]
    fn altering_adds_drops_and_reindexes_columns() {
        let statements = alter_table_statements(
            "orders",
            &[
                existing("id", "SERIAL", false),
                existing("label", "TEXT", true),
                existing("stale", "TEXT", false),
            ],
            &[attr("label", "string", &[]), attr("total", "number", &["hash"])],
            SqlDialect::Postgres,
        )
        .expect("statements");

        assert!(statements.iter().any(|s| s == "ALTER TABLE \"orders\" ADD COLUMN \"total\" DOUBLE PRECISION"));
        assert!(statements.iter().any(|s| s == "ALTER TABLE \"orders\" DROP COLUMN \"stale\""));
        // label lost its index, total gained one
        assert!(statements.iter().any(|s| s == "DROP INDEX \"idx_orders_label\""));
        assert!(statements.iter().any(|s| s.contains("CREATE INDEX \"idx_orders_total\"")));
        // the identity column is never dropped
        assert!(!statements.iter().any(|s| s.contains("DROP COLUMN \"id\"")));
    }

    #[test]
    fn a_type_the_engine_spells_differently_is_not_a_change() {
        // MySQL answers `varchar(255)` where this platform wrote VARCHAR(255).
        let statements = alter_table_statements(
            "t",
            &[existing("label", "varchar(255)", false)],
            &[attr("label", "string", &[])],
            SqlDialect::MySql,
        )
        .expect("no change");
        assert!(statements.is_empty(), "got {statements:?}");

        // SQLite stores a boolean as INTEGER, so reading it back is not a change.
        let statements = alter_table_statements(
            "t",
            &[existing("active", "INTEGER", false)],
            &[attr("active", "boolean", &[])],
            SqlDialect::Sqlite,
        )
        .expect("no change");
        assert!(statements.is_empty(), "got {statements:?}");
    }

    #[test]
    fn altering_refuses_a_type_change_rather_than_risk_the_contents() {
        let err = alter_table_statements(
            "orders",
            &[existing("total", "TEXT", false)],
            &[attr("total", "number", &[])],
            SqlDialect::Postgres,
        )
        .expect_err("should refuse");
        assert_eq!(err.code, "PLATFORM_DB_DDL_TYPE_CHANGE");
    }

    #[test]
    fn mysql_names_the_table_when_dropping_an_index() {
        let statements = alter_table_statements(
            "orders",
            &[existing("label", "VARCHAR(255)", true)],
            &[attr("label", "string", &[])],
            SqlDialect::MySql,
        )
        .expect("statements");
        assert_eq!(statements, vec!["DROP INDEX `idx_orders_label` ON `orders`"]);
    }

    #[test]
    fn an_engines_own_type_names_are_accepted_and_checked() {
        // PostgreSQL's own words, straight through.
        let statements = create_table_statements(
            "t",
            &[attr("ref", "uuid", &[]), attr("seen", "timestamptz", &[])],
            SqlDialect::Postgres,
        )
        .expect("statements");
        assert!(statements[0].contains("\"ref\" uuid"), "got {}", statements[0]);
        assert!(statements[0].contains("\"seen\" timestamptz"));

        // Arguments are kept when the type takes them.
        let statements =
            create_table_statements("t", &[attr("code", "varchar(20)", &[])], SqlDialect::Postgres)
                .expect("statements");
        assert!(statements[0].contains("\"code\" varchar(20)"), "got {}", statements[0]);

        // A type another engine has is not silently accepted here.
        let err = create_table_statements("t", &[attr("c", "jsonb", &[])], SqlDialect::MySql)
            .expect_err("mysql has no jsonb");
        assert_eq!(err.code, "PLATFORM_DB_DDL_KIND");

        // Arguments are numbers, never pasted text.
        let err = create_table_statements(
            "t",
            &[attr("c", "varchar(20); DROP TABLE x", &[])],
            SqlDialect::Postgres,
        )
        .expect_err("should refuse");
        assert_eq!(err.code, "PLATFORM_DB_DDL_KIND");
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
