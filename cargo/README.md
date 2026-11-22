# zebflow

This crate installs [Zebflow](https://zebflow.com). It does not contain
Zebflow's source code: `cargo install zebflow` builds a small launcher in
seconds, and the first time you run `zebflow` it downloads the ready-made
Zebflow program for your computer from the matching
[GitHub release](https://github.com/zebflow/zebflow/releases), checks it
against the release's checksums, and runs it. `pip install zebflow` and
`npm install -g zebflow` work the same way.

```bash
cargo install zebflow
zebflow              # starts Zebflow on port 10610
```

Supported: Linux (x86-64 and arm64, including a Raspberry Pi 4 or 5 on a
64-bit system), macOS on Apple Silicon, Windows (x86-64). The program is kept
in `~/.zebflow/bin/<version>/`. `ZEBFLOW_VERSION=v0.13.4` runs another
release.

To build Zebflow itself, clone the repository:
https://github.com/zebflow/zebflow
