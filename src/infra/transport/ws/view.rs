//! What a session is sent of a room's state (`docs/contracts/rooms.md`
//! § Views, § Clients and upgrades): only its part, and after `joined`,
//! only what changed.
//!
//! [`Visibility`] holds the room's `--visible-to` rules: a path written
//! with `--visible-to team:red` (and everything under it) is part only of
//! the views of sessions holding one of its tags. A path under no rule is
//! in every view. Hidden values are cut out of a view before it is
//! encoded, so their bytes never leave the server.
//!
//! [`diff`] compares the view a session was last sent with the new one and
//! answers the smallest list of operations that turns one into the other:
//! an unchanged subtree sends nothing, a changed leaf sends one `set`, a key
//! that is gone sends one `del`. A list is a leaf: a changed list is sent
//! whole.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

/// The room's `--visible-to` rules: a path, as its keys, and the tags that
/// may see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Visibility {
    rules: BTreeMap<Vec<String>, BTreeSet<String>>,
    /// Every tag some rule names, with how many rules name it.
    named: BTreeMap<String, usize>,
}

/// A session's tags that some rule names: two sessions with the same ones
/// see the same view.
pub type ViewKey = Vec<String>;

impl Visibility {
    /// `ws.state.put` at `path`: what was under it is replaced, so its rules
    /// go; `tags` (if any) limit the path, none makes it visible to all.
    pub fn put(&mut self, path: &[&str], tags: &[String]) {
        self.forget(path);
        self.limit(path, tags);
    }

    /// `ws.state.update` at `path` merging `keys`: each merged key's value is
    /// replaced, so its rules go; `tags` (if any) limit the path, none keeps
    /// what limits it now.
    pub fn update<'a>(&mut self, path: &[&str], keys: impl Iterator<Item = &'a str>, tags: &[String]) {
        for key in keys {
            let mut child = path.to_vec();
            child.push(key);
            self.forget(&child);
        }
        self.limit(path, tags);
    }

    /// `ws.state.delete` at `path`: the rules at and under it go.
    pub fn delete(&mut self, path: &[&str]) {
        self.forget(path);
    }

    fn limit(&mut self, path: &[&str], tags: &[String]) {
        if !tags.is_empty() && !path.is_empty() {
            self.insert(owned(path), tags.iter().cloned().collect());
        }
    }

    fn insert(&mut self, path: Vec<String>, tags: BTreeSet<String>) {
        for tag in &tags {
            *self.named.entry(tag.clone()).or_default() += 1;
        }
        if let Some(gone) = self.rules.insert(path, tags) {
            self.unname(&gone);
        }
    }

    fn unname(&mut self, tags: &BTreeSet<String>) {
        for tag in tags {
            if let Some(count) = self.named.get_mut(tag) {
                *count -= 1;
                if *count == 0 {
                    self.named.remove(tag);
                }
            }
        }
    }

    /// Drop every rule at or under `path`.
    fn forget(&mut self, path: &[&str]) {
        self.forget_under(&owned(path));
    }

    fn forget_under(&mut self, start: &[String]) {
        let under: Vec<Vec<String>> = self.rules.range(start.to_vec()..).take_while(|(key, _)| key.starts_with(start)).map(|(key, _)| key.clone()).collect();
        for key in under {
            if let Some(tags) = self.rules.remove(&key) {
                self.unname(&tags);
            }
        }
    }

    /// The tags that may see `path`, when a rule limits it there.
    pub fn rule(&self, path: &[&str]) -> Option<&BTreeSet<String>> {
        self.rules.get(&owned(path))
    }

    /// Make the rules at and under `path` the ones `from` has there.
    pub fn copy_under(&mut self, path: &[String], from: &Visibility) {
        self.forget_under(path);
        for (key, tags) in from.rules.range(path.to_vec()..).take_while(|(key, _)| key.starts_with(path)) {
            self.insert(key.clone(), tags.clone());
        }
    }

    /// Whether any rule lies at or under `path`.
    fn any_under(&self, path: &[String]) -> bool {
        self.rules.range(path.to_vec()..).next().is_some_and(|(key, _)| key.starts_with(path))
    }

    /// The tags among `tags` that some rule names.
    pub fn key(&self, tags: &BTreeSet<String>) -> ViewKey {
        tags.iter().filter(|tag| self.named.contains_key(*tag)).cloned().collect()
    }

    /// `state` as a session whose [`Visibility::key`] is `key` sees it.
    pub fn view(&self, state: &Value, key: &ViewKey) -> Value {
        if self.rules.is_empty() {
            return state.clone();
        }
        let mut path = Vec::new();
        self.view_at(state, key, &mut path)
    }

    /// The value at `path` as a session whose key is `key` sees it: `None`
    /// when nothing is there or a rule at or above `path` hides it.
    pub fn view_under(&self, state: &Value, path: &[String], key: &ViewKey) -> Option<Value> {
        let mut at = state;
        for (depth, part) in path.iter().enumerate() {
            at = at.as_object()?.get(part)?;
            if self.rules.get(&path[..=depth]).is_some_and(|tags| !key.iter().any(|tag| tags.contains(tag))) {
                return None;
            }
        }
        if self.rules.is_empty() {
            return Some(at.clone());
        }
        Some(self.view_at(at, key, &mut path.to_vec()))
    }

    fn view_at(&self, value: &Value, key: &ViewKey, path: &mut Vec<String>) -> Value {
        let Value::Object(map) = value else { return value.clone() };
        let mut out = Map::new();
        for (name, child) in map {
            path.push(name.clone());
            if !self.any_under(path) {
                out.insert(name.clone(), child.clone());
            } else if self.rules.get(path.as_slice()).is_none_or(|tags| key.iter().any(|tag| tags.contains(tag))) {
                out.insert(name.clone(), self.view_at(child, key, path));
            }
            path.pop();
        }
        Value::Object(out)
    }
}

