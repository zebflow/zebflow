//! `{{ }}` without JavaScript: compiled once, evaluated by Rust, with the
//! answer Deno would give.
//!
//! The language is the part of JavaScript expressions are made of: the scope
//! names (`input`, `$input`, `$item`, `$index`, `$count`, `$trigger`,
//! `$nodes`, `$placeholder`), member access (`a.b`, `a[0]`, `a["k"]`,
//! `a?.b`), literals, object and array literals, `+ - * / %`, comparisons,
//! `=== !== == !=`, `&& || ?? !`, `typeof`, unary `-` `+`, and `? :`.
//! [`compile`] refuses anything else (a call, an arrow function, a global
//! such as `Math`), and the caller sends that expression to Deno.
//!
//! The answer is Deno's answer, not an approximation of it:
//!
//! - `undefined` and `null` stay apart until the end, where `undefined`
//!   becomes `null` as `JSON.stringify` makes it (and drops an object key
//!   holding it);
//! - numbers are doubles: `7 * 2` is `14`, `7 / 2` is `3.5`, and a result
//!   leaves as JSON prints it (an integral double as an integer, `NaN` as
//!   `null`, an integer past 2^53 rounded as JavaScript rounds it);
//! - an object's keys leave in JavaScript's order, integer-like keys first;
//! - strings compare and count by UTF-16 unit.
//!
//! Where an exact answer cannot be had here, [`Expr::eval`] answers
//! [`Fallback`] and the caller asks Deno: an expression that would throw (so
//! the error is Deno's own message), objects compared by identity, a string
//! made from an object, a number JavaScript prints with an exponent.

use std::borrow::Cow;
use std::cmp::Ordering;

use serde_json::{Map, Number, Value};

/// This expression, with these values, needs Deno for an exact answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fallback;

type R<T> = Result<T, Fallback>;

/// The names an expression may start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    Input,
    Item,
    Index,
    Count,
    Trigger,
    Nodes,
    Placeholder,
}

/// What a scope binds. `{{ }}` binds all of them; a script's body binds
/// only `input`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Names {
    Expression,
    Script,
}

impl Root {
    fn named(name: &str, names: Names) -> Option<Root> {
        Some(match (name, names) {
            ("input", _) => Root::Input,
            ("$input", Names::Expression) => Root::Input,
            ("$item", Names::Expression) => Root::Item,
            ("$index", Names::Expression) => Root::Index,
            ("$count", Names::Expression) => Root::Count,
            ("$trigger", Names::Expression) => Root::Trigger,
            ("$nodes", Names::Expression) => Root::Nodes,
            ("$placeholder", Names::Expression) => Root::Placeholder,
            _ => return None,
        })
    }
}

/// The values the scope names stand for, borrowed from the run.
pub struct Scope<'a> {
    pub input: &'a Value,
    pub item: &'a Value,
    pub index: &'a Value,
    pub count: &'a Value,
    pub trigger: &'a Value,
    pub nodes: &'a Value,
    pub placeholder: &'a Value,
}

static NULL: Value = Value::Null;
static EMPTY: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| Value::Object(Map::new()));

