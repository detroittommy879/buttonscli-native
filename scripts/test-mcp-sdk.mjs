#!/usr/bin/env node

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { createServer } from "node:http";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";

const instanceId = "c".repeat(32);
const authToken = "d".repeat(64);
const requests = [];
const api = createServer(async (request, response) => {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  const bodyText = Buffer.concat(chunks).toString("utf8");
  const body = bodyText ? JSON.parse(bodyText) : undefined;
  requests.push({ method: request.method, url: request.url, authorization: request.headers.authorization, body });

  if (request.headers.authorization !== `Bearer ${authToken}`) {
    response.writeHead(401, { "Content-Type": "application/json" });
    response.end(JSON.stringify({ error: "missing test authorization" }));
    return;
  }
  response.writeHead(200, { "Content-Type": "application/json" });
  response.end(JSON.stringify({ ok: true, instanceId, method: request.method, path: request.url, body }));
});

const helperPath = fileURLToPath(new URL("./buttonscli-mcp.mjs", import.meta.url));
const descriptorArg = process.argv.indexOf("--descriptor");
const externalDescriptorPath = descriptorArg === -1 ? null : process.argv[descriptorArg + 1];
if (descriptorArg !== -1 && !externalDescriptorPath) {
  throw new Error("--descriptor requires a path to a live native control descriptor.");
}
let temporaryRoot;
let apiListening = false;
let client;

try {
  temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "buttonscli-mcp-sdk-"));
  const controlDir = path.join(temporaryRoot, "control");
  const sdkRoot = path.join(temporaryRoot, "sdk");
  await mkdir(sdkRoot);

  const installArgs = [
    "install", "--prefix", sdkRoot, "--no-save", "--no-package-lock", "--ignore-scripts",
    "@modelcontextprotocol/client@2.2.0",
  ];
  const npmCliPath = path.join(path.dirname(process.execPath), "node_modules", "npm", "bin", "npm-cli.js");
  if (process.platform === "win32" && !existsSync(npmCliPath)) {
    throw new Error(`The npm CLI beside Node was not found at ${npmCliPath}.`);
  }
  const install = spawnSync(process.platform === "win32" ? process.execPath : "npm", [
    ...(process.platform === "win32" ? [npmCliPath] : []),
    ...installArgs,
  ], { encoding: "utf8", timeout: 120_000 });
  if (install.error || install.status !== 0) {
    throw new Error(`Could not install the pinned MCP client SDK in a temporary directory.\n${install.stderr || install.error || `exit ${install.status}`}`);
  }

  let descriptorPath = externalDescriptorPath;
  if (!descriptorPath) {
    await mkdir(controlDir);
    await new Promise((resolve, reject) => {
      api.once("error", reject);
      api.listen(0, "127.0.0.1", resolve);
    });
    apiListening = true;
    const address = api.address();
    assert.ok(address && typeof address === "object");
    descriptorPath = path.join(controlDir, `${instanceId}.json`);
    await writeFile(descriptorPath, JSON.stringify({
      schemaVersion: 1,
      instanceId,
      baseUrl: `http://127.0.0.1:${address.port}`,
      authToken,
      updatedAtMs: Date.now(),
      capabilities: ["status", "tabs", "read", "send", "key", "run", "presets"],
    }));
  }
  const descriptor = JSON.parse(await readFile(descriptorPath, "utf8"));

  const resolveFromSdk = createRequire(path.join(sdkRoot, "package.json"));
  const clientModule = await import(pathToFileURL(resolveFromSdk.resolve("@modelcontextprotocol/client")).href);
  const stdioModule = await import(pathToFileURL(resolveFromSdk.resolve("@modelcontextprotocol/client/stdio")).href);
  client = new clientModule.Client({ name: "buttonscli-native-acceptance", version: "1.0.0" });
  await client.connect(new stdioModule.StdioClientTransport({
    command: process.execPath,
    args: [helperPath],
    env: { ...process.env, BUTTONSCLI_CONTROL_INFO_PATH: descriptorPath },
  }));

  const listed = await client.listTools();
  const names = listed.tools.map((tool) => tool.name).sort();
  assert.deepEqual(names, [
    "create_tab", "help", "key", "open_layout", "preset_run", "presets",
    "read", "rename_tab", "run", "send", "status", "tabs",
    "wait_for_quiet", "wait_for_text",
  ].sort());

  const status = await client.callTool({ name: "status", arguments: {} });
  assert.equal(status.isError, false);
  assert.equal(status.structuredContent.instanceId, descriptor.instanceId);
  const tabs = await client.callTool({ name: "tabs", arguments: {} });
  assert.equal(tabs.isError, false);
  if (!externalDescriptorPath) {
    assert.equal(tabs.structuredContent.method, "GET");
    assert.ok(requests.every(({ authorization }) => authorization === `Bearer ${authToken}`));
    assert.deepEqual(requests.map(({ method, url }) => `${method} ${url}`), [
      "GET /v1/status",
      "GET /v1/tabs",
    ]);
  }

  process.stdout.write(`Official MCP TypeScript client connected to ${externalDescriptorPath ? "the live native app" : "the fake API"}, listed 14 tools, and called status/tabs successfully.\n`);
} finally {
  if (client) await client.close().catch(() => {});
  if (apiListening) await new Promise((resolve) => api.close(resolve));
  if (temporaryRoot) await rm(temporaryRoot, { recursive: true, force: true });
}
