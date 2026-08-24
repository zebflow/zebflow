//! `zeb`: the short name, and the one `distribution.md` §Binary name calls
//! primary.
//!
//! Two names, one binary. This target compiles the same source as `zebflow`
//! rather than reimplementing it, and the program reports whichever name it
//! was invoked as, so nothing has to hardcode one of the two.

#[path = "zebflow.rs"]
mod zebflow_main;

fn main() {
    zebflow_main::main()
}
