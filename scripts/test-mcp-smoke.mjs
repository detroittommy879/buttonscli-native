#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const instanceId = "a".repeat(32);
const authToken = "b".repeat(64);
const requests = [];
let deniedRequest = null;
const server = createServer(async (request, response) => {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  const bodyText = Buffer.concat(chunks).toString("utf8");
  const body = bodyText ? JSON.parse(bodyText) : undefined;
  requests.push({
    method: request.method,
    url: request.url,
    authorization: request.headers.authorization,
    body,
  });

  const pathname = new URL(request.url, "http://127.0.0.1").pathname;
  if (request.headers.authorization !== `Bearer ${authToken}`) {
    response.writeHead(401, { "Content-Type": "application/json" });
    response.end(JSON.stringify({ error: "missing test authorization" }));
    return;
  }
  if (request.method === "GET" && pathname === "/v1/status") {
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(JSON.stringify({ ok: true, instanceId, connectedTabs: 2 }));
    return;
  }
  if (deniedRequest === `${request.method} ${pathname}`) {
    response.writeHead(403, { "Content-Type": "application/json" });
    response.end(JSON.stringify({ error: "remote control is not available for this installation" }));
    return;
  }
  response.writeHead(200, { "Content-Type": "application/json" });
  response.end(JSON.stringify({
    ok: true,
    instanceId,
    method: request.method,
    path: request.url,
    body,
    text: "READY marker",
    tab: { id: 7, title: "test tab", lastUpdatedAtMs: 100 },
  }));
});

let temporaryRoot;
let child;
let serverListening = false;
let allStdout = "";
let allStderr = "";
const pending = new Map();
let nextId = 1;

