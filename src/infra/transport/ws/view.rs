//! What a session is sent of a room's state (`docs/contracts/rooms.md`
//! § Clients and upgrades): after `joined`, only what changed.
//!
//! [`diff`] compares the view a session was last sent with the new one and
//! answers the smallest list of operations that turns one into the other:
//! an unchanged subtree sends nothing, a changed leaf sends one `set`, a key
//! that is gone sends one `del`. A list is a leaf: a changed list is sent
//! whole.

use serde_json::{Map, Value, json};

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

    #[test]
    fn a_value_that_changes_type_is_set_whole() {
        let ops = diff(&json!({ "a": { "b": 1 } }), &json!({ "a": 7 }));
        assert_eq!(ops, vec![json!({ "op": "set", "path": ["a"], "value": 7 })]);
    }
}
