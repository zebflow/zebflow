//! Math in a docs page, turned into MathML while the site is built.
//!
//! A docs site that needs an equation has three ways to get one. It can ship a
//! JavaScript library and typeset in the reader's browser, which costs every
//! reader a download and a reflow for something that never changes. It can
//! typeset to HTML and CSS at build time, which is what KaTeX does, and then
//! the output folder has to carry a stylesheet and a megabyte of web fonts.
//! Or it can write MathML, which every current browser renders itself.
//!
//! This writes MathML. The folder stays a folder of pages — no library, no
//! stylesheet, no font directory — and an equation is text the reader can
//! select and a screen reader can read.
//!
//! What is understood is the subset a documentation page reaches for:
//! fractions, roots, sub- and superscripts, sums and integrals with limits,
//! matrices, delimiters, accents, the Greek alphabet and the usual operators.
//! Anything else is **not** guessed at: the expression is left as the author's
//! own source text, so a page with one unusual macro still builds and still
//! says what it says.

use std::fmt::Write as _;

/// The opening of a MathML element. Browsers accept MathML in HTML without the
/// namespace, but a page that is also read as XML (a feed, an editor) needs it.
const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

/// Every `$…$` and `$$…$$` in the Markdown, replaced by the MathML it means.
///
/// The source is left alone inside a fenced block and inside a code span: a
/// page that documents the syntax must be able to show it. A lone `$` is
/// money, not math — an opening `$` has to be followed by something other than
/// a space, and a closing one preceded by the same, which is the rule Pandoc
/// settled on for the same reason.
pub fn render(markdown: &str) -> String {
    let chars: Vec<char> = markdown.chars().collect();
    let mut out = String::with_capacity(markdown.len());
    let mut i = 0;
    let mut at_line_start = true;
    let mut fence: Option<usize> = None;

    while i < chars.len() {
        let ch = chars[i];

        if at_line_start && let Some(width) = fence_at(&chars, i) {
            match fence {
                Some(open) if width >= open => fence = None,
                None => fence = Some(width),
                _ => {}
            }
            let line_end = line_end_from(&chars, i);
            out.extend(&chars[i..line_end]);
            i = line_end;
            at_line_start = true;
            continue;
        }
        if fence.is_some() {
            out.push(ch);
            at_line_start = ch == '\n';
            i += 1;
            continue;
        }

        match ch {
            // `\$` is a dollar sign the author wrote on purpose.
            '\\' if i + 1 < chars.len() && chars[i + 1] == '$' => {
                out.push('$');
                i += 2;
                at_line_start = false;
            }
            // A code span is the author showing the source.
            '`' => {
                let width = run_of(&chars, i, '`');
                out.extend(&chars[i..i + width]);
                i += width;
                while i < chars.len() {
                    if chars[i] == '`' && run_of(&chars, i, '`') == width {
                        out.extend(&chars[i..i + width]);
                        i += width;
                        break;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                at_line_start = false;
            }
            '$' => {
                let display = i + 1 < chars.len() && chars[i + 1] == '$';
                let open = if display { 2 } else { 1 };
                match closing_dollar(&chars, i + open, display) {
                    Some(end) => {
                        let source: String = chars[i + open..end].iter().collect();
                        out.push_str(&typeset(&source, display));
                        i = end + open;
                        at_line_start = false;
                    }
                    None => {
                        out.push(ch);
                        i += 1;
                        at_line_start = false;
                    }
                }
            }
            _ => {
                out.push(ch);
                at_line_start = ch == '\n';
                i += 1;
            }
        }
    }
    out
}

/// One expression as MathML, or the source text when it holds something this
/// file does not understand. A page never loses what its author wrote.
fn typeset(source: &str, display: bool) -> String {
    match to_mathml(source, display) {
        Some(mathml) => mathml,
        None => {
            let fence = if display { "$$" } else { "$" };
            format!("{fence}{source}{fence}")
        }
    }
}

/// An expression as MathML, or `None` when a command here is not known.
pub fn to_mathml(source: &str, display: bool) -> Option<String> {
    let tokens = tokenize(source);
    let mut cursor = 0;
    let nodes = parse_row(&tokens, &mut cursor, &[])?;
    if cursor != tokens.len() {
        return None;
    }
    let mut body = String::new();
    emit_all(&nodes, &mut body);
    let kind = if display { "block" } else { "inline" };
    Some(format!("<math xmlns=\"{MATHML_NS}\" display=\"{kind}\">{body}</math>"))
}

// ---------------------------------------------------------------------------
// Reading the Markdown around the math
// ---------------------------------------------------------------------------

fn run_of(chars: &[char], at: usize, ch: char) -> usize {
    chars[at..].iter().take_while(|c| **c == ch).count()
}

fn fence_at(chars: &[char], at: usize) -> Option<usize> {
    let mut i = at;
    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
        i += 1;
    }
    let width = run_of(chars, i, '`');
    (width >= 3).then_some(width)
}

fn line_end_from(chars: &[char], at: usize) -> usize {
    let mut i = at;
    while i < chars.len() && chars[i] != '\n' {
        i += 1;
    }
    (i + 1).min(chars.len())
}

/// Where an expression that opened at `from` closes, if it closes. Display
/// math may hold blank lines; inline math may not, because an unmatched `$` in
/// a paragraph should not swallow the rest of the page.
fn closing_dollar(chars: &[char], from: usize, display: bool) -> Option<usize> {
    if chars.get(from).is_none_or(|c| c.is_whitespace()) && !display {
        return None;
    }
    let mut i = from;
    let mut blank_run = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '\n' if !display => {
                blank_run += 1;
                if blank_run > 1 {
                    return None;
                }
                i += 1;
            }
            '$' => {
                let width = run_of(chars, i, '$');
                let want = if display { 2 } else { 1 };
                if width < want {
                    i += width;
                    continue;
                }
                // The closing marker of inline math is never preceded by a
                // space: `$5 and $7` is two prices, not an expression.
                if !display && chars.get(i.wrapping_sub(1)).is_some_and(|c| c.is_whitespace()) {
                    i += width;
                    continue;
                }
                return (i > from).then_some(i);
            }
            _ => {
                if chars[i] != '\n' {
                    blank_run = 0;
                }
                i += 1;
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// LaTeX
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// `\alpha`, `\frac`, and the one-character ones like `\,` and `\{`.
    Cmd(String),
    Sym(char),
    Open,
    Close,
    Sub,
    Sup,
    /// A column break inside a matrix.
    Amp,
    /// A row break: `\\`.
    Row,
    Space,
}

fn tokenize(source: &str) -> Vec<Tok> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '\\' => {
                i += 1;
                let Some(&next) = chars.get(i) else {
                    out.push(Tok::Sym('\\'));
                    break;
                };
                if next == '\\' {
                    out.push(Tok::Row);
                    i += 1;
                } else if next.is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    out.push(Tok::Cmd(chars[start..i].iter().collect()));
                } else {
                    out.push(Tok::Cmd(next.to_string()));
                    i += 1;
                }
            }
            '{' => {
                out.push(Tok::Open);
                i += 1;
            }
            '}' => {
                out.push(Tok::Close);
                i += 1;
            }
            '_' => {
                out.push(Tok::Sub);
                i += 1;
            }
            '^' => {
                out.push(Tok::Sup);
                i += 1;
            }
            '&' => {
                out.push(Tok::Amp);
                i += 1;
            }
            c if c.is_whitespace() => {
                out.push(Tok::Space);
                i += 1;
            }
            c => {
                out.push(Tok::Sym(c));
                i += 1;
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
enum Node {
    Ident(String),
    Number(String),
    Op(String),
    /// An operator that grows to fit what it encloses.
    Stretchy(String),
    Text(String),
    Row(Vec<Node>),
    Frac(Box<Node>, Box<Node>),
    Sqrt(Box<Node>, Option<Box<Node>>),
    Script {
        base: Box<Node>,
        sub: Option<Box<Node>>,
        sup: Option<Box<Node>>,
        /// `\sum_{n=0}^{∞}` puts its limits above and below, not beside.
        over_under: bool,
    },
    Accent(&'static str, Box<Node>),
    Styled(&'static str, Box<Node>),
    Fenced(String, String, Box<Node>),
    Matrix(Vec<Vec<Vec<Node>>>, Option<(String, String)>),
}

/// A list of nodes up to one of `stop`, or to the end. Sub- and superscripts
/// are attached to the atom in front of them as they are met.
fn parse_row(tokens: &[Tok], i: &mut usize, stop: &[Tok]) -> Option<Vec<Node>> {
    let mut out: Vec<Node> = Vec::new();
    while *i < tokens.len() {
        if stop.contains(&tokens[*i]) {
            break;
        }
        match &tokens[*i] {
            Tok::Space => {
                *i += 1;
            }
            Tok::Close => return None,
            Tok::Sub | Tok::Sup => {
                let base = out.pop().unwrap_or(Node::Row(Vec::new()));
                out.push(parse_scripts(base, tokens, i)?);
            }
            _ => {
                let atom = parse_atom(tokens, i)?;
                let atom = if matches!(tokens.get(*i), Some(Tok::Sub) | Some(Tok::Sup)) {
                    parse_scripts(atom, tokens, i)?
                } else {
                    atom
                };
                out.push(atom);
            }
        }
    }
    Some(out)
}

fn parse_scripts(base: Node, tokens: &[Tok], i: &mut usize) -> Option<Node> {
    let over_under = wants_limits(&base);
    let mut sub = None;
    let mut sup = None;
    while let Some(tok) = tokens.get(*i) {
        match tok {
            Tok::Sub if sub.is_none() => {
                *i += 1;
                sub = Some(Box::new(parse_atom(tokens, i)?));
            }
            Tok::Sup if sup.is_none() => {
                *i += 1;
                sup = Some(Box::new(parse_atom(tokens, i)?));
            }
            _ => break,
        }
    }
    if sub.is_none() && sup.is_none() {
        return None;
    }
    Some(Node::Script { base: Box::new(base), sub, sup, over_under })
}

/// Whether this operator carries its limits above and below rather than
/// beside: a sum does, an integral does not.
fn wants_limits(node: &Node) -> bool {
    matches!(node, Node::Op(op) if matches!(op.as_str(), "∑" | "∏" | "∐" | "⋃" | "⋂" | "lim" | "max" | "min" | "sup" | "inf"))
}

fn parse_atom(tokens: &[Tok], i: &mut usize) -> Option<Node> {
    while matches!(tokens.get(*i), Some(Tok::Space)) {
        *i += 1;
    }
    match tokens.get(*i)? {
        Tok::Open => {
            *i += 1;
            let inner = parse_row(tokens, i, &[Tok::Close])?;
            if tokens.get(*i) != Some(&Tok::Close) {
                return None;
            }
            *i += 1;
            Some(Node::Row(inner))
        }
        Tok::Sym(c) => {
            let c = *c;
            if c.is_ascii_digit() {
                let start = *i;
                while matches!(tokens.get(*i), Some(Tok::Sym(d)) if d.is_ascii_digit() || *d == '.') {
                    *i += 1;
                }
                let text: String = tokens[start..*i]
                    .iter()
                    .map(|t| match t {
                        Tok::Sym(d) => *d,
                        _ => ' ',
                    })
                    .collect();
                return Some(Node::Number(text.trim_end_matches('.').to_string()));
            }
            *i += 1;
            Some(symbol(c))
        }
        Tok::Cmd(name) => {
            let name = name.clone();
            *i += 1;
            command(&name, tokens, i)
        }
        Tok::Close | Tok::Sub | Tok::Sup | Tok::Amp | Tok::Row | Tok::Space => None,
    }
}

fn symbol(c: char) -> Node {
    match c {
        '+' | '-' | '=' | '<' | '>' | '*' | '/' | ',' | ';' | ':' | '!' | '?' | '|' => {
            Node::Op(if c == '-' { "−".to_string() } else { c.to_string() })
        }
        '(' | ')' | '[' | ']' => Node::Op(c.to_string()),
        '\'' => Node::Op("′".to_string()),
        c if c.is_alphabetic() => Node::Ident(c.to_string()),
        c => Node::Op(c.to_string()),
    }
}

fn command(name: &str, tokens: &[Tok], i: &mut usize) -> Option<Node> {
    // One Greek letter, one arrow, one operator: the long tail of the subset.
    if let Some(sym) = greek(name) {
        return Some(Node::Ident(sym.to_string()));
    }
    if let Some(sym) = operator(name) {
        return Some(Node::Op(sym.to_string()));
    }
    if let Some(sym) = big_operator(name) {
        return Some(Node::Op(sym.to_string()));
    }
    if let Some(word) = function(name) {
        return Some(Node::Op(word.to_string()));
    }
    if let Some(space) = spacing(name) {
        return Some(Node::Text(space.to_string()));
    }

    match name {
        "frac" | "dfrac" | "tfrac" => {
            let top = parse_atom(tokens, i)?;
            let bottom = parse_atom(tokens, i)?;
            Some(Node::Frac(Box::new(top), Box::new(bottom)))
        }
        "sqrt" => {
            let index = if matches!(tokens.get(*i), Some(Tok::Sym('['))) {
                *i += 1;
                let inner = parse_row(tokens, i, &[Tok::Sym(']')])?;
                if tokens.get(*i) != Some(&Tok::Sym(']')) {
                    return None;
                }
                *i += 1;
                Some(Box::new(Node::Row(inner)))
            } else {
                None
            };
            Some(Node::Sqrt(Box::new(parse_atom(tokens, i)?), index))
        }
        "hat" => Some(Node::Accent("^", Box::new(parse_atom(tokens, i)?))),
        "bar" | "overline" => Some(Node::Accent("‾", Box::new(parse_atom(tokens, i)?))),
        "vec" => Some(Node::Accent("→", Box::new(parse_atom(tokens, i)?))),
        "tilde" => Some(Node::Accent("~", Box::new(parse_atom(tokens, i)?))),
        "dot" => Some(Node::Accent("˙", Box::new(parse_atom(tokens, i)?))),
        "ddot" => Some(Node::Accent("¨", Box::new(parse_atom(tokens, i)?))),
        "text" | "textrm" | "mbox" => Some(Node::Text(raw_group(tokens, i)?)),
        "mathrm" | "operatorname" => Some(Node::Styled("normal", Box::new(parse_atom(tokens, i)?))),
        "mathbf" => Some(Node::Styled("bold", Box::new(parse_atom(tokens, i)?))),
        "mathit" => Some(Node::Styled("italic", Box::new(parse_atom(tokens, i)?))),
        "mathbb" => Some(Node::Styled("double-struck", Box::new(parse_atom(tokens, i)?))),
        "mathcal" => Some(Node::Styled("script", Box::new(parse_atom(tokens, i)?))),
        "left" => parse_fenced(tokens, i),
        "begin" => parse_environment(tokens, i),
        "{" => Some(Node::Op("{".to_string())),
        "}" => Some(Node::Op("}".to_string())),
        "|" => Some(Node::Op("‖".to_string())),
        "%" | "&" | "#" | "_" | "$" => Some(Node::Text(name.to_string())),
        _ => None,
    }
}

/// `\left( … \right)`, with the delimiters set to grow.
fn parse_fenced(tokens: &[Tok], i: &mut usize) -> Option<Node> {
    let open = delimiter(tokens, i)?;
    let inner = parse_row(tokens, i, &[Tok::Cmd("right".to_string())])?;
    if tokens.get(*i) != Some(&Tok::Cmd("right".to_string())) {
        return None;
    }
    *i += 1;
    let close = delimiter(tokens, i)?;
    Some(Node::Fenced(open, close, Box::new(Node::Row(inner))))
}

fn delimiter(tokens: &[Tok], i: &mut usize) -> Option<String> {
    while matches!(tokens.get(*i), Some(Tok::Space)) {
        *i += 1;
    }
    let out = match tokens.get(*i)? {
        Tok::Sym('(') => "(",
        Tok::Sym(')') => ")",
        Tok::Sym('[') => "[",
        Tok::Sym(']') => "]",
        Tok::Sym('|') => "|",
        Tok::Sym('.') => "",
        Tok::Sym('/') => "/",
        Tok::Cmd(name) => match name.as_str() {
            "{" => "{",
            "}" => "}",
            "|" => "‖",
            "langle" => "⟨",
            "rangle" => "⟩",
            "lfloor" => "⌊",
            "rfloor" => "⌋",
            "lceil" => "⌈",
            "rceil" => "⌉",
            _ => return None,
        },
        _ => return None,
    };
    *i += 1;
    Some(out.to_string())
}

/// `\begin{pmatrix} a & b \\ c & d \end{pmatrix}`.
fn parse_environment(tokens: &[Tok], i: &mut usize) -> Option<Node> {
    let env = raw_group(tokens, i)?;
    let fences = match env.as_str() {
        "matrix" => None,
        "pmatrix" => Some(("(".to_string(), ")".to_string())),
        "bmatrix" => Some(("[".to_string(), "]".to_string())),
        "Bmatrix" => Some(("{".to_string(), "}".to_string())),
        "vmatrix" => Some(("|".to_string(), "|".to_string())),
        "Vmatrix" => Some(("‖".to_string(), "‖".to_string())),
        _ => return None,
    };

    let mut rows: Vec<Vec<Vec<Node>>> = Vec::new();
    let mut row: Vec<Vec<Node>> = Vec::new();
    let stop = [Tok::Amp, Tok::Row, Tok::Cmd("end".to_string())];
    loop {
        let cell = parse_row(tokens, i, &stop)?;
        row.push(cell);
        match tokens.get(*i) {
            Some(Tok::Amp) => *i += 1,
            Some(Tok::Row) => {
                *i += 1;
                rows.push(std::mem::take(&mut row));
            }
            Some(Tok::Cmd(name)) if name == "end" => {
                *i += 1;
                let closing = raw_group(tokens, i)?;
                if closing != env {
                    return None;
                }
                rows.push(row);
                return Some(Node::Matrix(rows, fences));
            }
            _ => return None,
        }
    }
}

/// A braced argument read as plain text: `\text{…}` and `\begin{…}` do not
/// hold math, and tokenising them would lose the author's spaces.
fn raw_group(tokens: &[Tok], i: &mut usize) -> Option<String> {
    while matches!(tokens.get(*i), Some(Tok::Space)) {
        *i += 1;
    }
    if tokens.get(*i) != Some(&Tok::Open) {
        return None;
    }
    *i += 1;
    let mut out = String::new();
    while let Some(tok) = tokens.get(*i) {
        match tok {
            Tok::Close => {
                *i += 1;
                return Some(out);
            }
            Tok::Sym(c) => out.push(*c),
            Tok::Space => out.push(' '),
            Tok::Sub => out.push('_'),
            Tok::Sup => out.push('^'),
            Tok::Amp => out.push('&'),
            Tok::Cmd(name) => {
                out.push('\\');
                out.push_str(name);
            }
            Tok::Open | Tok::Row => return None,
        }
        *i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// MathML
// ---------------------------------------------------------------------------

fn emit_all(nodes: &[Node], out: &mut String) {
    for node in nodes {
        emit(node, out);
    }
}

fn emit(node: &Node, out: &mut String) {
    match node {
        Node::Ident(text) => {
            let _ = write!(out, "<mi>{}</mi>", escape(text));
        }
        Node::Number(text) => {
            let _ = write!(out, "<mn>{}</mn>", escape(text));
        }
        Node::Op(text) => {
            // A delimiter written plainly is the size it is written: only
            // `\left` and `\right` ask one to grow. Without this a bra-ket
            // sets its bars to the height of the whole line, which is how
            // `|a\rangle\langle a|` ends up looking like a fence.
            if is_delimiter(text) {
                let _ = write!(out, "<mo fence=\"true\" stretchy=\"false\">{}</mo>", escape(text));
            } else {
                let _ = write!(out, "<mo>{}</mo>", escape(text));
            }
        }
        Node::Stretchy(text) => {
            let _ = write!(out, "<mo stretchy=\"true\">{}</mo>", escape(text));
        }
        Node::Text(text) => {
            let _ = write!(out, "<mtext>{}</mtext>", escape(text));
        }
        // A group of one needs no group: `\frac{1}{2}` is two numbers, not
        // two rows holding a number each. Every element that counts its
        // children still gets exactly as many as it requires, because an
        // empty group keeps its row.
        Node::Row(nodes) if nodes.len() == 1 => emit(&nodes[0], out),
        Node::Row(nodes) => {
            out.push_str("<mrow>");
            emit_all(nodes, out);
            out.push_str("</mrow>");
        }
        Node::Frac(top, bottom) => {
            out.push_str("<mfrac>");
            emit(top, out);
            emit(bottom, out);
            out.push_str("</mfrac>");
        }
        Node::Sqrt(base, index) => match index {
            Some(index) => {
                out.push_str("<mroot>");
                emit(base, out);
                emit(index, out);
                out.push_str("</mroot>");
            }
            None => {
                out.push_str("<msqrt>");
                emit(base, out);
                out.push_str("</msqrt>");
            }
        },
        Node::Script { base, sub, sup, over_under } => {
            let tag = match (sub.is_some(), sup.is_some(), over_under) {
                (true, true, true) => "munderover",
                (true, false, true) => "munder",
                (false, true, true) => "mover",
                (true, true, false) => "msubsup",
                (true, false, false) => "msub",
                (false, true, false) => "msup",
                (false, false, _) => "mrow",
            };
            let _ = write!(out, "<{tag}>");
            emit(base, out);
            if let Some(sub) = sub {
                emit(sub, out);
            }
            if let Some(sup) = sup {
                emit(sup, out);
            }
            let _ = write!(out, "</{tag}>");
        }
        Node::Accent(mark, base) => {
            out.push_str("<mover accent=\"true\">");
            emit(base, out);
            let _ = write!(out, "<mo>{}</mo></mover>", escape(mark));
        }
        Node::Styled(variant, base) => {
            let _ = write!(out, "<mstyle mathvariant=\"{variant}\">");
            emit(base, out);
            out.push_str("</mstyle>");
        }
        Node::Fenced(open, close, inner) => {
            out.push_str("<mrow>");
            if !open.is_empty() {
                emit(&Node::Stretchy(open.clone()), out);
            }
            emit(inner, out);
            if !close.is_empty() {
                emit(&Node::Stretchy(close.clone()), out);
            }
            out.push_str("</mrow>");
        }
        Node::Matrix(rows, fences) => {
            out.push_str("<mrow>");
            if let Some((open, _)) = fences {
                emit(&Node::Stretchy(open.clone()), out);
            }
            // A MathML table has no spacing of its own, so without this the
            // entries of a 2x2 matrix touch and read as a four-digit number.
            out.push_str("<mtable rowspacing=\"0.4em\" columnspacing=\"0.7em\">");
            for row in rows {
                out.push_str("<mtr>");
                for cell in row {
                    out.push_str("<mtd>");
                    emit_all(cell, out);
                    out.push_str("</mtd>");
                }
                out.push_str("</mtr>");
            }
            out.push_str("</mtable>");
            if let Some((_, close)) = fences {
                emit(&Node::Stretchy(close.clone()), out);
            }
            out.push_str("</mrow>");
        }
    }
}

/// The characters that a renderer would otherwise stretch to the height of
/// whatever stands beside them.
fn is_delimiter(text: &str) -> bool {
    matches!(text, "(" | ")" | "[" | "]" | "{" | "}" | "|" | "‖" | "⟨" | "⟩" | "⌊" | "⌋" | "⌈" | "⌉")
}

/// Content of a MathML element. The characters that could end it early are the
/// only ones that must change.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The names
// ---------------------------------------------------------------------------

fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "imath" => "ı",
        "jmath" => "ȷ",
        "infty" => "∞",
        _ => return None,
    })
}

fn operator(name: &str) -> Option<&'static str> {
    Some(match name {
        "times" => "×",
        "div" => "÷",
        "cdot" => "⋅",
        "cdots" => "⋯",
        "ldots" | "dots" => "…",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "pm" => "±",
        "mp" => "∓",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "∙",
        "oplus" => "⊕",
        "ominus" => "⊖",
        "otimes" => "⊗",
        "approx" => "≈",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "equiv" => "≡",
        "propto" => "∝",
        "neq" | "ne" => "≠",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "ll" => "≪",
        "gg" => "≫",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "supset" => "⊃",
        "supseteq" => "⊇",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "cup" => "∪",
        "cap" => "∩",
        "setminus" => "∖",
        "emptyset" | "varnothing" => "∅",
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        "to" | "rightarrow" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" | "implies" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" | "iff" => "⇔",
        "mapsto" => "↦",
        "longrightarrow" => "⟶",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "partial" => "∂",
        "nabla" => "∇",
        "dagger" => "†",
        "perp" => "⊥",
        "parallel" => "∥",
        "angle" => "∠",
        "degree" => "°",
        "prime" => "′",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "therefore" => "∴",
        "because" => "∵",
        "aleph" => "ℵ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        _ => return None,
    })
}

fn big_operator(name: &str) -> Option<&'static str> {
    Some(match name {
        "sum" => "∑",
        "prod" => "∏",
        "coprod" => "∐",
        "int" => "∫",
        "iint" => "∬",
        "iiint" => "∭",
        "oint" => "∮",
        "bigcup" => "⋃",
        "bigcap" => "⋂",
        "bigoplus" => "⨁",
        "bigotimes" => "⨂",
        _ => return None,
    })
}

/// A name that is a word rather than a symbol: it is set upright, and the ones
/// that take limits carry them above and below.
fn function(name: &str) -> Option<&'static str> {
    Some(match name {
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "cot" => "cot",
        "sec" => "sec",
        "csc" => "csc",
        "arcsin" => "arcsin",
        "arccos" => "arccos",
        "arctan" => "arctan",
        "sinh" => "sinh",
        "cosh" => "cosh",
        "tanh" => "tanh",
        "exp" => "exp",
        "log" => "log",
        "ln" => "ln",
        "lg" => "lg",
        "det" => "det",
        "dim" => "dim",
        "ker" => "ker",
        "deg" => "deg",
        "gcd" => "gcd",
        "arg" => "arg",
        "mod" => "mod",
        "tr" => "tr",
        "Tr" => "Tr",
        "lim" => "lim",
        "max" => "max",
        "min" => "min",
        "sup" => "sup",
        "inf" => "inf",
        _ => return None,
    })
}

