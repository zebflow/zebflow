//! Room names and the patterns `trigger.room --room` declares
//! (`docs/contracts/rooms.md` § Two kinds of room).
//!
//! | Written | Is | Matches |
//! |---|---|---|
//! | `lobby` | a fixed room | `lobby` |
//! | `match-:id` | a pattern | `match-` and one or more name characters |
//! | `zone-*`, `*-eu` | a pattern | the literal parts around one or more name characters |
//!
//! A room name is 1–128 of `A-Z a-z 0-9 - _ .`; it is one URL segment of
//! `/ws/{o}/{p}/rooms/{room}`. A pattern has one `*`, or one `:param` at its
//! end, and some literal part: `*` or `:id` alone would be every room, and
//! no visitor may make a room by naming one.

/// Longest room name.
pub const MAX_ROOM_NAME: usize = 128;

/// One `trigger.room --room` value, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomPattern {
    /// An exact name: a fixed room.
    Exact(String),
    /// `prefix` + one or more name characters + `suffix`.
    Wild { prefix: String, suffix: String },
}

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// Whether `name` is a room name (not a pattern).
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_ROOM_NAME && name.chars().all(name_char)
}

/// Why a name or a pattern was refused.
pub fn name_problem(name: &str) -> String {
    format!(
        "'{name}' is not a room name: 1–{MAX_ROOM_NAME} of letters, digits, '-', '_' and '.'"
    )
}

impl RoomPattern {
    /// Read a `--room` value; an error says what is wrong with it.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("--room is required: name the room or pattern (e.g. lobby, match-:id, zone-*)".to_string());
        }
        let wild = |prefix: &str, suffix: &str| -> Result<Self, String> {
            if prefix.is_empty() && suffix.is_empty() {
                return Err(format!(
                    "'{text}' alone is every room, and is not a room: give it a literal part (match-:id, zone-*)"
                ));
            }
            if !prefix.chars().chain(suffix.chars()).all(name_char) || text.len() > MAX_ROOM_NAME {
                return Err(name_problem(text));
            }
            Ok(Self::Wild { prefix: prefix.to_string(), suffix: suffix.to_string() })
        };
        if let Some((prefix, suffix)) = text.split_once('*') {
            if suffix.contains('*') || suffix.contains(':') || prefix.contains(':') {
                return Err(format!("'{text}': a pattern has one '*' or one ':param'"));
            }
            return wild(prefix, suffix);
        }
        if let Some((prefix, param)) = text.split_once(':') {
            if param.is_empty() || !param.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("'{text}': ':{param}' is not a parameter; a ':param' ends the pattern (match-:id)"));
            }
            return wild(prefix, "");
        }
        if !valid_name(text) {
            return Err(name_problem(text));
        }
        Ok(Self::Exact(text.to_string()))
    }

    /// Whether this is a pattern (opened rooms) rather than a fixed room.
    pub fn is_pattern(&self) -> bool {
        matches!(self, Self::Wild { .. })
    }

    /// Whether the room `name` is this one or matches this pattern.
    pub fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exact(exact) => exact == name,
            Self::Wild { prefix, suffix } => {
                valid_name(name)
                    && name.len() > prefix.len() + suffix.len()
                    && name.starts_with(prefix.as_str())
                    && name.ends_with(suffix.as_str())
            }
        }
    }

    /// Whether some room matches both: two triggers that can serve the same
    /// room. Both literal ends must agree; then the longer prefix, a name
    /// character and the longer suffix is a room both match.
    pub fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(a), b) | (b, Self::Exact(a)) => b.matches(a),
            (Self::Wild { prefix: p1, suffix: s1 }, Self::Wild { prefix: p2, suffix: s2 }) => {
                let prefix = if p1.len() >= p2.len() { p1 } else { p2 };
                let suffix = if s1.len() >= s2.len() { s1 } else { s2 };
                if !prefix.starts_with(p1.as_str()) || !prefix.starts_with(p2.as_str()) {
                    return false;
                }
                if !suffix.ends_with(s1.as_str()) || !suffix.ends_with(s2.as_str()) {
                    return false;
                }
                let witness = format!("{prefix}x{suffix}");
                self.matches(&witness) && other.matches(&witness)
            }
        }
    }
}

/// Whether `name` matches a selection: an exact name, a pattern, or `*`
/// (every room — a selection, never a room). An invalid selection matches nothing.
pub fn selects(selection: &str, name: &str) -> bool {
    let selection = selection.trim();
    if selection == "*" {
        return true;
    }
    RoomPattern::parse(selection).is_ok_and(|p| p.matches(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> RoomPattern {
        RoomPattern::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    #[test]
    fn exact_names_and_patterns_are_told_apart() {
        assert_eq!(p("lobby"), RoomPattern::Exact("lobby".into()));
        assert!(p("match-:id").is_pattern());
        assert!(p("zone-*").is_pattern());
        assert!(p("*-eu").is_pattern());
        for refused in ["", "*", ":id", "a*b*c", "a:id-x", "a:", "room one", "a/b", "match-:id*"] {
            assert!(RoomPattern::parse(refused).is_err(), "{refused:?} must be refused");
        }
    }

    #[test]
    fn a_pattern_matches_one_or_more_name_characters() {
        assert!(p("match-:id").matches("match-42"));
        assert!(!p("match-:id").matches("match-"));
        assert!(!p("match-:id").matches("lobby"));
        assert!(p("zone-*").matches("zone-forest-3"));
        assert!(p("*-eu").matches("match-eu"));
        assert!(!p("zone-*").matches("zone-a/b"), "a name is one URL segment");
        assert!(p("lobby").matches("lobby") && !p("lobby").matches("lobby-2"));
    }

    #[test]
    fn overlap_is_some_room_both_match() {
        assert!(p("match-:id").overlaps(&p("match-*")));
        assert!(p("match-*").overlaps(&p("*-eu")), "match-x-eu");
        assert!(p("m*").overlaps(&p("match-:id")));
        assert!(p("lobby").overlaps(&p("lob*")));
        assert!(!p("lobby").overlaps(&p("match-*")));
        assert!(!p("match-*").overlaps(&p("zone-*")));
        assert!(!p("a*-eu").overlaps(&p("b*")));
        assert!(!p("lobby").overlaps(&p("arena")));
    }

    #[test]
    fn a_selection_may_be_everything() {
        assert!(selects("*", "lobby"));
        assert!(selects("zone-*", "zone-1"));
        assert!(!selects("zone-*", "lobby"));
        assert!(selects("lobby", "lobby"));
    }
}
