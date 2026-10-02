import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const helperPath = fileURLToPath(new URL("./buttonsclictl.mjs", import.meta.url));
const instanceId = "a".repeat(32);
const authToken = "b".repeat(64);

// Exercise the shipped helper in another process against real local HTTP.
async function withApi(handler, check) {
  const root = await mkdtemp(path.join(os.tmpdir(), "buttonscli-cli-test-"));
  const requests = [];
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    requests.push({ method: request.method, url: request.url });
    assert.equal(request.headers.authorization, `Bearer ${authToken}`);
    handler(request, response);
  });
  try {
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const infoPath = path.join(root, `${instanceId}.json`);
    await writeFile(infoPath, JSON.stringify({
      schemaVersion: 1, instanceId, authToken,
      baseUrl: `http://127.0.0.1:${server.address().port}`,
      capabilities: [], updatedAtMs: Date.now(),
    }));
    const cli = (args) => new Promise((resolve, reject) => {
      const started = performance.now();
      const child = spawn(process.execPath, [helperPath, ...args], {
        env: { ...process.env, BUTTONSCLI_CONTROL_INFO_PATH: infoPath },
        stdio: ["ignore", "pipe", "pipe"], windowsHide: true,
      });
      let stdout = "";
      let stderr = "";
      child.stdout.on("data", (data) => { stdout += data; });
      child.stderr.on("data", (data) => { stderr += data; });
      const timer = setTimeout(() => {
        child.kill();
        reject(new Error("CLI exceeded the 15-second test deadline"));
      }, 15_000);
      child.once("error", (error) => { clearTimeout(timer); reject(error); });
      child.once("close", (code) => {
        clearTimeout(timer);
        assert.ok(!(`${stdout}${stderr}`).includes(authToken));
        resolve({ code, stdout, stderr, elapsedMs: performance.now() - started });
      });
    });
    await check(cli, requests);
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
    const resolvedRoot = path.resolve(root);
    assert.equal(path.dirname(resolvedRoot), path.resolve(os.tmpdir()));
    assert.ok(path.basename(resolvedRoot).startsWith("buttonscli-cli-test-"));
    await rm(resolvedRoot, { recursive: true, force: true });
  }
}

test("wait deadline bounds a read with no response headers", async () => {
  await withApi(() => {}, async (cli, requests) => {
    const result = await cli([
      "wait-for-text", "--text", "never", "--timeout-ms", "500", "--interval-ms", "10000",
    ]);
    assert.equal(result.code, 1);
    assert.match(result.stderr, /request timed out/i);
    assert.ok(result.elapsedMs < 4000, `${result.elapsedMs}ms`);
    assert.equal(requests.length, 1);
  });
});

test("ordinary request deadline includes a stalled response body", async () => {
  await withApi((_request, response) => {
    response.writeHead(200, { "Content-Type": "application/json" });
    response.write('{"ok":');
  }, async (cli, requests) => {
    const result = await cli(["status", "--json"]);
    assert.equal(result.code, 1);
    assert.match(result.stderr, /request timed out/i);
    assert.equal(requests.length, 1);
  });
});

test("a timed-out write warns about delivery and never retries", async () => {
  await withApi(() => {}, async (cli, requests) => {
    const result = await cli(["send", "--text", "echo fixture", "--enter", "--json"]);
    assert.equal(result.code, 1);
    assert.match(result.stderr, /Delivery may have begun; check the target before repeating/);
    assert.deepEqual(requests, [{ method: "POST", url: "/v1/tabs/active/send" }]);
  });
});

test("poll sleep is bounded by the remaining overall deadline", async () => {
  await withApi((_request, response) => {
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(JSON.stringify({ text: "", tab: { lastUpdatedAtMs: Date.now() } }));
  }, async (cli) => {
    for (const args of [
      ["wait-for-text", "--text", "never"],
      ["wait-for-quiet", "--quiet-ms", "10000"],
    ]) {
      const result = await cli([...args, "--timeout-ms", "500", "--interval-ms", "10000"]);
      assert.equal(result.code, 1);
      assert.match(result.stderr, /timed out/i);
      assert.ok(result.elapsedMs < 4000, `${result.elapsedMs}ms`);
    }
  });
});

test("malformed response body is not echoed into CLI errors", async () => {
  await withApi((_request, response) => {
    response.writeHead(502, { "Content-Type": "text/plain" });
    response.end("private-body-canary not JSON");
  }, async (cli) => {
    const result = await cli(["status", "--json"]);
    assert.equal(result.code, 1);
    assert.match(result.stderr, /invalid JSON \(HTTP 502\)/);
    assert.ok(!result.stderr.includes("private-body-canary"));
  });
});