/// Explicit space. MathML has `<mspace>`, but a space in `<mtext>` is what a
/// browser lays out most predictably across the three engines.
fn spacing(name: &str) -> Option<&'static str> {
    Some(match name {
        "," => "\u{2009}",
        ":" | ";" => "\u{2005}",
        "!" => "",
        " " => " ",
        "quad" => "\u{2003}",
        "qquad" => "\u{2003}\u{2003}",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_inline_expression_becomes_mathml() {
        let out = render("The mean is $\\langle A\\rangle$ over many runs.");
        assert!(out.contains("<math"), "{out}");
        assert!(out.contains("display=\"inline\""), "{out}");
        assert!(out.contains(">⟨</mo>"), "{out}");
        assert!(out.contains("<mi>A</mi>"), "{out}");
        assert!(out.contains("over many runs."), "the prose is untouched: {out}");
    }

    #[test]
    fn a_display_expression_is_marked_as_one() {
        let out = render("$$P(a) = |\\langle a|\\psi\\rangle|^2$$");
        assert!(out.contains("display=\"block\""), "{out}");
        assert!(out.contains("<msup>"), "the exponent is a superscript: {out}");
        assert!(out.contains("<mi>ψ</mi>"), "{out}");
    }

    #[test]
    fn a_fraction_a_root_and_a_sum_each_get_their_element() {
        let frac = to_mathml("\\frac{1}{2}", false).expect("fraction");
        assert!(frac.contains("<mfrac><mn>1</mn><mn>2</mn></mfrac>"), "{frac}");
        let root = to_mathml("\\sqrt{2}", false).expect("root");
        assert!(root.contains("<msqrt>"), "{root}");
        let nth = to_mathml("\\sqrt[3]{x}", false).expect("nth root");
        assert!(nth.contains("<mroot>"), "{nth}");
        // A sum carries its limits above and below; an integral beside.
        let sum = to_mathml("\\sum_{n=0}^{N} n", false).expect("sum");
        assert!(sum.contains("<munderover>"), "{sum}");
        let int = to_mathml("\\int_0^1 x", false).expect("integral");
        assert!(int.contains("<msubsup>"), "{int}");
    }

    #[test]
    fn a_matrix_becomes_a_table_inside_its_brackets() {
        let out = to_mathml("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}", true).expect("matrix");
        assert_eq!(out.matches("<mtr>").count(), 2, "{out}");
        assert_eq!(out.matches("<mtd>").count(), 4, "{out}");
        assert!(out.contains("<mo stretchy=\"true\">(</mo>"), "{out}");
        assert!(out.contains("columnspacing="), "entries do not touch: {out}");
    }

    /// A page about prices is not a page about mathematics.
    #[test]
    fn money_is_left_alone() {
        let out = render("It costs $5 and then $7 more.");
        assert_eq!(out, "It costs $5 and then $7 more.");
    }

    #[test]
    fn a_dollar_in_a_code_span_or_a_fence_is_not_math() {
        let span = render("Write `$x^2$` to get an equation.");
        assert_eq!(span, "Write `$x^2$` to get an equation.");
        let fence = render("```md\n$x^2$\n```\n");
        assert_eq!(fence, "```md\n$x^2$\n```\n");
        let escaped = render("A literal \\$ sign.");
        assert_eq!(escaped, "A literal $ sign.");
    }

    /// Nothing here guesses. A macro this file does not know leaves the
    /// author's own source on the page.
    #[test]
    fn an_unknown_command_keeps_its_source() {
        assert_eq!(to_mathml("\\xcancel{x}", false), None);
        let out = render("Here: $\\xcancel{x}$ and on.");
        assert_eq!(out, "Here: $\\xcancel{x}$ and on.");
    }

    /// The characters that would end the element early are the only ones that
    /// may not survive as themselves.
    #[test]
    fn a_comparison_cannot_close_the_element() {
        let out = to_mathml("a < b", false).expect("comparison");
        assert!(out.contains("<mo>&lt;</mo>"), "{out}");
        assert!(!out.contains("<mo><</mo>"));
    }

    /// A bra-ket is written with bars, and a bar that stretches to the height
    /// of the line reads as a fence rather than as notation.
    #[test]
    fn a_plain_delimiter_does_not_stretch() {
        let out = to_mathml("|a\\rangle\\langle a|", false).expect("bra-ket");
        assert!(out.contains("<mo fence=\"true\" stretchy=\"false\">|</mo>"), "{out}");
        assert!(out.contains("<mo fence=\"true\" stretchy=\"false\">⟩</mo>"), "{out}");
        // `\left(` still grows: that is what it is for.
        let grown = to_mathml("\\left( x \\right)", false).expect("grown");
        assert!(grown.contains("<mo stretchy=\"true\">(</mo>"), "{grown}");
    }

    #[test]
    fn an_unclosed_expression_is_left_as_text() {
        let out = render("An opening $x + y with no close.\n\nNext paragraph.");
        assert!(out.starts_with("An opening $x + y"), "{out}");
        assert!(!out.contains("<math"), "{out}");
    }
}