impl<'a> Scope<'a> {
    /// The expression scope of a node: the same bindings as the Deno batch
    /// (`$trigger || null`, `$nodes || {}`, `$placeholder || {}`, and the
    /// loop fields from the metadata's foreach frame, else the input's).
    pub fn of_node(input: &'a Value, metadata: &'a Value) -> Scope<'a> {
        let run = metadata.get(super::resolver::FOREACH_METADATA_KEY).filter(|v| v.is_object());
        let loop_field = |name: &str| run.and_then(|r| r.get(name)).or_else(|| input.get(name)).unwrap_or(&NULL);
        let or = |v: Option<&'a Value>, otherwise: &'a Value| match v {
            Some(v) if truthy_json(v) => v,
            _ => otherwise,
        };
        Scope {
            input,
            item: loop_field("item"),
            index: loop_field("index"),
            count: loop_field("count"),
            trigger: or(metadata.get("trigger"), &NULL),
            nodes: or(metadata.get("nodes"), &*EMPTY),
            placeholder: or(metadata.get("placeholder"), &*EMPTY),
        }
    }

    /// A script body's scope: `input` only.
    pub fn of_script(input: &'a Value) -> Scope<'a> {
        Scope { input, item: &NULL, index: &NULL, count: &NULL, trigger: &NULL, nodes: &*EMPTY, placeholder: &*EMPTY }
    }

    fn root(&self, root: Root) -> &'a Value {
        match root {
            Root::Input => self.input,
            Root::Item => self.item,
            Root::Index => self.index,
            Root::Count => self.count,
            Root::Trigger => self.trigger,
            Root::Nodes => self.nodes,
            Root::Placeholder => self.placeholder,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Key {
    Name(String),
    Index(usize),
    Computed(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Step {
    key: Key,
    /// `?.`: a nullish base ends the whole chain as `undefined`.
    optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Lt,
    Le,
    Gt,
    Ge,
    StrictEq,
    StrictNe,
    LooseEq,
    LooseNe,
    And,
    Or,
    Nullish,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Path(Root, Vec<Step>),
    /// A member chain on something other than a scope name: `(a || b).c`.
    Member(Box<Expr>, Vec<Step>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Plus(Box<Expr>),
    TypeOf(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Object(Vec<(String, Expr)>),
    Array(Vec<Expr>),
}

/// A JavaScript value during evaluation. Objects and arrays stay borrowed
/// JSON; primitives are JavaScript's.
#[derive(Debug, Clone)]
pub enum Js<'a> {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(Cow<'a, str>),
    /// An object or array (never a primitive).
    Json(Cow<'a, Value>),
}

impl<'a> Js<'a> {
    fn of(v: &'a Value) -> Js<'a> {
        match v {
            Value::Null => Js::Null,
            Value::Bool(b) => Js::Bool(*b),
            Value::Number(n) => Js::Num(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => Js::Str(Cow::Borrowed(s)),
            other => Js::Json(Cow::Borrowed(other)),
        }
    }

    fn of_owned(v: Value) -> Js<'a> {
        match v {
            Value::Null => Js::Null,
            Value::Bool(b) => Js::Bool(b),
            Value::Number(n) => Js::Num(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => Js::Str(Cow::Owned(s)),
            other => Js::Json(Cow::Owned(other)),
        }
    }

    fn nullish(&self) -> bool {
        matches!(self, Js::Undefined | Js::Null)
    }

    fn truthy(&self) -> bool {
        match self {
            Js::Undefined | Js::Null => false,
            Js::Bool(b) => *b,
            Js::Num(n) => *n != 0.0 && !n.is_nan(),
            Js::Str(s) => !s.is_empty(),
            Js::Json(_) => true,
        }
    }

    /// The value as `JSON.stringify` then `serde_json` would leave it:
    /// `undefined` is `None`.
    pub fn into_json(self) -> Option<Value> {
        Some(match self {
            Js::Undefined => return None,
            Js::Null => Value::Null,
            Js::Bool(b) => Value::Bool(b),
            Js::Num(n) => number_json(n),
            Js::Str(s) => Value::String(s.into_owned()),
            Js::Json(v) => match as_js_prints(&v) {
                None => v.into_owned(),
                Some(v) => v,
            },
        })
    }
}

fn truthy_json(v: &Value) -> bool {
    Js::of(v).truthy()
}

/// A double as JSON prints it and `serde_json` reads it back.
fn number_json(n: f64) -> Value {
    if !n.is_finite() {
        return Value::Null;
    }
    // JSON prints an integral double below 1e21 as digits, which serde
    // reads back as an integer when it fits one.
    if n.fract() == 0.0 && n.abs() < 1e21 {
        if (0.0..18_446_744_073_709_551_616.0).contains(&n) {
            return Value::Number((n as u64).into());
        }
        if (-9_223_372_036_854_775_808.0..0.0).contains(&n) {
            return Value::Number((n as i64).into());
        }
    }
    Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)
}

/// `Some(copy)` when the value would not come back from JavaScript
/// unchanged: a number not in the form JSON prints it, or an object whose
/// integer-like keys are not first and ascending. `None` when it would.
fn as_js_prints(v: &Value) -> Option<Value> {
    if prints_as_is(v) {
        return None;
    }
    Some(reprint(v))
}

fn prints_as_is(v: &Value) -> bool {
    match v {
        Value::Number(n) => number_json(n.as_f64().unwrap_or(f64::NAN)) == *v,
        Value::Array(a) => a.iter().all(prints_as_is),
        Value::Object(m) => keys_in_js_order(m) && m.values().all(prints_as_is),
        _ => true,
    }
}

fn reprint(v: &Value) -> Value {
    match v {
        Value::Number(n) => number_json(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => Value::Array(a.iter().map(reprint).collect()),
        Value::Object(m) => {
            let mut index: Vec<(u32, &String)> = m.keys().filter_map(|k| array_index(k).map(|i| (i, k))).collect();
            index.sort();
            let mut out = Map::with_capacity(m.len());
            for (_, k) in &index {
                out.insert((*k).clone(), reprint(&m[k.as_str()]));
            }
            for (k, v) in m {
                if array_index(k).is_none() {
                    out.insert(k.clone(), reprint(v));
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// `"0"`, `"7"`, `"42"`: the keys JavaScript orders first.
fn array_index(k: &str) -> Option<u32> {
    if k.is_empty() || (k.len() > 1 && k.starts_with('0')) || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    k.parse::<u32>().ok().filter(|i| *i != u32::MAX)
}

fn keys_in_js_order(m: &Map<String, Value>) -> bool {
    let mut last: Option<u32> = None;
    let mut seen_name = false;
    for k in m.keys() {
        match array_index(k) {
            Some(i) => {
                if seen_name || last.is_some_and(|l| i <= l) {
                    return false;
                }
                last = Some(i);
            }
            None => seen_name = true,
        }
    }
    true
}

impl Expr {
    pub fn eval<'a>(&'a self, s: &Scope<'a>) -> R<Js<'a>> {
        Ok(match self {
            Expr::Undefined => Js::Undefined,
            Expr::Null => Js::Null,
            Expr::Bool(b) => Js::Bool(*b),
            Expr::Num(n) => Js::Num(*n),
            Expr::Str(t) => Js::Str(Cow::Borrowed(t)),
            Expr::Path(root, steps) => walk(Js::of(s.root(*root)), steps, s)?,
            Expr::Member(base, steps) => walk(base.eval(s)?, steps, s)?,
            Expr::Not(e) => Js::Bool(!e.eval(s)?.truthy()),
            Expr::Neg(e) => Js::Num(-to_number(&e.eval(s)?)?),
            Expr::Plus(e) => Js::Num(to_number(&e.eval(s)?)?),
            Expr::TypeOf(e) => Js::Str(Cow::Borrowed(match e.eval(s)? {
                Js::Undefined => "undefined",
                Js::Bool(_) => "boolean",
                Js::Num(_) => "number",
                Js::Str(_) => "string",
                Js::Null | Js::Json(_) => "object",
            })),
            Expr::Bin(Op::And, a, b) => {
                let a = a.eval(s)?;
                if a.truthy() { b.eval(s)? } else { a }
            }
            Expr::Bin(Op::Or, a, b) => {
                let a = a.eval(s)?;
                if a.truthy() { a } else { b.eval(s)? }
            }
            Expr::Bin(Op::Nullish, a, b) => {
                let a = a.eval(s)?;
                if a.nullish() { b.eval(s)? } else { a }
            }
            Expr::Bin(op, a, b) => binary(*op, a.eval(s)?, b.eval(s)?)?,
            Expr::If(c, t, e) => {
                if c.eval(s)?.truthy() {
                    t.eval(s)?
                } else {
                    e.eval(s)?
                }
            }
            Expr::Object(fields) => {
                let mut map = Map::with_capacity(fields.len());
                for (k, e) in fields {
                    // JSON.stringify drops a key whose value is undefined.
                    if let Some(v) = e.eval(s)?.into_json() {
                        map.insert(k.clone(), v);
                    }
                }
                let v = Value::Object(map);
                Js::Json(Cow::Owned(as_js_prints(&v).unwrap_or(v)))
            }
            Expr::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for e in items {
                    out.push(e.eval(s)?.into_json().unwrap_or(Value::Null));
                }
                Js::Json(Cow::Owned(Value::Array(out)))
            }
        })
    }
}

fn walk<'a>(mut at: Js<'a>, steps: &'a [Step], s: &Scope<'a>) -> R<Js<'a>> {
    for step in steps {
        if at.nullish() {
            if step.optional {
                return Ok(Js::Undefined);
            }
            // `undefined.x` throws; let Deno say so in its own words.
            return Err(Fallback);
        }
        at = match &step.key {
            Key::Name(name) => member(at, name)?,
            Key::Index(i) => index(at, *i)?,
            Key::Computed(e) => match e.eval(s)? {
                Js::Num(n) if n >= 0.0 && n.fract() == 0.0 && n < u32::MAX as f64 => index(at, n as usize)?,
                Js::Str(k) => match array_index(&k) {
                    Some(i) => index(at, i as usize)?,
                    None => member(at, &k)?,
                },
                _ => return Err(Fallback),
            },
        };
    }
    Ok(at)
}

fn member<'a>(at: Js<'a>, name: &str) -> R<Js<'a>> {
    // An own key answers; a missing one is `undefined`, unless every
    // object answers it through its prototype (`toString`, `map`), where
    // JavaScript has a function and this module has no answer.
    let missing = |name: &str| if is_prototype_name(name) { Err(Fallback) } else { Ok(Js::Undefined) };
    Ok(match at {
        Js::Json(Cow::Borrowed(Value::Object(m))) => match m.get(name) {
            Some(v) => Js::of(v),
            None => return missing(name),
        },
        Js::Json(Cow::Owned(Value::Object(mut m))) => match m.remove(name) {
            Some(v) => Js::of_owned(v),
            None => return missing(name),
        },
        Js::Json(Cow::Borrowed(Value::Array(a))) if name == "length" => Js::Num(a.len() as f64),
        Js::Json(Cow::Owned(Value::Array(a))) if name == "length" => Js::Num(a.len() as f64),
        Js::Str(t) if name == "length" => Js::Num(t.encode_utf16().count() as f64),
        Js::Undefined | Js::Null => return Err(Fallback),
        other => match array_index(name) {
            Some(i) => return index(other, i as usize),
            None => return missing(name),
        },
    })
}

/// Names every object, array, string or number answers through its
/// prototype (`a.map`, `s.toUpperCase`, `o.constructor`). Reading one
/// natively would answer `undefined` where JavaScript answers a function.
fn is_prototype_name(name: &str) -> bool {
    const NAMES: &[&str] = &[
        "constructor", "__proto__", "toString", "toLocaleString", "valueOf", "hasOwnProperty", "isPrototypeOf",
        "propertyIsEnumerable", "toJSON", "at", "concat", "copyWithin", "entries", "every", "fill", "filter", "find",
        "findIndex", "findLast", "findLastIndex", "flat", "flatMap", "forEach", "includes", "indexOf", "join", "keys",
        "lastIndexOf", "map", "pop", "push", "reduce", "reduceRight", "reverse", "shift", "slice", "some", "sort",
        "splice", "unshift", "values", "with", "toReversed", "toSorted", "toSpliced", "charAt", "charCodeAt",
        "codePointAt", "endsWith", "localeCompare", "match", "matchAll", "normalize", "padEnd", "padStart", "repeat",
        "replace", "replaceAll", "search", "split", "startsWith", "substring", "substr", "toLowerCase", "toUpperCase",
        "toLocaleLowerCase", "toLocaleUpperCase", "trim", "trimStart", "trimEnd", "toFixed", "toExponential",
        "toPrecision", "isWellFormed", "toWellFormed", "anchor", "big", "blink", "bold", "fixed", "fontcolor",
        "fontsize", "italics", "link", "small", "strike", "sub", "sup", "trimLeft", "trimRight", "__defineGetter__",
        "__defineSetter__", "__lookupGetter__", "__lookupSetter__",
    ];
    NAMES.contains(&name)
}

fn index<'a>(at: Js<'a>, i: usize) -> R<Js<'a>> {
    Ok(match at {
        Js::Json(Cow::Borrowed(Value::Array(a))) => a.get(i).map(Js::of).unwrap_or(Js::Undefined),
        Js::Json(Cow::Owned(Value::Array(mut a))) => {
            if i < a.len() {
                Js::of_owned(a.swap_remove(i))
            } else {
                Js::Undefined
            }
        }
        Js::Json(Cow::Borrowed(Value::Object(m))) => m.get(&i.to_string()).map(Js::of).unwrap_or(Js::Undefined),
        Js::Json(Cow::Owned(Value::Object(mut m))) => m.remove(&i.to_string()).map(Js::of_owned).unwrap_or(Js::Undefined),
        // A string's units are UTF-16; only ASCII is answered here.
        Js::Str(t) if t.is_ascii() => t.get(i..i + 1).map(|c| Js::Str(Cow::Owned(c.to_string()))).unwrap_or(Js::Undefined),
        Js::Str(_) => return Err(Fallback),
        Js::Num(_) | Js::Bool(_) | Js::Json(_) => Js::Undefined,
        Js::Undefined | Js::Null => return Err(Fallback),
    })
}

/// JavaScript's ToNumber, for the values it is exact on.
fn to_number(v: &Js) -> R<f64> {
    Ok(match v {
        Js::Undefined => f64::NAN,
        Js::Null => 0.0,
        Js::Bool(b) => *b as u8 as f64,
        Js::Num(n) => *n,
        Js::Str(t) => string_to_number(t)?,
        Js::Json(_) => return Err(Fallback),
    })
}

fn string_to_number(t: &str) -> R<f64> {
    // JavaScript's whitespace is not Unicode's; only ASCII is answered here.
    if !t.is_ascii() {
        return Err(Fallback);
    }
    let t = t.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\u{b}');
    if t.is_empty() {
        return Ok(0.0);
    }
    match t {
        "Infinity" | "+Infinity" => return Ok(f64::INFINITY),
        "-Infinity" => return Ok(f64::NEG_INFINITY),
        _ => {}
    }
    let body = t.trim_start_matches(['+', '-']);
    if body.len() > 1 && body.starts_with('0') && body.as_bytes()[1].is_ascii_alphabetic() {
        // 0x.., 0o.., 0b..: exact in JavaScript, not worth a parser here.
        return Err(Fallback);
    }
    let decimal = body.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'));
    if !decimal || t.contains("++") {
        return Ok(f64::NAN);
    }
    Ok(t.parse::<f64>().unwrap_or(f64::NAN))
}

/// JavaScript's ToString for a primitive, where it is exact here.
fn to_string(v: &Js) -> R<String> {
    Ok(match v {
        Js::Undefined => "undefined".into(),
        Js::Null => "null".into(),
        Js::Bool(b) => b.to_string(),
        Js::Num(n) => number_to_string(*n)?,
        Js::Str(t) => t.to_string(),
        // "[object Object]" and array joins: rare, and Deno knows them.
        Js::Json(_) => return Err(Fallback),
    })
}

fn number_to_string(n: f64) -> R<String> {
    if n.is_nan() {
        return Ok("NaN".into());
    }
    if n.is_infinite() {
        return Ok(if n > 0.0 { "Infinity" } else { "-Infinity" }.into());
    }
    if n == 0.0 {
        return Ok("0".into());
    }
    // Past 2^53 JavaScript prints the shortest digits padded with zeros,
    // Rust the double's exact value.
    if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_992.0 {
        return Ok(format!("{n:.0}"));
    }
    // JavaScript switches to an exponent below 1e-6 and from 1e21.
    if n.abs() < 1e-6 || n.abs() >= 1e21 {
        return Err(Fallback);
    }
    // Rust prints the shortest form that reads back as the same double, in
    // plain decimal, which is what JavaScript prints in this range.
    Ok(format!("{n}"))
}

fn utf16_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn binary<'a>(op: Op, a: Js<'a>, b: Js<'a>) -> R<Js<'a>> {
    Ok(match op {
        Op::Add => match (&a, &b) {
            (Js::Str(_), _) | (_, Js::Str(_)) => {
                let mut out = to_string(&a)?;
                out.push_str(&to_string(&b)?);
                Js::Str(Cow::Owned(out))
            }
            _ => Js::Num(to_number(&a)? + to_number(&b)?),
        },
        Op::Sub => Js::Num(to_number(&a)? - to_number(&b)?),
        Op::Mul => Js::Num(to_number(&a)? * to_number(&b)?),
        Op::Div => Js::Num(to_number(&a)? / to_number(&b)?),
        Op::Rem => Js::Num(js_rem(to_number(&a)?, to_number(&b)?)),
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            let ord = match (&a, &b) {
                (Js::Str(x), Js::Str(y)) => Some(utf16_cmp(x, y)),
                _ => to_number(&a)?.partial_cmp(&to_number(&b)?),
            };
            Js::Bool(match (op, ord) {
                (_, None) => false,
                (Op::Lt, Some(o)) => o.is_lt(),
                (Op::Le, Some(o)) => o.is_le(),
                (Op::Gt, Some(o)) => o.is_gt(),
                (_, Some(o)) => o.is_ge(),
            })
        }
        Op::StrictEq => Js::Bool(strict_eq(&a, &b)?),
        Op::StrictNe => Js::Bool(!strict_eq(&a, &b)?),
        Op::LooseEq => Js::Bool(loose_eq(&a, &b)?),
        Op::LooseNe => Js::Bool(!loose_eq(&a, &b)?),
        Op::And | Op::Or | Op::Nullish => unreachable!("short-circuit operators are evaluated in Expr::eval"),
    })
}

/// `%` keeps the dividend's sign, as Rust's does; only the zero sign of
/// an exact result differs, and JSON prints both zeros as 0.
fn js_rem(a: f64, b: f64) -> f64 {
    a % b
}

fn strict_eq(a: &Js, b: &Js) -> R<bool> {
    Ok(match (a, b) {
        (Js::Undefined, Js::Undefined) | (Js::Null, Js::Null) => true,
        (Js::Bool(x), Js::Bool(y)) => x == y,
        (Js::Num(x), Js::Num(y)) => x == y,
        (Js::Str(x), Js::Str(y)) => x == y,
        // Two objects are equal only when they are the same object.
        (Js::Json(_), Js::Json(_)) => return Err(Fallback),
        _ => false,
    })
}

fn loose_eq(a: &Js, b: &Js) -> R<bool> {
    Ok(match (a, b) {
        (x, y) if x.nullish() || y.nullish() => x.nullish() && y.nullish(),
        (Js::Json(_), _) | (_, Js::Json(_)) => return Err(Fallback),
        (Js::Str(x), Js::Str(y)) => x == y,
        _ => to_number(a)? == to_number(b)?,
    })
}

/// Compiles one expression. `Err` names what is not native, and the caller
/// sends the expression to Deno.
pub fn compile(src: &str, names: Names) -> Result<Expr, String> {
    let mut p = Parser { s: src.as_bytes(), i: 0, names };
    p.ws();
    let e = p.expr()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("not native from {:?}", String::from_utf8_lossy(&p.s[p.i..])));
    }
    Ok(e)
}

/// A script body native code can run: exactly `return <expression>;`.
pub fn compile_script(body: &str) -> Result<Expr, String> {
    let t = body.trim();
    let t = t.strip_prefix("return").filter(|r| r.starts_with(|c: char| c.is_whitespace() || c == '{' || c == '(' || c == '['))
        .ok_or("a script native code runs is one `return …;`")?;
    let t = t.trim().strip_suffix(';').unwrap_or(t.trim());
    if t.contains(';') {
        return Err("more than one statement".into());
    }
    compile(t, Names::Script)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    names: Names,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }
    fn at(&mut self, t: &str) -> bool {
        self.ws();
        self.s[self.i..].starts_with(t.as_bytes())
    }
    fn eat(&mut self, t: &str) -> bool {
        if self.at(t) {
            self.i += t.len();
            true
        } else {
            false
        }
    }
    /// An operator that is not the prefix of a longer one (`=` vs `==`).
    fn eat_op(&mut self, t: &str, not_before: &[u8]) -> bool {
        if self.at(t) && !self.s.get(self.i + t.len()).is_some_and(|c| not_before.contains(c)) {
            self.i += t.len();
            true
        } else {
            false
        }
    }
    fn expr(&mut self) -> Result<Expr, String> {
        let c = self.nullish()?;
        if self.at("?") && !self.at("?.") && !self.at("??") {
            self.i += 1;
            let t = self.expr()?;
            if !self.eat(":") {
                return Err("expected :".into());
            }
            let e = self.expr()?;
            return Ok(Expr::If(c.into(), t.into(), e.into()));
        }
        Ok(c)
    }
    fn nullish(&mut self) -> Result<Expr, String> {
        let mut left = self.or()?;
        while self.eat("??") {
            let right = self.or()?;
            left = Expr::Bin(Op::Nullish, left.into(), right.into());
        }
        Ok(left)
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut left = self.and()?;
        while self.eat("||") {
            let right = self.and()?;
            left = Expr::Bin(Op::Or, left.into(), right.into());
        }
        Ok(left)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut left = self.equality()?;
        while self.eat("&&") {
            let right = self.equality()?;
            left = Expr::Bin(Op::And, left.into(), right.into());
        }
        Ok(left)
    }
    fn equality(&mut self) -> Result<Expr, String> {
        let mut left = self.relational()?;
        loop {
            let op = if self.eat("===") {
                Op::StrictEq
            } else if self.eat("!==") {
                Op::StrictNe
            } else if self.eat("==") {
                Op::LooseEq
            } else if self.eat("!=") {
                Op::LooseNe
            } else {
                return Ok(left);
            };
            let right = self.relational()?;
            left = Expr::Bin(op, left.into(), right.into());
        }
    }
    fn relational(&mut self) -> Result<Expr, String> {
        let mut left = self.additive()?;
        loop {
            let op = if self.eat("<=") {
                Op::Le
            } else if self.eat(">=") {
                Op::Ge
            } else if self.eat_op("<", b"<") {
                Op::Lt
            } else if self.eat_op(">", b">") {
                Op::Gt
            } else {
                return Ok(left);
            };
            let right = self.additive()?;
            left = Expr::Bin(op, left.into(), right.into());
        }
    }
    fn additive(&mut self) -> Result<Expr, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = if self.eat_op("+", b"+=") {
                Op::Add
            } else if self.eat_op("-", b"-=") {
                Op::Sub
            } else {
                return Ok(left);
            };
            let right = self.multiplicative()?;
            left = Expr::Bin(op, left.into(), right.into());
        }
    }
    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = if self.eat_op("*", b"*=") {
                Op::Mul
            } else if self.eat_op("/", b"/=") {
                Op::Div
            } else if self.eat_op("%", b"=") {
                Op::Rem
            } else {
                return Ok(left);
            };
            let right = self.unary()?;
            left = Expr::Bin(op, left.into(), right.into());
        }
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_op("!", b"=") {
            return Ok(Expr::Not(self.unary()?.into()));
        }
        if self.eat_op("-", b"-=") {
            return Ok(Expr::Neg(self.unary()?.into()));
        }
        if self.eat_op("+", b"+=") {
            return Ok(Expr::Plus(self.unary()?.into()));
        }
        self.ws();
        if self.word("typeof") {
            return Ok(Expr::TypeOf(self.unary()?.into()));
        }
        self.postfix()
    }
    fn postfix(&mut self) -> Result<Expr, String> {
        let base = self.primary()?;
        let mut steps = Vec::new();
        loop {
            let optional = self.eat("?.");
            if optional || self.eat_op(".", b".") {
                if self.at("[") {
                    if !optional {
                        return Err("unexpected [".into());
                    }
                    self.i += 1;
                    steps.push(Step { key: self.bracket()?, optional });
                    continue;
                }
                if self.at("(") {
                    return Err("a call is not native".into());
                }
                self.ws();
                let name = self.ident().ok_or("expected a name after .")?;
                steps.push(Step { key: Key::Name(name), optional });
            } else if self.eat("[") {
                steps.push(Step { key: self.bracket()?, optional: false });
            } else if self.at("(") || self.at("`") {
                return Err("a call is not native".into());
            } else {
                break;
            }
        }
        if steps.is_empty() {
            return Ok(base);
        }
        Ok(match base {
            Expr::Path(root, mut first) => {
                first.extend(steps);
                Expr::Path(root, first)
            }
            other => Expr::Member(other.into(), steps),
        })
    }
    fn bracket(&mut self) -> Result<Key, String> {
        let k = self.expr()?;
        if !self.eat("]") {
            return Err("expected ]".into());
        }
        Ok(match k {
            Expr::Str(s) => match array_index(&s) {
                Some(i) => Key::Index(i as usize),
                None => Key::Name(s),
            },
            Expr::Num(n) if n >= 0.0 && n.fract() == 0.0 && n < u32::MAX as f64 => Key::Index(n as usize),
            other => Key::Computed(other.into()),
        })
    }
    fn primary(&mut self) -> Result<Expr, String> {
        self.ws();
        match self.peek().ok_or("unexpected end")? {
            b'(' => {
                self.i += 1;
                let e = self.expr()?;
                if !self.eat(")") {
                    return Err("expected )".into());
                }
                if self.at("=>") {
                    return Err("an arrow function is not native".into());
                }
                Ok(e)
            }
            b'"' | b'\'' => Ok(Expr::Str(self.string()?)),
            b'0'..=b'9' | b'.' => self.number(),
            b'{' => {
                self.i += 1;
                let mut fields: Vec<(String, Expr)> = Vec::new();
                loop {
                    if self.eat("}") {
                        break;
                    }
                    if self.at("...") || self.at("[") {
                        return Err("spread and computed keys are not native".into());
                    }
                    self.ws();
                    let key = match self.peek() {
                        Some(b'"' | b'\'') => self.string()?,
                        Some(b'0'..=b'9') => match self.number()? {
                            Expr::Num(n) => number_to_string(n).map_err(|_| "an unusual numeric key")?,
                            _ => unreachable!(),
                        },
                        _ => self.ident().ok_or("expected a key")?,
                    };
                    if key == "__proto__" {
                        return Err("__proto__ sets a prototype, not a key".into());
                    }
                    let value = if self.eat(":") {
                        self.expr()?
                    } else if self.at(",") || self.at("}") {
                        // `{ input }`: shorthand for a scope name.
                        let root = Root::named(&key, self.names).ok_or("shorthand for an unknown name")?;
                        Expr::Path(root, Vec::new())
                    } else {
                        return Err("a method is not native".into());
                    };
                    // A repeated key keeps its first place and its last value.
                    match fields.iter_mut().find(|(k, _)| *k == key) {
                        Some(slot) => slot.1 = value,
                        None => fields.push((key, value)),
                    }
                    if self.eat(",") {
                        continue;
                    }
                    if !self.eat("}") {
                        return Err("expected , or }".into());
                    }
                    break;
                }
                Ok(Expr::Object(fields))
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    if self.eat("]") {
                        break;
                    }
                    if self.at("...") || self.at(",") {
                        return Err("spread and holes are not native".into());
                    }
                    items.push(self.expr()?);
                    if self.eat(",") {
                        continue;
                    }
                    if !self.eat("]") {
                        return Err("expected , or ]".into());
                    }
                    break;
                }
                Ok(Expr::Array(items))
            }
            _ => {
                let name = self.ident().ok_or_else(|| format!("unexpected {:?}", self.peek().map(|c| c as char)))?;
                if self.at("=>") {
                    return Err("an arrow function is not native".into());
                }
                Ok(match name.as_str() {
                    "true" => Expr::Bool(true),
                    "false" => Expr::Bool(false),
                    "null" => Expr::Null,
                    "undefined" => Expr::Undefined,
                    "NaN" => Expr::Num(f64::NAN),
                    "Infinity" => Expr::Num(f64::INFINITY),
                    _ => Expr::Path(Root::named(&name, self.names).ok_or_else(|| format!("{name} is not a native name"))?, Vec::new()),
                })
            }
        }
    }
    fn word(&mut self, w: &str) -> bool {
        let end = self.i + w.len();
        if self.s[self.i..].starts_with(w.as_bytes()) && !self.s.get(end).is_some_and(|c| is_ident(*c)) {
            self.i = end;
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Option<String> {
        let start = self.i;
        while self.peek().is_some_and(is_ident) {
            self.i += 1;
        }
        if self.i == start || self.s[start].is_ascii_digit() {
            self.i = start;
            return None;
        }
        Some(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
    }
    fn string(&mut self) -> Result<String, String> {
        let q = self.s[self.i];
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek().ok_or("unclosed string")? {
                c if c == q => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\n' | b'\r' => return Err("a line break in a string".into()),
                b'\\' => {
                    self.i += 1;
                    let c = self.peek().ok_or("unclosed string")?;
                    self.i += 1;
                    match c {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'v' => out.push('\u{b}'),
                        b'0' if !self.peek().is_some_and(|d| d.is_ascii_digit()) => out.push('\0'),
                        b'\\' | b'\'' | b'"' | b'/' => out.push(c as char),
                        b'x' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 2).ok_or("short \\x")?).map_err(|e| e.to_string())?;
                            let n = u32::from_str_radix(hex, 16).map_err(|_| "bad \\x")?;
                            out.push(char::from_u32(n).ok_or("bad \\x")?);
                            self.i += 2;
                        }
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4).ok_or("short \\u")?).map_err(|e| e.to_string())?;
                            let n = u32::from_str_radix(hex, 16).map_err(|_| "\\u{…} and surrogates are not native")?;
                            out.push(char::from_u32(n).ok_or("a lone surrogate is not native")?);
                            self.i += 4;
                        }
                        _ => return Err("an unusual escape".into()),
                    }
                }
                _ => {
                    let rest = std::str::from_utf8(&self.s[self.i..]).map_err(|e| e.to_string())?;
                    let ch = rest.chars().next().unwrap();
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }
    fn number(&mut self) -> Result<Expr, String> {
        let start = self.i;
        if self.s[self.i..].starts_with(b"0x") || self.s[self.i..].starts_with(b"0o") || self.s[self.i..].starts_with(b"0b") {
            return Err("a prefixed number is not native".into());
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
            self.i += 1;
        }
        if self.peek().is_some_and(|c| c == b'e' || c == b'E') {
            self.i += 1;
            if self.peek().is_some_and(|c| c == b'+' || c == b'-') {
                self.i += 1;
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if self.peek().is_some_and(|c| is_ident(c) || c == b'_') {
            return Err("an unusual number".into());
        }
        let t = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        if t.len() > 1 && t.starts_with('0') && t.as_bytes()[1].is_ascii_digit() {
            return Err("a legacy octal number is not native".into());
        }
        t.parse::<f64>().map(Expr::Num).map_err(|_| format!("bad number {t}"))
    }
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn eval(src: &str, input: Value) -> Option<Value> {
        let e = compile(src, Names::Expression).unwrap_or_else(|e| panic!("{src}: {e}"));
        let meta = json!({});
        let s = Scope::of_node(&input, &meta);
        e.eval(&s).expect("native").into_json()
    }

    #[test]
    fn answers_as_javascript_does() {
        let i = json!({ "webhook": { "body": { "id": 7, "f": 1.0, "s": "héllo", "tags": ["a", "b"], "big": 9007199254740993u64 } } });
        assert_eq!(eval("input.webhook.body.id", i.clone()), Some(json!(7)));
        assert_eq!(eval("input.webhook.body.id * 2", i.clone()), Some(json!(14)));
        assert_eq!(eval("input.webhook.body.id / 2", i.clone()), Some(json!(3.5)));
        assert_eq!(eval("input.webhook.body.id % 100", i.clone()), Some(json!(7)));
        assert_eq!(eval("input.webhook.body.f", i.clone()), Some(json!(1)));
        assert_eq!(eval("input.webhook.body.big", i.clone()), Some(json!(9007199254740992u64)));
        assert_eq!(eval("input.webhook.body.s.length", i.clone()), Some(json!(5)));
        assert_eq!(eval("input.webhook.body.tags[1]", i.clone()), Some(json!("b")));
        assert_eq!(eval("input.webhook.body.missing", i.clone()), None);
        assert_eq!(eval("input.webhook.body.missing ?? 'none'", i.clone()), Some(json!("none")));
        assert_eq!(eval("input.webhook.nope?.deep.er", i.clone()), None);
        assert_eq!(eval("'id:' + input.webhook.body.id", i.clone()), Some(json!("id:7")));
        assert_eq!(eval("'x' + 0.1 * 3", i.clone()), Some(json!("x0.30000000000000004")));
        assert_eq!(eval("{ b: 1, 2: 'two', a: input.webhook.body.missing }", i.clone()), Some(json!({ "2": "two", "b": 1 })));
        assert_eq!(eval("[input.webhook.body.missing, 1]", i.clone()), Some(json!([null, 1])));
        assert_eq!(eval("typeof input.webhook.body.id", i.clone()), Some(json!("number")));
        assert_eq!(eval("input.webhook.body.id == '7' && input.webhook.body.id !== '7'", i.clone()), Some(json!(true)));
        assert_eq!(eval("$nodes.a", i.clone()), None);
        assert_eq!(eval("$trigger", i.clone()), Some(Value::Null));
    }

    #[test]
    fn what_needs_deno_is_refused_or_falls_back() {
        for src in ["input.items.filter(x => x.ok)", "Math.round(input.x)", "input.name.toUpperCase()", "`a${input.x}`", "new Date()", "x", "input.a = 1"] {
            assert!(compile(src, Names::Expression).is_err(), "{src} should not compile");
        }
        let input = json!({ "a": {}, "s": "é" });
        let meta = json!({});
        let s = Scope::of_node(&input, &meta);
        for src in ["input.missing.deeper", "input.a === input.a", "'x' + input.a", "input.s[0]", "input.a.map"] {
            let e = compile(src, Names::Expression).unwrap();
            assert_eq!(e.eval(&s).err(), Some(Fallback), "{src}");
        }
    }

    #[test]
    fn a_script_of_one_return_compiles() {
        assert!(compile_script("return { x: input.webhook.body.id * 2 };").is_ok());
        assert!(compile_script("const a = 1; return a;").is_err());
        assert!(compile_script("return $trigger;").is_err());
    }
}
