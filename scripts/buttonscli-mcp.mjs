#!/usr/bin/env node

// Adapted from the read-only legacy MCP helper template
// (SHA-256 763661545A727180853E743C4682872D98F59298BAB764761000A5656B1C5F83).
// Native changes use only the .buttonscli-native control API and descriptor.

import os from "node:os";
import path from "node:path";
import process from "node:process";
import { readFile, readdir, stat } from "node:fs/promises";

const PROTOCOL_VERSION = "2025-06-18";
const SERVER_INFO = {
  name: "buttonscli-native",
  version: "1.0.0",
};
const MAX_MESSAGE_BYTES = 1024 * 1024;

const HELP_TEXT = `ButtonsCLI Native MCP tools mirror the local terminal control surface.

Available tools:
- help
- status
- tabs
- create_tab
- rename_tab
- open_layout
- read
- wait_for_text
- wait_for_quiet
- send
- run
- key
- presets
- preset_run

Selectors:
- active: currently focused tab
- <ptyId>: numeric PTY id
- <tabId>: internal tab id
- <title>: exact tab title, case-insensitive
`;

function defaultControlInfoPath(homedir = os.homedir()) {
  return path.join(homedir, ".buttonscli-native", "control");
}

async function loadConnectionInfo() {
  const infoPath = process.env.BUTTONSCLI_CONTROL_INFO_PATH || defaultControlInfoPath();
  const infoStats = await stat(infoPath).catch(() => null);
  if (infoStats?.isDirectory()) {
    return await discoverNativeConnectionInfo(infoPath);
  }
  if (!infoStats?.isFile()) {
    throw new Error(`Native ButtonsCLI discovery path is unavailable: ${infoPath}`);
  }
  return await readNativeConnectionInfo(infoPath);
}

async function discoverNativeConnectionInfo(controlDir) {
  const names = await readdir(controlDir).catch(() => []);
  const candidatePaths = names
    .filter((name) => name.toLowerCase().endsWith(".json"))
    .map((name) => path.join(controlDir, name));
  const active = [];
  for (const candidatePath of candidatePaths) {
    const connection = await readNativeConnectionInfo(candidatePath).catch(() => null);
    if (connection && (await isLiveNativeInstance(connection))) {
      active.push(connection);
    }
  }
  if (active.length === 1) {
    return active[0];
  }
  if (active.length > 1) {
    throw new Error(
      `More than one native ButtonsCLI instance is running. Set BUTTONSCLI_CONTROL_INFO_PATH to the exact instance file under ${controlDir}.`,
    );
  }
  throw new Error(
    `No running native ButtonsCLI instance was found in ${controlDir}. Start ButtonsCLI or set BUTTONSCLI_CONTROL_INFO_PATH to its exact connection file.`,
  );
}

async function readNativeConnectionInfo(infoPath) {
  const absolutePath = path.resolve(infoPath);
  const raw = await readFile(absolutePath, "utf8").catch((error) => {
    throw new Error(
      `Could not read native ButtonsCLI connection info at ${absolutePath}. ${error.message}`,
    );
  });
  const parsed = JSON.parse(raw);
  const expectedName = `${parsed.instanceId}.json`;
  const endpoint = new URL(parsed.baseUrl);
  if (
    parsed.schemaVersion !== 1 ||
    !/^[0-9a-f]{32}$/.test(parsed.instanceId) ||
    path.basename(absolutePath).toLowerCase() !== expectedName.toLowerCase() ||
    !/^[0-9a-f]{64}$/.test(parsed.authToken) ||
    endpoint.protocol !== "http:" ||
    endpoint.hostname !== "127.0.0.1" ||
    !endpoint.port ||
    endpoint.username ||
    endpoint.password ||
    endpoint.pathname !== "/" ||
    endpoint.search ||
    endpoint.hash ||
    !Array.isArray(parsed.capabilities) ||
    !Number.isFinite(parsed.updatedAtMs)
  ) {
    throw new Error(`Invalid native control descriptor: ${absolutePath}`);
  }
  return {
    infoPath: absolutePath,
    baseUrl: endpoint.origin,
    authToken: parsed.authToken,
    instanceId: parsed.instanceId,
    updatedAtMs: parsed.updatedAtMs,
  };
}

