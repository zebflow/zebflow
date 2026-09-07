//! What a SQL statement does, decided from the statement itself.
//!
//! The query endpoint used to take the caller's word for it. A request carried
//! `read_only`, the route checked only `TablesRead`, and nothing compared the
//! flag against the SQL — so `{"sql": "DELETE FROM posts", "read_only": false}`
//! ran for anyone who could read a table. The role ladder already said
//! Reporters read and Maintainers write; the endpoint simply did not ask.
//!
//! A client may still send `read_only: true` to ask for a stricter run than its
//! capabilities require. It may not send `false` to ask for a looser one.

/// Whether a statement changes data or schema.
///
/// Reads the first keyword, which is what every SQL dialect Zebflow drives
/// agrees on. Deliberately conservative: a statement this does not recognise is
/// treated as a write, because being wrong in that direction refuses a query
/// that should have run, and being wrong in the other direction runs a `DELETE`
/// for somebody who may only read.
pub fn statement_writes(sql: &str) -> bool {
    let Some(first) = leading_keyword(sql) else {
        // Nothing recognisable. Not a read.
        return true;
    };
    !matches!(
        first.as_str(),
        "SELECT" | "WITH" | "SHOW" | "EXPLAIN" | "DESCRIBE" | "DESC" | "PRAGMA" | "VALUES"
    )
}

/// The first bare keyword, skipping leading comments and parentheses.
///
/// `-- drop everything\nSELECT 1` is a read, and `(SELECT 1)` is a read. A
/// scanner that only looked at `split_whitespace().next()` called the first one
/// a write and the second one unrecognised.
fn leading_keyword(sql: &str) -> Option<String> {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("--") {
            rest = after.find('\n').map_or("", |at| &after[at + 1..]).trim_start();
            continue;
        }
        if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |at| &after[at + 2..]).trim_start();
            continue;
        }
        if let Some(after) = rest.strip_prefix('(') {
            rest = after.trim_start();
            continue;
        }
        break;
    }
    let word: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    (!word.is_empty()).then(|| word.to_ascii_uppercase())
}

/// The capability a statement needs, decided from the statement.
///
/// Kept here beside `statement_writes` rather than inline in the route, so the
/// rule that a `DELETE` needs `TablesWrite` is a thing that can be tested
/// without standing up an HTTP request and a session.
pub fn capability_for_statement(sql: &str) -> crate::platform::model::ProjectCapability {
    use crate::platform::model::ProjectCapability;
    if statement_writes(sql) {
        ProjectCapability::TablesWrite
    } else {
        ProjectCapability::TablesRead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_recognised_as_reads() {
        for sql in [
            "SELECT 1",
            "  select * from posts",
            "WITH t AS (SELECT 1) SELECT * FROM t",
            "(SELECT 1)",
            "-- a comment about DELETE\nSELECT 1",
            "/* DROP TABLE posts */ SELECT 1",
            "EXPLAIN SELECT 1",
            "PRAGMA table_info(posts)",
        ] {
            assert!(!statement_writes(sql), "expected a read: {sql:?}");
        }
    }

    #[test]
    fn writes_are_recognised_as_writes() {
        for sql in [
            "DELETE FROM posts",
            "delete from posts",
            "INSERT INTO posts VALUES (1)",
            "UPDATE posts SET title = 'x'",
            "DROP TABLE posts",
            "ALTER TABLE posts ADD COLUMN x TEXT",
            "TRUNCATE posts",
            "GRANT ALL ON posts TO someone",
            "-- looks harmless\nDELETE FROM posts",
        ] {
            assert!(statement_writes(sql), "expected a write: {sql:?}");
        }
    }

    /// A reader cannot delete.
    ///
    /// The query endpoint took the caller's word for this. A request carried
    /// `read_only`, the route checked only `TablesRead`, and nothing compared
    /// the flag to the SQL — so a Reporter, whose whole point is reading, could
    /// send `DELETE FROM posts` and have it run.
    #[test]
    fn a_write_statement_needs_the_write_capability() {
        use crate::platform::model::{ProjectAccessRolePreset, ProjectCapability};
        use crate::platform::services::access::roles::role_capabilities;

        assert_eq!(
            capability_for_statement("DELETE FROM posts"),
            ProjectCapability::TablesWrite
        );
        assert_eq!(
            capability_for_statement("SELECT * FROM posts"),
            ProjectCapability::TablesRead
        );

        // And the ladder must actually withhold it, or the check buys nothing.
        let reporter = role_capabilities(ProjectAccessRolePreset::Reporter);
        assert!(reporter.contains(&ProjectCapability::TablesRead));
        assert!(
            !reporter.contains(&ProjectCapability::TablesWrite),
            "a Reporter holding TablesWrite would make the statement check pointless"
        );

        // A Developer builds and runs, and still does not edit rows by hand.
        let developer = role_capabilities(ProjectAccessRolePreset::Developer);
        assert!(!developer.contains(&ProjectCapability::TablesWrite));
    }

    /// The safe direction for something unreadable is "write".
    #[test]
    fn anything_unrecognised_counts_as_a_write() {
        for sql in ["", "   ", ";;;", "42", "🙂"] {
            assert!(statement_writes(sql), "expected a write: {sql:?}");
        }
    }
}