fn owned(path: &[&str]) -> Vec<String> {
    path.iter().map(|part| part.to_string()).collect()
}

/// One change of a view: `{ "op": "set", "path": [...], "value": … }` or
/// `{ "op": "del", "path": [...] }`. `path` is the keys from the root.
pub fn set_op(path: &[&str], value: &Value) -> Value {
    json!({ "op": "set", "path": path, "value": value })
}

/// See [`set_op`].
pub fn del_op(path: &[&str]) -> Value {
    json!({ "op": "del", "path": path })
}

/// The operations that turn `old` into `new`: in each object, the keys that
/// are gone first, then the new object's keys in its own order.
pub fn diff(old: &Value, new: &Value) -> Vec<Value> {
    let mut ops = Vec::new();
    let mut path = Vec::new();
    diff_at(old, new, &mut path, &mut ops);
    ops
}

/// The operations that turn the value at `path` from `old` into `new`,
/// each `None` where nothing is.
pub fn diff_under(path: &[String], old: Option<&Value>, new: Option<&Value>, ops: &mut Vec<Value>) {
    let mut keys: Vec<&str> = path.iter().map(String::as_str).collect();
    match (old, new) {
        (None, None) => {}
        (Some(_), None) => ops.push(del_op(&keys)),
        (None, Some(new)) => ops.push(set_op(&keys, new)),
        (Some(old), Some(new)) => diff_at(old, new, &mut keys, ops),
    }
}

fn diff_at<'a>(old: &'a Value, new: &'a Value, path: &mut Vec<&'a str>, ops: &mut Vec<Value>) {
    match (old, new) {
        (Value::Object(old), Value::Object(new)) => diff_objects(old, new, path, ops),
        _ if old == new => {}
        // The root is always an object; a different value anywhere else is set.
        _ => ops.push(set_op(path, new)),
    }
}

fn diff_objects<'a>(old: &'a Map<String, Value>, new: &'a Map<String, Value>, path: &mut Vec<&'a str>, ops: &mut Vec<Value>) {
    for key in old.keys().filter(|key| !new.contains_key(*key)) {
        path.push(key);
        ops.push(del_op(path));
        path.pop();
    }
    for (key, value) in new {
        path.push(key);
        match old.get(key) {
            Some(before) => diff_at(before, value, path, ops),
            None => ops.push(set_op(path, value)),
        }
        path.pop();
    }
}

