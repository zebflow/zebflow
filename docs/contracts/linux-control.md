# Linux Control

Status: **candidate**, drawn 2026-11-20, for 0.12.0.

How a pipeline uses a Linux machine: it runs a command there over SSH, as one
Linux user, and gets back what the command printed. The machine is any Linux
host the office can reach — a server, or a long-running container in the same
cluster. What the command may touch is decided on the machine, by that user's
own permissions; Zebflow decides only who may reach it and with what.

```text
pipeline ─▶ linux.command.run --credential compute-1 --run "…"
               └▶ ssh (system program) ─▶ user@host ─▶ the user's shell runs the line
            ◀─ command: { ok, exit_code, stdout, stderr, duration_ms, truncated }
```

## The `ssh` credential

| Field | Rule |
| --- | --- |
| `host` | name or address of the machine |
| `port` | default 22 |
| `user` | the Linux user every command runs as; never `root` |
| `private_key` | an OpenSSH private key without a passphrase; secret, never answered |
| `host_key` | the machine's public host key, one `known_hosts` line body (`ssh-ed25519 AAAA…`); **required** — a machine that answers with another key is refused |
| `commands` | optional: the programs a line may start with (`bun`, `systemctl`, `cat`); empty = any |
| `workdir` | optional: the folder commands start in, relative to the user's home or absolute; no `..` |

A credential without `host_key`, with `user: root`, or with a key that has a
passphrase is refused when it is saved. A control door
(`app-control.md`) may manage `ssh` credentials only to outside addresses,
like every kind it manages.

## `linux.command.run`

| Flag | Meaning |
| --- | --- |
| `--credential` | the `ssh` credential |
| `--run` | the command line, run by the user's login shell on the machine |
| `--stdin` | optional text sent to the command's standard input (how a file is written: `--run "cat > notes.txt" --stdin "{{ … }}"`) |
| `--timeout` | default 60 s, at most 3600 s; the connection is closed when it passes |
| `--max-output` | bytes kept of each of stdout and stderr, default 1 MB, at most 16 MB |

The answer is `command: { ok, exit_code, stdout, stderr, duration_ms,
truncated }`. A command that ran and exited non-zero is an answer with
`ok: false`, not a failure; a refused credential, an unreachable machine, a
changed host key or a timeout fails the node.

## Rules

1. Zebflow starts the system `ssh` with an argument list, never through a local shell, with `BatchMode`, `IdentitiesOnly`, `StrictHostKeyChecking=yes` and a `known_hosts` holding only the credential's `host_key`. Agent and X11 forwarding, port forwarding and a terminal are off.
2. The private key and the `known_hosts` file are written to a private temporary folder for the one run and removed after it, whatever happened.
3. With `commands` set, a line must start with one of them and may not contain `;`, `&`, `|`, `` ` ``, `$(`, `>`, `<` or a line break; the machine's shell never sees a second command.
4. Nothing of the secret reaches the answer, the run record or a log.

## Implementation

`src/pipeline/nodes/basic/linux/`; the credential kind in
`src/platform/services/credential.rs`. The server image carries
`openssh-client`. Tests run a real `sshd` on a high port, and skip where none
is installed.