function rpc(method, params) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Timed out waiting for MCP response ${id}.`));
    }, 3000);
    pending.set(id, (message) => {
      clearTimeout(timeout);
      resolve(message);
    });
    child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  });
}

async function callTool(name, args = {}) {
  const message = await rpc("tools/call", { name, arguments: args });
  assert.equal(message.error, undefined, `${name} should use a tool result`);
  assert.ok(message.result, `${name} should return a result`);
  return message.result;
}

try {
  temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "buttonscli-mcp-smoke-"));
  const controlDir = path.join(temporaryRoot, "control");
  await mkdir(controlDir);

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  serverListening = true;
  const address = server.address();
  assert.ok(address && typeof address === "object");
  const descriptorPath = path.join(controlDir, `${instanceId}.json`);
  await writeFile(
    descriptorPath,
    JSON.stringify({
      schemaVersion: 1,
      instanceId,
      baseUrl: `http://127.0.0.1:${address.port}`,
      authToken,
      updatedAtMs: Date.now(),
      capabilities: ["status", "tabs", "read", "send", "key", "run", "presets"],
    }),
  );

  const helperPath = fileURLToPath(new URL("./buttonscli-mcp.mjs", import.meta.url));
  child = spawn(process.execPath, [helperPath], {
    env: { ...process.env, BUTTONSCLI_CONTROL_INFO_PATH: descriptorPath },
    stdio: ["pipe", "pipe", "pipe"],
  });
  const lines = readline.createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    allStdout += `${line}\n`;
    let message;
    try {
      message = JSON.parse(line);
    } catch (error) {
      allStderr += `Invalid MCP stdout JSON: ${String(error)}\n`;
      return;
    }
    const settle = pending.get(message.id);
    if (settle) {
      pending.delete(message.id);
      settle(message);
    }
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => (allStderr += chunk));

  const initialized = await rpc("initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "contract-smoke", version: "1" },
  });
  assert.equal(initialized.result?.serverInfo?.name, "buttonscli-native");
  const listed = await rpc("tools/list");
  const tools = listed.result?.tools ?? [];
  const toolNames = tools.map((tool) => tool.name).sort();
  assert.deepEqual(toolNames, [
    "create_tab", "help", "key", "open_layout", "preset_run", "presets",
    "read", "rename_tab", "run", "send", "status", "tabs",
    "wait_for_quiet", "wait_for_text",
  ].sort());
  assert.ok(tools.find((tool) => tool.name === "send")?.inputSchema?.properties?.delivery?.enum?.includes("bracketed"));

  assert.equal((await callTool("help")).structuredContent.help.includes("Selectors:"), true);
  assert.equal((await callTool("status")).structuredContent.instanceId, instanceId);
  assert.equal((await callTool("tabs")).structuredContent.method, "GET");
  assert.equal((await callTool("create_tab", { name: "build", shell: "pwsh", cwd: "G:/repo" })).structuredContent.body.cwd, "G:/repo");
  assert.equal((await callTool("rename_tab", { tab: "Build / Logs", name: "build-2" })).structuredContent.path, "/v1/tabs/Build%20%2F%20Logs/rename");
  assert.equal((await callTool("open_layout", { layout: "grid", names: ["one", "two"], columns: 2 })).structuredContent.body.columns, 2);
  assert.equal((await callTool("read", { tab: "active", lines: 3, from: "top" })).structuredContent.path, "/v1/tabs/active/read?lines=3&from=top");
  assert.equal((await callTool("wait_for_text", { tab: "server", text: "READY", intervalMs: 1 })).structuredContent.type, "wait-for-text");
  assert.equal((await callTool("wait_for_quiet", { tab: "server", quietMs: 0, intervalMs: 1 })).structuredContent.type, "wait-for-quiet");
  assert.equal((await callTool("send", { tab: "active", payloadBase64: "cHJpbnQ=", delivery: "bracketed", enter: true })).structuredContent.body.payloadBase64, "cHJpbnQ=");
  assert.equal((await callTool("run", { tab: "active", text: "echo ready", waitForText: "ready", ignoreCase: true })).structuredContent.body.waitForText, "ready");
  assert.equal((await callTool("key", { tab: "active", key: "ctrl+c" })).structuredContent.body.key, "ctrl+c");
  assert.equal((await callTool("presets")).structuredContent.method, "GET");
  assert.equal((await callTool("preset_run", { label: "prod ssh", tab: "active" })).structuredContent.body.label, "prod ssh");

  const routeRequests = requests.filter(({ url }) => url !== "/v1/status");
  assert.deepEqual(routeRequests.map(({ method, url }) => `${method} ${url}`), [
    "GET /v1/tabs",
    "POST /v1/tabs",
    "POST /v1/tabs/Build%20%2F%20Logs/rename",
    "POST /v1/layout/open",
    "GET /v1/tabs/active/read?lines=3&from=top",
    "GET /v1/tabs/server/read?chars=12000&from=bottom",
    "GET /v1/tabs/server/read?chars=12000&from=bottom",
    "POST /v1/tabs/active/send",
    "POST /v1/tabs/active/run",
    "POST /v1/tabs/active/key",
    "GET /v1/presets",
    "POST /v1/presets/run",
  ]);
  assert.ok(requests.every(({ authorization }) => authorization === `Bearer ${authToken}`));
  assert.equal(requests.find(({ url }) => url === "/v1/tabs/active/send")?.body?.delivery, "bracketed");

  deniedRequest = "POST /v1/tabs/active/send";
  const denied = await callTool("send", { text: "blocked" });
  assert.equal(denied.isError, true);
  assert.match(denied.structuredContent.error, /not available for this installation/);
  deniedRequest = null;

  const invalid = await callTool("send", {});
  assert.equal(invalid.isError, true);
  assert.match(invalid.structuredContent.error, /exactly one payload source/);

  await new Promise((resolve) => server.close(resolve));
  serverListening = false;
  const disconnected = await callTool("tabs");
  assert.equal(disconnected.isError, true);
  assert.match(disconnected.structuredContent.error, /Could not reach the native ButtonsCLI control API/);

  child.stdin.end();
  const [exitCode] = await once(child, "close");
  assert.equal(exitCode, 0, allStderr);
  assert.equal(allStderr, "");
  assert.ok(!allStdout.includes(authToken), "the descriptor token must never be printed");
  process.stdout.write("MCP stdio route, gate, and disconnect contract passed.\n");
} finally {
  child?.kill();
  if (serverListening) await new Promise((resolve) => server.close(resolve));
  if (temporaryRoot) await rm(temporaryRoot, { recursive: true, force: true });
}
