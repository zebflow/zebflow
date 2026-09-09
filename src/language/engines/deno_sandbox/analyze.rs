//! AST-based script analysis and instrumentation.
//!
//! # Why this replaced a character scanner
//!
//! The previous implementation rewrote source as **text**, tracking string,
//! template and comment state by hand. It had no regex-literal state, so
//! `/for (a;b;c)/` was rewritten inside the literal — silently changing what
//! the program meant, and in one shape turning a valid program into a
//! `SyntaxError`. It matched loop keywords only when adjacent to their paren,
//! so `while/**/(true)` was never instrumented at all. And it decided policy
//! by substring, so a script whose *data* contained `import ` or `eval()` was
//! refused while an attacker writing `globalThis['ev'+'al']` walked past.
//!
//! `js_masker.rs` already made this argument for the RWE bundler: OXC knows
//! every JavaScript lexical context, so we piggyback on that knowledge rather
//! than reimplementing a half-baked lexer. This module applies the same rule
//! to the sandbox.
//!
//! # What is decidable here, and what is not
//!
//! Static analysis **closes** a defect class only when the language cannot
//! express an escape. A binding name is such a case: you cannot declare a
//! `var` with a computed name, so a script that shadows the runtime's own
//! `__tj_tick` is always visible in the AST and is refused here, permanently.
//!
//! It only **raises the cost** where a dynamic form exists: `x.constructor` is
//! visible, `x['cons'+'tructor']` is not. And it says nothing at all about
//! non-termination, runaway allocation, or catastrophic backtracking — those
//! are undecidable and belong to the host-side watchdog.
//!
//! Because these checks run at pipeline **save** time, an author (human or
//! agent) sees a line number instead of a production failure.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    DoWhileStatement, ForInStatement, ForOfStatement, ForStatement, Statement, WhileStatement,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

/// One reason a script was refused, positioned in the author's own source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDiagnostic {
    pub code: &'static str,
    pub message: String,
    /// 1-based, relative to the body the author wrote.
    pub line: u32,
    /// 1-based.
    pub column: u32,
}

impl std::fmt::Display for ScriptDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}:{}: {}", self.line, self.column, self.message)
    }
}

/// Identifiers the runtime owns. The injected loop guard calls `__tj_tick()`
/// as a bare identifier, so a function-scoped binding of that name captures
/// it and the op budget silently stops applying. A global `defineProperty`
/// lock cannot prevent that — but the parser can see it every time.
const RESERVED: &[&str] = &[
    "__tj_tick",
    "__fetchConfig",
    "__script_input",
    "__script_n",
    "__script_ctx",
    "__zebflow_script_result",
    "__zfOps",
    "__zfScriptResult",
    "__zfReadLocalFile",
    "__opsLeft",
    "__deadline",
    "__r",
    "__fn",
];

/// The wrapper's own parameters. A body that redeclares one of these is a
/// JavaScript early error, but the message V8 produces at run time —
/// "Identifier 'n' has already been declared" — tells an author nothing about
/// why `n` was taken. Named here so the save-time message can.
const WRAPPER_PARAMS: &[&str] = &["input", "n", "ctx"];

/// The header wrapped around an author's body to make it a parseable program.
/// Kept on ONE line so a reported line number maps to the author's source by
/// subtracting exactly one.
const WRAPPER_OPEN: &str = "async function __zf_main(input, n, ctx) {";

/// What the resolved config permits. Mirrors the danger-zone switches that
/// were previously enforced by substring scanning.
#[derive(Debug, Clone, Copy)]
pub struct ScriptPolicy {
    pub allow_dynamic_code: bool,
    pub allow_import: bool,
    pub allow_timers: bool,
    pub inject_guards: bool,
}

/// A byte-offset edit to apply to the wrapped source.
struct Splice {
    at: usize,
    text: &'static str,
}

