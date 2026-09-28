#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const instanceId = "a".repeat(32);
const authToken = "b".repeat(64);
const requests = [];
const server = createServer((request, response) => {
  requests.push({
    url: request.url,
    authorization: request.headers.authorization,
  });
  if (request.url !== "/v1/status" || request.method !== "GET") {
    response.writeHead(404).end();
    return;
  }
  if (request.headers.authorization !== `Bearer ${authToken}`) {
    response.writeHead(401).end();
    return;
  }
  response.writeHead(200, { "Content-Type": "application/json" });
  response.end(JSON.stringify({ ok: true, instanceId, connectedTabs: 2 }));
});

let temporaryRoot;
let child;
try {
  temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "buttonscli-mcp-smoke-"));
  const controlDir = path.join(temporaryRoot, "control");
  await mkdir(controlDir);

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert.ok(address && typeof address === "object");
  await writeFile(
    path.join(controlDir, `${instanceId}.json`),
    JSON.stringify({
      schemaVersion: 1,
      instanceId,
      baseUrl: `http://127.0.0.1:${address.port}`,
      authToken,
      updatedAtMs: Date.now(),
      capabilities: ["status"],
    }),
  );

  const helperPath = fileURLToPath(new URL("./buttonscli-mcp.mjs", import.meta.url));
  child = spawn(process.execPath, [helperPath], {
    env: { ...process.env, BUTTONSCLI_CONTROL_INFO_PATH: controlDir },
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => (stdout += chunk));
  child.stderr.on("data", (chunk) => (stderr += chunk));

  const messages = [
    { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "smoke", version: "1" } } },
    { jsonrpc: "2.0", id: 2, method: "tools/list" },
    { jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "status", arguments: {} } },
  ];
  const closed = once(child, "close");
  const timeout = setTimeout(() => child.kill(), 5000);
  child.stdin.end(`${messages.map((message) => JSON.stringify(message)).join("\n")}\n`);
  const [exitCode] = await closed;
  clearTimeout(timeout);

  assert.equal(exitCode, 0, `MCP helper failed: ${stderr}`);
  assert.equal(stderr, "");
  const responses = stdout
    .trim()
    .split("\n")
    .map((line) => JSON.parse(line));
  const byId = new Map(responses.map((response) => [response.id, response]));
  assert.equal(byId.get(1)?.result?.serverInfo?.name, "buttonscli-native");
  const tools = byId.get(2)?.result?.tools ?? [];
  assert.ok(tools.some((tool) => tool.name === "status"));
  assert.ok(
    tools.find((tool) => tool.name === "send")?.inputSchema?.properties?.delivery?.enum?.includes("bracketed"),
  );
  assert.deepEqual(byId.get(3)?.result?.structuredContent, {
    ok: true,
    instanceId,
    connectedTabs: 2,
  });
  assert.equal(requests.length, 2, "discovery and tool call should both check status");
  assert.ok(requests.every((request) => request.authorization === `Bearer ${authToken}`));
  assert.ok(!stdout.includes(authToken), "the descriptor token must never be printed");
  process.stdout.write("MCP stdio initialize/list/status smoke passed.\n");
} finally {
  child?.kill();
  await new Promise((resolve) => server.close(resolve));
  if (temporaryRoot) {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}
