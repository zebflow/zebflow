//! Engine-level `{{ expr }}` expression resolution for pipeline node configs.
//!
//! Every string field in a node's `config` object may contain `{{ js_expr }}` placeholders.
//! Before a node executes, the pipeline engine resolves all such expressions in one Deno
//! sandbox trip — giving every node access to `$input`, `$trigger`, `$nodes`, and `$ctx`
//! without any per-node implementation required.
//!
//! # Quick reference
//!
//! | Variable         | What you get                                            |
//! |------------------|---------------------------------------------------------|
//! | `$input`         | The current payload flowing into this node              |
//! | `$input.field`   | Specific field from upstream output                     |
//! | `$trigger.auth`  | Verified JWT claims from the original request           |
//! | `$trigger.params`| URL path params (`:id`, `:slug`, etc.)                  |
//! | `$trigger.query` | Query string params (`?page=2` etc.)                    |
//! | `$nodes.id`      | Output of a completed upstream node by its graph ID     |
//! | `$nodes.id.field`| Specific field from that node's output                  |
//! | `$ctx.pipeline`  | Current pipeline identifier                             |
//! | `$ctx.request_id`| Unique execution request id                             |
//!
//! # Examples
//!
//! Whole-field expression (preserves native type):
//! ```text
//! --params-expr "{{ [$trigger.auth.sub] }}"
//! --url "{{ $input.endpoint }}"
//! ```
//!
//! Interpolated expression (result stringified):
//! ```text
//! --url "https://api.example.com/users/{{ $trigger.auth.sub }}/profile"
//! ```
//!
//! Accessing previous node output:
//! ```text
//! --url "{{ $nodes.userLookup.rows[0].api_endpoint }}"
//! ```

pub mod scanner;
pub mod resolver;

pub use resolver::resolve_config_expressions;