/// Parses `body`, refuses what is statically refusable, and returns the
/// instrumented function expression ready for the runtime.
///
/// `inject_guards == false` skips instrumentation but still validates.
pub fn compile_body(body: &str, policy: ScriptPolicy) -> Result<String, ScriptDiagnostic> {
    let wrapped = format!("{WRAPPER_OPEN}\n{body}\n}}");
    let allocator = Allocator::default();
    let source_type = SourceType::default();
    let parsed = Parser::new(&allocator, &wrapped, source_type).parse();

    if let Some(err) = parsed.errors.first() {
        let offset = err
            .labels
            .as_ref()
            .and_then(|l| l.first().map(|s| s.offset()))
            .unwrap_or(0);
        let (line, column) = line_col(&wrapped, offset);
        return Err(ScriptDiagnostic {
            code: "SCRIPT_SYNTAX",
            message: format!("script does not parse: {}", err.message),
            line,
            column,
        });
    }

    // Parsing alone does not catch early errors that need scope analysis —
    // redeclaring a binding, most importantly. Without this, a script that
    // declares `const n` parses cleanly here and dies at run time with a
    // message that never mentions the wrapper it collided with.
    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program);
    if let Some(err) = semantic.errors.first() {
        let offset = err
            .labels
            .as_ref()
            .and_then(|l| l.first().map(|s| s.offset()))
            .unwrap_or(0);
        let (line, column) = line_col(&wrapped, offset);
        // Match only the quoted identifier. A bare `contains` is wrong here:
        // "n" occurs inside "Identifier", so every message matched `n` and
        // a collision on `ctx` was reported as a collision on `n`.
        let collided = WRAPPER_PARAMS
            .iter()
            .find(|p| err.message.contains(&format!("`{p}`")));
        let message = match collided {
            Some(name) => format!(
                "`{name}` is already defined: a script body receives `input`, `n` and `ctx` \
                 as its arguments, so it cannot declare them again. Rename your variable."
            ),
            None => format!("script is not valid: {}", err.message),
        };
        return Err(ScriptDiagnostic {
            code: "SCRIPT_SEMANTIC",
            message,
            line,
            column,
        });
    }

    let mut collector = Collector {
        splices: Vec::new(),
        violation: None,
        policy,
    };
    oxc_ast_visit::walk::walk_program(&mut collector, &parsed.program);

    if let Some((code, message, offset)) = collector.violation {
        let (line, column) = line_col(&wrapped, offset);
        return Err(ScriptDiagnostic { code, message, line, column });
    }

    let mut out = wrapped;
    if policy.inject_guards {
        // Apply from the end so earlier offsets stay valid.
        collector.splices.sort_by(|a, b| b.at.cmp(&a.at));
        for splice in &collector.splices {
            out.insert_str(splice.at, splice.text);
        }
    }
    Ok(out)
}

/// Line and column (both 1-based) of a byte offset, reported against the
/// author's body rather than the wrapper — hence the one-line adjustment.
fn line_col(text: &str, offset: usize) -> (u32, u32) {
    let upto = &text[..offset.min(text.len())];
    let line = upto.matches('\n').count() as u32; // wrapper occupies line 1
    let column = upto.rsplit('\n').next().map(str::len).unwrap_or(0) as u32 + 1;
    (line.max(1), column)
}

struct Collector {
    splices: Vec<Splice>,
    violation: Option<(&'static str, String, usize)>,
    policy: ScriptPolicy,
}

impl Collector {
    fn refuse(&mut self, code: &'static str, message: String, offset: u32) {
        if self.violation.is_none() {
            self.violation = Some((code, message, offset as usize));
        }
    }

    fn note_reserved(&mut self, name: &str, offset: u32) {
        if RESERVED.contains(&name) {
            self.refuse(
                "SCRIPT_RESERVED_BINDING",
                format!(
                    "`{name}` is a runtime-reserved name and cannot be declared or \
                     assigned; binding it would disable this script's own execution limits"
                ),
                offset,
            );
        }
    }

    /// A bare-identifier call, which is the only form these checks can see.
    /// `globalThis['ev'+'al']` is deliberately out of scope: it is not
    /// statically decidable, and the host watchdog is what actually contains
    /// the consequences.
    fn check_callee_name(&mut self, name: &str, offset: u32) {
        if !self.policy.allow_dynamic_code && matches!(name, "eval" | "Function") {
            self.refuse(
                "SCRIPT_DYNAMIC_CODE",
                format!("`{name}` is disabled: dynamic code generation is not permitted"),
                offset,
            );
        }
        if !self.policy.allow_timers
            && matches!(name, "setTimeout" | "setInterval" | "queueMicrotask")
        {
            self.refuse(
                "SCRIPT_TIMERS",
                format!("`{name}` is disabled: timers are not permitted"),
                offset,
            );
        }
    }

    /// A loop with a test gets the tick folded into the test via the comma
    /// operator: `while (__tj_tick(), cond)`. The existing parens make this
    /// safe without adding any of our own.
    fn guard_test(&mut self, test_start: u32) {
        self.splices.push(Splice {
            at: test_start as usize,
            text: "__tj_tick(), ",
        });
    }

