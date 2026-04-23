# Platform Executable

## Formal Concept

`zebflow` is the main platform executable.

It should support two levels of entry:

1. **platform-level entry**
2. **project-level entry**

## Current Commands

Current real commands:

- `zebflow`
- `zebflow standalone`
- `zebflow controller`
- `zebflow office`
- `zebflow k8s cluster ...`

Current meaning:

| Command | Meaning |
| --- | --- |
| `zebflow` | start the combined standalone platform |
| `zebflow standalone` | start the combined standalone platform |
| `zebflow controller` | start the control-plane oriented server |
| `zebflow office` | start the execution-plane oriented server |
| `zebflow k8s cluster ...` | manage Kubernetes cluster manifest folders on disk |

Current aliases:

- `master` = `controller`
- `worker` = `office`

## Target Command Model

The long-term command model should distinguish:

- platform boot
- desktop shell
- project runtime
- infrastructure management

Target commands:

- `zebflow`
- `zebflow standalone`
- `zebflow desktop`
- `zebflow run <project-or-marketplace-asset-url>`
- `zebflow controller`
- `zebflow office`

## Meaning

### `zebflow standalone`

Platform boot.

Meaning:

- start the Zebflow platform server
- expose login, home, project routes, Studio, and runtime services
- no specific project intent

### `zebflow`

Platform boot alias.

Meaning:

- same behavior as `zebflow standalone`
- primary user-facing command for opening the full local platform UI

### `zebflow desktop`

Desktop shell boot.

Meaning:

- start the local desktop shell for Zebflow
- use a lean native desktop wrapper around the existing Zebflow web platform
- give users the easiest local entry for running projects and opening Studio

Important note:

- desktop is not a separate application model
- it is a thin local shell around the same Zebflow runtime and Studio
- anything advanced can still be done through Studio itself
- desktop does not introduce new platform semantics
- desktop actions are preselected local platform actions such as:
  - start local runtime
  - open launcher
  - install project to local office
  - open app route
  - open Studio route

### `zebflow run <project-or-marketplace-asset-url>`

Project runtime entry.

Meaning:

- run a local installed project as an app
- or fetch/materialize a project app from a marketplace asset URL first
- ensure runtime/bootstrap is active
- expose the project's public app route locally

Current implementation direction:

- local project:
  - `zebflow run my-project`
- remote marketplace asset:
  - `zebflow run http://host/api/projects/{owner}/{project}/marketplace/remote/assets/{package}/{version}`

If project is not installed yet, this command may later auto-install it first.

### `zebflow controller` / `zebflow office`

Infrastructure entry.

Meaning:

- office/controller topology management
- not normal end-user UI entry
- kept for runtime placement and federated office management

## First-Principles Split

The executable should separate these concerns clearly:

- `zebflow` = boot the full local platform UI
- `standalone` = boot Zebflow itself
- `desktop` = boot a thin local desktop shell around Zebflow
- `run` = use project as app/runtime
- `controller/office` = infrastructure/runtime topology

## Desktop Product Decision

Desktop should be:

- a lean native shell
- minimal wrapper only
- not a second product architecture

Desktop responsibilities:

- start or connect to the local Zebflow runtime
- show a simple launcher first
- provide the easiest local user entry
- route users into:
  - Run
  - Studio
  - Marketplace

Desktop should not duplicate Studio logic.

The main editor, automation, data, pipeline, and agentic experiences should
still live in Studio, whether used locally or through the web.

Current lightweight implementation direction:

- direct Rust desktop shell
- local launcher surface
- embedded webview
- local `zebflow standalone` behind the shell
- same Zebflow platform routes underneath

## Intended User Flow

### Desktop-first user

1. Install Zebflow Desktop
2. Open Zebflow
3. See launcher:
   - Run App
   - Open Studio
   - Open Marketplace
4. Install or open a project locally
5. Run it locally
6. Optionally open it in Studio

### CLI-first user

1. Install Zebflow CLI
2. Run:
   - `zebflow run <project>`
   - or `zebflow run <marketplace-asset-url>`
3. Optionally open:
   - `zebflow`

### Server/web user

1. Start:
   - `zebflow standalone`
2. Use the same Studio and runtime through the browser

## Important Principle

Zebflow should have one core runtime and one core Studio.

Desktop, CLI, and web/server are different entry modes into the same system,
not different application models.

## Publish Model

Desktop distribution should be treated like first-class release artifacts.

- Docker image:
  - runtime/server distribution
- GitHub Releases:
  - desktop executables and archives

Recommended release outputs:

- macOS:
  - `Zebflow-Desktop-macos-arm64.zip`
  - later signed/notarized `.dmg`
- Windows:
  - `Zebflow-Desktop-windows-x64.zip`
  - later installer
- Linux:
  - `Zebflow-Desktop-linux-x64.tar.gz`
  - later AppImage or distro packages

Website download links should point to GitHub Release assets.
