# VS Code Zebflow Extension

Lightweight VS Code extension for:

- `*.tsx` (Zebflow Reactive Web Engine template files)
- `*.zf.json` (Zebflow pipeline contract files)

## Features

- Language registration for `.tsx` and `.zf.json`
- Basic syntax highlighting (template/script/style aware for TSX)
- Snippets for common TSX and ZF JSON structure
- `Zebflow: Preview TSX` command
  - Opens static webview preview from the current `<Page>...</Page>` body
  - Auto refresh on save

## Install (Local Development)

1. Open folder `integrations/vscode-zebflow` in terminal.
2. Run:
   - `npm install`
   - `npm run package`
3. Install generated `.vsix` via VS Code:
   - Command Palette -> `Extensions: Install from VSIX...`

## Notes

- Preview currently renders the `<Page>...</Page>` body only (static render).
- Runtime control logic (`export const app`) is not executed in preview.