    /// A loop with no test (`for(;;)`, `for-of`, `for-in`) gets the tick at
    /// the top of its body instead.
    fn guard_body(&mut self, body: &Statement<'_>) {
        match body {
            Statement::BlockStatement(block) => self.splices.push(Splice {
                at: block.span.start as usize + 1, // just inside `{`
                text: "__tj_tick();",
            }),
            other => {
                // A braceless body must become a block, or the tick would
                // steal the loop's only statement.
                let span = other.span();
                self.splices.push(Splice {
                    at: span.start as usize,
                    text: "{__tj_tick();",
                });
                self.splices.push(Splice {
                    at: span.end as usize,
                    text: "}",
                });
            }
        }
    }
}

impl<'a> oxc_ast_visit::Visit<'a> for Collector {
    fn visit_binding_identifier(&mut self, ident: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.note_reserved(ident.name.as_str(), ident.span.start);
    }

    fn visit_assignment_target(&mut self, target: &oxc_ast::ast::AssignmentTarget<'a>) {
        use oxc_ast::ast::AssignmentTarget as AT;
        match target {
            AT::AssignmentTargetIdentifier(ident) => {
                self.note_reserved(ident.name.as_str(), ident.span.start);
            }
            // `globalThis.__tj_tick = f` and `globalThis["__tj_tick"] = f`.
            // Both are statically visible; a computed key built at run time
            // is not, and is left to the watchdog by design.
            AT::StaticMemberExpression(member) => {
                self.note_reserved(member.property.name.as_str(), member.span.start);
            }
            AT::ComputedMemberExpression(member) => {
                if let oxc_ast::ast::Expression::StringLiteral(key) = &member.expression {
                    self.note_reserved(key.value.as_str(), member.span.start);
                }
            }
            _ => {}
        }
        oxc_ast_visit::walk::walk_assignment_target(self, target);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if let oxc_ast::ast::Expression::Identifier(ident) = &call.callee {
            self.check_callee_name(ident.name.as_str(), ident.span.start);
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, expr: &oxc_ast::ast::NewExpression<'a>) {
        if let oxc_ast::ast::Expression::Identifier(ident) = &expr.callee {
            self.check_callee_name(ident.name.as_str(), ident.span.start);
        }
        oxc_ast_visit::walk::walk_new_expression(self, expr);
    }

    fn visit_import_expression(&mut self, expr: &oxc_ast::ast::ImportExpression<'a>) {
        if !self.policy.allow_import {
            self.refuse(
                "SCRIPT_IMPORT",
                "dynamic `import()` is disabled".to_string(),
                expr.span.start,
            );
        }
        oxc_ast_visit::walk::walk_import_expression(self, expr);
    }

    /// `({ __tj_tick } = { … })`. Shorthand object destructuring reaches its
    /// target through a distinct node that neither the identifier nor the
    /// member-expression arm above ever sees, so a reserved name could be
    /// reassigned without any of the checks firing.
    fn visit_assignment_target_property_identifier(
        &mut self,
        prop: &oxc_ast::ast::AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.note_reserved(prop.binding.name.as_str(), prop.binding.span.start);
        oxc_ast_visit::walk::walk_assignment_target_property_identifier(self, prop);
    }

    /// `with` re-binds every free identifier in its body against an object the
    /// script controls, which captures the injected `__tj_tick()` calls and
    /// silently voids the op budget. The body is not strict-mode, so the
    /// engine accepts it; there is no legitimate use in a pipeline script.
    fn visit_with_statement(&mut self, stmt: &oxc_ast::ast::WithStatement<'a>) {
        self.refuse(
            "SCRIPT_WITH_STATEMENT",
            "`with` is not permitted: it rebinds every name in its body, \
             including the runtime's own execution limits"
                .to_string(),
            stmt.span.start,
        );
        oxc_ast_visit::walk::walk_with_statement(self, stmt);
    }

    fn visit_while_statement(&mut self, stmt: &WhileStatement<'a>) {
        self.guard_test(stmt.test.span().start);
        oxc_ast_visit::walk::walk_while_statement(self, stmt);
    }

    fn visit_do_while_statement(&mut self, stmt: &DoWhileStatement<'a>) {
        self.guard_test(stmt.test.span().start);
        oxc_ast_visit::walk::walk_do_while_statement(self, stmt);
    }

    fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
        match &stmt.test {
            Some(test) => self.guard_test(test.span().start),
            None => self.guard_body(&stmt.body),
        }
        oxc_ast_visit::walk::walk_for_statement(self, stmt);
    }

    fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
        self.guard_body(&stmt.body);
        oxc_ast_visit::walk::walk_for_of_statement(self, stmt);
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        self.guard_body(&stmt.body);
        oxc_ast_visit::walk::walk_for_in_statement(self, stmt);
    }
}
