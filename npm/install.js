#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { createWriteStream, mkdirSync } = fs;

const REPO = "zebflow/zebflow";
const VERSION = process.env.ZEBFLOW_VERSION || `v${require("./package.json").version}`;
const BIN_DIR = path.join(__dirname, "bin");

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  const map = {
    "darwin-arm64": { asset: "zebflow-darwin-arm64.tar.gz", binary: "zebflow" },
    "linux-x64": { asset: "zebflow-linux-amd64.tar.gz", binary: "zebflow" },
    "linux-arm64": { asset: "zebflow-linux-arm64.tar.gz", binary: "zebflow" },
    "win32-x64": { asset: "zebflow-windows-amd64.zip", binary: "zebflow.exe" },
  };

  const key = `${platform}-${arch}`;
  const entry = map[key];
  if (!entry) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported: ${Object.keys(map).join(", ")}`);
    process.exit(1);
  }
  return entry;
}

function downloadFile(url) {
  return new Promise((resolve, reject) => {
    const follow = (url, redirects = 0) => {
      if (redirects > 5) return reject(new Error("Too many redirects"));
      const mod = url.startsWith("https") ? https : require("http");
      mod.get(url, { headers: { "User-Agent": "zebflow-npm-installer" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return follow(res.headers.location, redirects + 1);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      }).on("error", reject);
    };
    follow(url);
  });
}

async function extractTarGz(buffer, destDir) {
  const tmpFile = path.join(destDir, "_tmp.tar.gz");
  fs.writeFileSync(tmpFile, buffer);
  execSync(`tar -xzf "${tmpFile}" -C "${destDir}"`, { stdio: "pipe" });
  fs.unlinkSync(tmpFile);
}

async function extractZip(buffer, destDir) {
  const tmpFile = path.join(destDir, "_tmp.zip");
  fs.writeFileSync(tmpFile, buffer);
  execSync(`tar -xf "${tmpFile}" -C "${destDir}"`, { stdio: "pipe" });
  fs.unlinkSync(tmpFile);
}

async function main() {
  const { asset, binary } = getPlatform();
  const url = `https://github.com/${REPO}/releases/download/${VERSION}/${asset}`;

  console.log(`[zebflow] Downloading ${asset} (${VERSION})...`);

  const buffer = await downloadFile(url);

  mkdirSync(BIN_DIR, { recursive: true });

  if (asset.endsWith(".tar.gz")) {
    await extractTarGz(buffer, BIN_DIR);
  } else {
    await extractZip(buffer, BIN_DIR);
  }

  const binaryPath = path.join(BIN_DIR, binary);
  if (!fs.existsSync(binaryPath)) {
    console.error(`[zebflow] Binary not found after extraction: ${binaryPath}`);
    process.exit(1);
  }

  fs.chmodSync(binaryPath, 0o755);
  console.log(`[zebflow] Installed ${binary} to ${BIN_DIR}`);
}

main().catch((err) => {
  console.error(`[zebflow] Installation failed: ${err.message}`);
  process.exit(1);
});
