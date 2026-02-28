const vscode = require("vscode");

function extractBlock(source, tagName) {
  const re = new RegExp(`<${tagName}\\b[^>]*>([\\s\\S]*?)<\\/${tagName}>`, "i");
  const m = source.match(re);
  return m ? m[1].trim() : "";
}

function extractPageMarkup(source) {
  const page = extractBlock(source, "Page");
  return page || source;
}

function htmlEnvelope(body) {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>TSX Preview</title>
    <style>
      body {
        margin: 0;
        padding: 0;
        font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
        background: #f5f7fb;
      }
      .toolbar {
        position: sticky;
        top: 0;
        z-index: 20;
        padding: 10px 14px;
        color: #dbe1ea;
        background: #111827;
        border-bottom: 1px solid #1f2937;
        font-size: 12px;
      }
      .viewport {
        margin: 18px;
        background: #ffffff;
        border: 1px solid #e5e7eb;
        border-radius: 10px;
        min-height: calc(100vh - 72px);
        overflow: auto;
      }
      .content {
        padding: 20px;
      }
      .note {
        margin-top: 10px;
        color: #6b7280;
        font-size: 12px;
      }
      [hidden] { display: none !important; }
    </style>
  </head>
  <body>
    <div class="toolbar">Zebflow TSX Preview (static template render)</div>
    <div class="viewport">
      <div class="content">
        ${body}
        <div class="note">
          Static preview only. Runtime logic from <code>export const app</code>
          is not executed in this extension preview.
        </div>
      </div>
    </div>
  </body>
</html>`;
}

function createOrUpdatePreview(panel, document) {
  const source = document.getText();
  const markup = extractPageMarkup(source);
  panel.webview.html = htmlEnvelope(markup);
}

function activate(context) {
  const openPanels = new Map();

  const disposable = vscode.commands.registerCommand("zebflow.previewTsx", async (uri) => {
    let targetUri = uri;
    if (!targetUri) {
      const active = vscode.window.activeTextEditor;
      if (!active) {
        vscode.window.showErrorMessage("No active TSX file to preview.");
        return;
      }
      targetUri = active.document.uri;
    }

    const document = await vscode.workspace.openTextDocument(targetUri);
    if (!document.fileName.endsWith(".tsx")) {
      vscode.window.showErrorMessage("Preview is only available for .tsx files.");
      return;
    }

    const key = document.uri.toString();
    let panel = openPanels.get(key);
    if (!panel) {
      panel = vscode.window.createWebviewPanel(
        "zebflowTsxPreview",
        `TSX Preview: ${document.fileName.split("/").pop()}`,
        vscode.ViewColumn.Beside,
        {
          enableScripts: false
        }
      );
      openPanels.set(key, panel);
      panel.onDidDispose(() => {
        openPanels.delete(key);
      });
    } else {
      panel.reveal(vscode.ViewColumn.Beside);
    }

    createOrUpdatePreview(panel, document);
  });

  const saveWatcher = vscode.workspace.onDidSaveTextDocument((doc) => {
    if (!doc.fileName.endsWith(".tsx")) {
      return;
    }
    const key = doc.uri.toString();
    const panel = openPanels.get(key);
    if (panel) {
      createOrUpdatePreview(panel, doc);
    }
  });

  context.subscriptions.push(disposable, saveWatcher);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