/// Apply operations to a view, as a client does.
#[cfg(test)]
pub fn apply(view: &mut Value, ops: &[Value]) {
    for op in ops {
        let path: Vec<&str> = op["path"].as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
        let Some((last, above)) = path.split_last() else { continue };
        let mut at = &mut *view;
        for key in above {
            at = at.as_object_mut().unwrap().entry(key.to_string()).or_insert_with(|| json!({}));
        }
        let map = at.as_object_mut().unwrap();
        match op["op"].as_str() {
            Some("set") => {
                map.insert(last.to_string(), op["value"].clone());
            }
            _ => {
                map.remove(*last);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_subtree_sends_nothing_and_a_changed_leaf_one_set() {
        let old = json!({ "players": { "s1": { "x": 1, "y": 2 }, "s2": { "x": 5, "y": 5 } }, "map": { "w": 64, "tiles": [1, 2, 3] } });
        let mut new = old.clone();
        new["players"]["s1"]["x"] = json!(2);
        assert_eq!(diff(&old, &new), vec![json!({ "op": "set", "path": ["players", "s1", "x"], "value": 2 })]);
        assert!(diff(&old, &old).is_empty());
    }

    #[test]
    fn a_gone_key_is_one_del_a_new_one_one_set_and_a_list_is_a_leaf() {
        let old = json!({ "players": { "s1": { "x": 1 }, "s2": { "x": 5 } }, "log": [1, 2] });
        let new = json!({ "players": { "s1": { "x": 1 }, "s3": { "x": 0 } }, "log": [1, 2, 3], "round": 2 });
        let ops = diff(&old, &new);
        assert_eq!(
            ops,
            vec![
                json!({ "op": "del", "path": ["players", "s2"] }),
                json!({ "op": "set", "path": ["players", "s3"], "value": { "x": 0 } }),
                json!({ "op": "set", "path": ["log"], "value": [1, 2, 3] }),
                json!({ "op": "set", "path": ["round"], "value": 2 }),
            ]
        );
        let mut client = old.clone();
        apply(&mut client, &ops);
        assert_eq!(client, new, "a client applying the ops holds the new view");
    }

    fn tags(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn a_limited_path_and_all_under_it_is_only_in_the_views_of_its_tags() {
        let state = json!({ "board": { "turn": 2 }, "hands": { "s1": { "cards": [1, 2] }, "s2": { "cards": [3] } } });
        let mut vis = Visibility::default();
        vis.put(&["hands", "s1"], &["seat:1".to_string()]);
        vis.put(&["hands", "s2"], &["seat:2".to_string(), "dealer".to_string()]);
        let view = |held: &[&str]| vis.view(&state, &vis.key(&tags(held)));
        assert_eq!(view(&[]), json!({ "board": { "turn": 2 }, "hands": {} }), "untagged: neither hand");
        assert_eq!(view(&["seat:1", "team:red"]), json!({ "board": { "turn": 2 }, "hands": { "s1": { "cards": [1, 2] } } }));
        assert_eq!(view(&["dealer"])["hands"], json!({ "s2": { "cards": [3] } }), "one of its tags is enough");
        assert_eq!(vis.key(&tags(&["seat:1", "team:red"])), vec!["seat:1".to_string()], "a tag no rule names makes no other view");
    }

    #[test]
    fn a_put_replaces_the_rules_under_it_an_update_keeps_its_own_and_a_delete_drops_them() {
        let mut vis = Visibility::default();
        let red = vec!["team:red".to_string()];
        vis.put(&["units", "u1"], &red);
        vis.update(&["units", "u1"], ["x"].into_iter(), &[]);
        assert_eq!(vis.view(&json!({ "units": { "u1": { "x": 1 } } }), &Vec::new()), json!({ "units": {} }), "an update without --visible-to keeps the limit");
        vis.put(&["units", "u1"], &[]);
        assert_eq!(vis, Visibility::default(), "a put without --visible-to makes it visible to all");
        vis.put(&["units", "u1", "hp"], &red);
        vis.put(&["units"], &[]);
        assert_eq!(vis, Visibility::default(), "a put over a parent replaces what was under it");
        vis.put(&["units", "u2"], &red);
        vis.delete(&["units"]);
        assert_eq!(vis, Visibility::default());
    }

    #[test]
    fn a_value_that_changes_type_is_set_whole() {
        let ops = diff(&json!({ "a": { "b": 1 } }), &json!({ "a": 7 }));
        assert_eq!(ops, vec![json!({ "op": "set", "path": ["a"], "value": 7 })]);
    }
}