async function isLiveNativeInstance(connection) {
  try {
    const response = await fetch(`${connection.baseUrl}/v1/status`, {
      headers: { Authorization: `Bearer ${connection.authToken}` },
      signal: AbortSignal.timeout(750),
    });
    if (!response.ok) {
      return false;
    }
    const status = await response.json();
    return status.instanceId === connection.instanceId;
  } catch {
    return false;
  }
}

async function apiRequest(method, pathname, body) {
  const connection = await loadConnectionInfo();
  let response;
  try {
    response = await fetch(`${connection.baseUrl}${pathname}`, {
      method,
      headers: {
        Authorization: `Bearer ${connection.authToken}`,
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Could not reach the native ButtonsCLI control API using ${connection.infoPath}. Start ButtonsCLI and retry. ${detail}`,
    );
  }

  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;

  if (!response.ok) {
    throw new Error(
      payload?.error || `Control API returned ${response.status}.`,
    );
  }

  return payload;
}

function makeTextResult(text, structuredContent, isError = false) {
  return {
    content: [
      {
        type: "text",
        text,
      },
    ],
    structuredContent,
    isError,
  };
}

function makeJsonResult(value) {
  return makeTextResult(JSON.stringify(value, null, 2), value, false);
}

function asOptionalString(value) {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function asOptionalNumber(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? value
    : undefined;
}

function asOptionalBoolean(value) {
  return typeof value === "boolean" ? value : undefined;
}

async function callTool(name, args) {
  switch (name) {
    case "help":
      return makeTextResult(HELP_TEXT, { help: HELP_TEXT });
    case "status":
      return makeJsonResult(await apiRequest("GET", "/v1/status"));
    case "tabs":
      return makeJsonResult(await apiRequest("GET", "/v1/tabs"));
    case "create_tab":
      return makeJsonResult(
        await apiRequest("POST", "/v1/tabs", {
          name: asOptionalString(args?.name),
          shell: asOptionalString(args?.shell),
          cwd: asOptionalString(args?.cwd),
        }),
      );
    case "rename_tab": {
      const tab = asOptionalString(args?.tab) || "active";
      return makeJsonResult(
        await apiRequest("POST", `/v1/tabs/${encodeURIComponent(tab)}/rename`, {
          name: asOptionalString(args?.name),
        }),
      );
    }
    case "open_layout":
      return makeJsonResult(
        await apiRequest("POST", "/v1/layout/open", {
          layout: asOptionalString(args?.layout),
          names: Array.isArray(args?.names)
            ? args.names.filter((value) => typeof value === "string" && value)
            : [],
          columns: asOptionalNumber(args?.columns),
          shell: asOptionalString(args?.shell),
          cwd: asOptionalString(args?.cwd),
        }),
      );
    case "read": {
      const tab = asOptionalString(args?.tab) || "active";
      const query = new URLSearchParams();
      const chars = asOptionalNumber(args?.chars);
      const lines = asOptionalNumber(args?.lines);
      const from = asOptionalString(args?.from);
      if (chars !== undefined) {
        query.set("chars", String(chars));
      }
      if (lines !== undefined) {
        query.set("lines", String(lines));
      }
      if (from) {
        query.set("from", from);
      }

      return makeJsonResult(
        await apiRequest(
          "GET",
          `/v1/tabs/${encodeURIComponent(tab)}/read?${query.toString()}`,
        ),
      );
    }
    case "wait_for_text": {
      const tab = asOptionalString(args?.tab) || "active";
      const text = asOptionalString(args?.text);
      if (!text) {
        throw new Error("wait_for_text requires text.");
      }

      const timeoutMs = asOptionalNumber(args?.timeoutMs) ?? 15000;
      const intervalMs = asOptionalNumber(args?.intervalMs) ?? 250;
      const chars = asOptionalNumber(args?.chars) ?? 12000;
      const ignoreCase = asOptionalBoolean(args?.ignoreCase) ?? false;
      const startedAt = Date.now();

      while (true) {
        const payload = await apiRequest(
          "GET",
          `/v1/tabs/${encodeURIComponent(tab)}/read?chars=${chars}&from=bottom`,
        );
        const haystack = String(payload?.text || "");
        const matched = ignoreCase
          ? haystack.toLowerCase().includes(text.toLowerCase())
          : haystack.includes(text);
        if (matched) {
          return makeJsonResult({
            ok: true,
            type: "wait-for-text",
            elapsedMs: Date.now() - startedAt,
            tab: payload.tab,
            text: haystack,
            needle: text,
          });
        }

        if (Date.now() - startedAt >= timeoutMs) {
          throw new Error(`Timed out waiting for text '${text}'.`);
        }

        await new Promise((resolve) => setTimeout(resolve, intervalMs));
      }
    }
    case "wait_for_quiet": {
      const tab = asOptionalString(args?.tab) || "active";
      const timeoutMs = asOptionalNumber(args?.timeoutMs) ?? 15000;
      const intervalMs = asOptionalNumber(args?.intervalMs) ?? 250;
      const quietMs = asOptionalNumber(args?.quietMs) ?? 1200;
      const chars = asOptionalNumber(args?.chars) ?? 12000;
      const startedAt = Date.now();
      let lastSignature = "";
      let stableSince = Date.now();

      while (true) {
        const payload = await apiRequest(
          "GET",
          `/v1/tabs/${encodeURIComponent(tab)}/read?chars=${chars}&from=bottom`,
        );
        const text = String(payload?.text || "");
        const signature = `${payload?.tab?.lastUpdatedAtMs || 0}:${text}`;
        const now = Date.now();

        if (signature !== lastSignature) {
          lastSignature = signature;
          stableSince = now;
        }

        if (now - stableSince >= quietMs) {
          return makeJsonResult({
            ok: true,
            type: "wait-for-quiet",
            elapsedMs: now - startedAt,
            quietMs,
            tab: payload.tab,
            text,
          });
        }

        if (now - startedAt >= timeoutMs) {
          throw new Error("Timed out waiting for quiet output.");
        }

        await new Promise((resolve) => setTimeout(resolve, intervalMs));
      }
    }
    case "send": {
      const tab = asOptionalString(args?.tab) || "active";
      const text = asOptionalString(args?.text);
      const payloadBase64 = asOptionalString(args?.payloadBase64);
      const delivery = asOptionalString(args?.delivery);
      const delayMs = asOptionalNumber(args?.delayMs);
      const providedSources =
        Number(Boolean(text)) + Number(Boolean(payloadBase64));
      if (providedSources !== 1) {
        throw new Error(
          "send requires exactly one payload source: text or payloadBase64.",
        );
      }

      return makeJsonResult(
        await apiRequest("POST", `/v1/tabs/${encodeURIComponent(tab)}/send`, {
          ...(text ? { text } : { payloadBase64 }),
          delivery,
          delayMs,
          enter: asOptionalBoolean(args?.enter) ?? false,
        }),
      );
    }
    case "run": {
      const tab = asOptionalString(args?.tab) || "active";
      const text = asOptionalString(args?.text);
      const payloadBase64 = asOptionalString(args?.payloadBase64);
      const delivery = asOptionalString(args?.delivery);
      const delayMs = asOptionalNumber(args?.delayMs);
      const providedSources =
        Number(Boolean(text)) + Number(Boolean(payloadBase64));
      if (providedSources !== 1) {
        throw new Error(
          "run requires exactly one payload source: text or payloadBase64.",
        );
      }

      return makeJsonResult(
        await apiRequest("POST", `/v1/tabs/${encodeURIComponent(tab)}/run`, {
          ...(text ? { text } : { payloadBase64 }),
          delivery,
          delayMs,
          enter: asOptionalBoolean(args?.enter) ?? true,
          chars: asOptionalNumber(args?.chars),
          quietMs: asOptionalNumber(args?.quietMs),
          maxWaitMs: asOptionalNumber(args?.maxWaitMs),
          intervalMs: asOptionalNumber(args?.intervalMs),
          waitForText: asOptionalString(args?.waitForText),
          ignoreCase: asOptionalBoolean(args?.ignoreCase) ?? false,
        }),
      );
    }
    case "key": {
      const tab = asOptionalString(args?.tab) || "active";
      const key = asOptionalString(args?.key);
      if (!key) {
        throw new Error("key requires a key value.");
      }

      return makeJsonResult(
        await apiRequest("POST", `/v1/tabs/${encodeURIComponent(tab)}/key`, {
          key,
        }),
      );
    }
    case "presets":
      return makeJsonResult(await apiRequest("GET", "/v1/presets"));
    case "preset_run":
      return makeJsonResult(
        await apiRequest("POST", "/v1/presets/run", {
          label: asOptionalString(args?.label),
          tab: asOptionalString(args?.tab),
        }),
      );
    default:
      throw new Error(`Unknown tool: ${name}`);
  }
}

const TOOL_DEFINITIONS = [
  {
    name: "help",
    title: "Help",
    description: "Show the ButtonsCLI MCP tool list and selector rules.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: "status",
    title: "Status",
    description: "Show local ButtonsCLI control server status.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: "tabs",
    title: "Tabs",
    description: "List current ButtonsCLI tabs and PTYs.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: "create_tab",
    title: "Create Tab",
    description: "Create a new terminal tab.",
    inputSchema: {
      type: "object",
      properties: {
        name: { type: "string" },
        shell: { type: "string" },
        cwd: { type: "string" },
      },
      required: ["name"],
      additionalProperties: false,
    },
  },
  {
    name: "rename_tab",
    title: "Rename Tab",
    description: "Rename an existing tab selected by active/title/id.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        name: { type: "string" },
      },
      required: ["name"],
      additionalProperties: false,
    },
  },
  {
    name: "open_layout",
    title: "Open Layout",
    description: "Open named tabs into a horizontal, vertical, or grid layout.",
    inputSchema: {
      type: "object",
      properties: {
        layout: { type: "string", enum: ["horizontal", "vertical", "grid"] },
        names: { type: "array", items: { type: "string" }, minItems: 2 },
        columns: { type: "number" },
        shell: { type: "string" },
        cwd: { type: "string" },
      },
      required: ["layout", "names"],
      additionalProperties: false,
    },
  },
  {
    name: "read",
    title: "Read Output",
    description: "Read recent terminal output from a tab.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        chars: { type: "number" },
        lines: { type: "number" },
        from: { type: "string", enum: ["top", "bottom"] },
      },
      additionalProperties: false,
    },
  },
  {
    name: "wait_for_text",
    title: "Wait For Text",
    description: "Poll a tab until matching text appears.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        text: { type: "string" },
        chars: { type: "number" },
        timeoutMs: { type: "number" },
        intervalMs: { type: "number" },
        ignoreCase: { type: "boolean" },
      },
      required: ["text"],
      additionalProperties: false,
    },
  },
  {
    name: "wait_for_quiet",
    title: "Wait For Quiet",
    description: "Poll a tab until output stays unchanged for a quiet window.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        chars: { type: "number" },
        quietMs: { type: "number" },
        timeoutMs: { type: "number" },
        intervalMs: { type: "number" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "send",
    title: "Send Payload",
    description:
      "Send text to a tab, optionally pressing Enter. Prefer payloadBase64 for multiline or quote-heavy content.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        text: { type: "string" },
        payloadBase64: { type: "string" },
        delivery: { enum: ["raw", "bracketed", "slow-typed"] },
        delayMs: { type: "number" },
        enter: { type: "boolean" },
      },
      oneOf: [{ required: ["text"] }, { required: ["payloadBase64"] }],
      additionalProperties: false,
    },
  },
  {
    name: "run",
    title: "Run Command",
    description:
      "Send command text, optionally press Enter, and wait for quiet output or matching text. Prefer payloadBase64 for multiline or quote-heavy content.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        text: { type: "string" },
        payloadBase64: { type: "string" },
        delivery: { enum: ["raw", "bracketed", "slow-typed"] },
        delayMs: { type: "number" },
        enter: { type: "boolean" },
        chars: { type: "number" },
        quietMs: { type: "number" },
        maxWaitMs: { type: "number" },
        intervalMs: { type: "number" },
        waitForText: { type: "string" },
        ignoreCase: { type: "boolean" },
      },
      oneOf: [{ required: ["text"] }, { required: ["payloadBase64"] }],
      additionalProperties: false,
    },
  },
  {
    name: "key",
    title: "Send Key",
    description: "Send a control key like ctrl+c to a tab.",
    inputSchema: {
      type: "object",
      properties: {
        tab: { type: "string" },
        key: { type: "string" },
      },
      required: ["key"],
      additionalProperties: false,
    },
  },
  {
    name: "presets",
    title: "List Presets",
    description: "List saved terminal presets.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: "preset_run",
    title: "Run Preset",
    description: "Run a saved preset into a selected tab.",
    inputSchema: {
      type: "object",
      properties: {
        label: { type: "string" },
        tab: { type: "string" },
      },
      required: ["label"],
      additionalProperties: false,
    },
  },
];

function writeMessage(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function writeResult(id, result) {
  writeMessage({ jsonrpc: "2.0", id, result });
}

function writeError(id, code, message) {
  writeMessage({
    jsonrpc: "2.0",
    id,
    error: { code, message },
  });
}

async function handleRequest(message) {
  const { id, method, params } = message;

  try {
    switch (method) {
      case "initialize":
        writeResult(id, {
          protocolVersion: PROTOCOL_VERSION,
          capabilities: {
            tools: {
              listChanged: false,
            },
          },
          serverInfo: SERVER_INFO,
        });
        return;
      case "ping":
        writeResult(id, {});
        return;
      case "notifications/initialized":
        return;
      case "tools/list":
        writeResult(id, { tools: TOOL_DEFINITIONS });
        return;
      case "tools/call": {
        const toolName = params?.name;
        const args = params?.arguments ?? {};
        if (typeof toolName !== "string") {
          writeError(id, -32602, "tools/call requires a tool name.");
          return;
        }

        const result = await callTool(toolName, args);
        writeResult(id, result);
        return;
      }
      case "resources/list":
        writeResult(id, { resources: [] });
        return;
      case "prompts/list":
        writeResult(id, { prompts: [] });
        return;
      default:
        if (id !== undefined && id !== null) {
          writeError(id, -32601, `Method not found: ${method}`);
        }
    }
  } catch (error) {
    const messageText = error instanceof Error ? error.message : String(error);
    writeResult(id, makeTextResult(messageText, { error: messageText }, true));
  }
}

let buffer = "";
let discardingOversizeLine = false;
const pendingRequests = new Set();
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buffer += chunk;
  while (true) {
    const newlineIndex = buffer.indexOf("\n");
    if (newlineIndex === -1) {
      if (Buffer.byteLength(buffer, "utf8") > MAX_MESSAGE_BYTES) {
        buffer = "";
        discardingOversizeLine = true;
        process.stderr.write("buttonscli-native-mcp: discarded oversized input line.\n");
      }
      break;
    }

    const line = buffer.slice(0, newlineIndex).trim();
    buffer = buffer.slice(newlineIndex + 1);
    if (discardingOversizeLine) {
      discardingOversizeLine = false;
      continue;
    }
    if (!line) {
      continue;
    }
    if (Buffer.byteLength(line, "utf8") > MAX_MESSAGE_BYTES) {
      process.stderr.write("buttonscli-native-mcp: discarded oversized input line.\n");
      continue;
    }

    let message;
    try {
      message = JSON.parse(line);
    } catch (error) {
      process.stderr.write(
        `buttonscli-native-mcp: invalid JSON input: ${String(error)}\n`,
      );
      continue;
    }

    const pending = handleRequest(message).finally(() => {
      pendingRequests.delete(pending);
    });
    pendingRequests.add(pending);
  }
});

process.stdin.on("end", () => {
  void Promise.allSettled([...pendingRequests]).then(() => {
    process.exitCode = 0;
  });
});
