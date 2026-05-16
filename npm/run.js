#!/usr/bin/env node
"use strict";

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binary = process.platform === "win32" ? "zebflow.exe" : "zebflow";
const binaryPath = path.join(__dirname, "bin", binary);

if (!fs.existsSync(binaryPath)) {
  console.error("[zebflow] Binary not found. Run `npm rebuild zebflow` or reinstall.");
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

child.on("close", (code) => process.exit(code ?? 1));
child.on("error", (err) => {
  console.error(`[zebflow] Failed to start: ${err.message}`);
  process.exit(1);
});
