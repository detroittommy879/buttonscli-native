/*
 * Pinned native copy of O/scripts/buttonsclictl.mjs at source revision
 * 032c9f21a17f17e48974f57259b1ad4a6506b858
 * (SHA-256 B44265F1212A8C7381E2713BEDF685D7EDD57BE8A6B6AF7AC8ACE286EF6DB76B).
 * Native changes are limited to exact ~/.buttonscli-native discovery,
 * descriptor validation, bracketed input mode, and bounded request deadlines. Keep O read-only.
 */
import { readFile, readdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const HELP_TEXT = `buttonsclictl - local control CLI for a running ButtonsCLI app

Usage:
  buttonsclictl status [--json]
  buttonsclictl tabs [--json]
  buttonsclictl create-tab --name <name> [--shell <shell>] [--cwd <cwd>] [--json]
  buttonsclictl rename-tab --name <name> [--tab <selector>] [--json]
  buttonsclictl open-layout --layout <horizontal|vertical|grid> --name <name> --name <name> [--name <name> ...] [--columns <n>] [--shell <shell>] [--cwd <cwd>] [--json]
  buttonsclictl read [--tab <selector>] [--chars <n> | --lines <n>] [--from top|bottom] [--json]
  buttonsclictl wait-for-text --text <text> [--tab <selector>] [--chars <n>] [--timeout-ms <n>] [--interval-ms <n>] [--ignore-case] [--json]
  buttonsclictl wait-for-quiet [--tab <selector>] [--chars <n>] [--quiet-ms <n>] [--timeout-ms <n>] [--interval-ms <n>] [--json]
  buttonsclictl send [--tab <selector>] [--delivery <raw|bracketed|slow-typed>] [--delay-ms <n>] [--enter] [--json] (--text <text> | --base64 <payload> | --file <path> | --stdin | <text...>)
  buttonsclictl run [--tab <selector>] [--delivery <raw|bracketed|slow-typed>] [--delay-ms <n>] [--chars <n>] [--quiet-ms <n>] [--timeout-ms <n>] [--interval-ms <n>] [--wait-for-text <text>] [--ignore-case] [--no-enter] [--json] (--text <text> | --base64 <payload> | --file <path> | --stdin | <text...>)
  buttonsclictl key <key> [--tab <selector>] [--json]
  buttonsclictl presets [--json]
  buttonsclictl preset-run --label <label> [--tab <selector>] [--json]

Selectors:
  active          currently focused tab
  <ptyId>         numeric PTY id
  <tabId>         internal tab id
  <title>         exact tab title, case-insensitive

Examples:
  buttonsclictl tabs
  buttonsclictl create-tab --name "logs-prod"
  buttonsclictl rename-tab --tab active --name "deploy-a"
  buttonsclictl open-layout --layout vertical --name "deploy-a" --name "deploy-b"
  buttonsclictl read --tab active --lines 120
  buttonsclictl wait-for-text --tab active --text "server ready"
  buttonsclictl wait-for-quiet --tab active --quiet-ms 1500
  buttonsclictl send --tab Deploy docker ps --enter
  buttonsclictl send --tab active "cd repo && pnpm test && pnpm lint" --enter
  buttonsclictl run --tab active "git status"
  buttonsclictl run --tab active --wait-for-text "Build complete" "pnpm build"
  buttonsclictl send --tab active --file ./bootstrap.ps1
  Get-Content ./script.sh -Raw | buttonsclictl send --tab active --stdin
  buttonsclictl send --tab active --base64 IyEvdXNyL2Jpbi9lbnYgYmFzaAo= --enter
  buttonsclictl send --tab active --delivery slow-typed --delay-ms 14 --file ./bootstrap.sh --enter
  buttonsclictl send --tab active --delivery bracketed --file ./long-paste.txt
  buttonsclictl key ctrl+c
  buttonsclictl presets
  buttonsclictl preset-run --label "prod ssh"
`;

export function defaultControlInfoPath(homedir = os.homedir()) {
  return path.join(homedir, ".buttonscli-native", "control");
}

export function parseArgs(argv) {
  const tokens = [...argv];
  const json = takeFlag(tokens, "--json");
  const command = tokens.shift();

  if (
    !command ||
    command === "help" ||
    command === "--help" ||
    command === "-h"
  ) {
    return { type: "help" };
  }

  switch (command) {
    case "status":
    case "tabs":
    case "presets":
      return { type: command, json };
    case "create-tab": {
      const name = takeOption(tokens, "--name");
      const shell = takeOption(tokens, "--shell");
      const cwd = takeOption(tokens, "--cwd");
      if (!name) {
        throw new Error("create-tab requires --name.");
      }
      assertNoExtraArgs(tokens);
      return { type: command, json, name, shell, cwd };
    }
    case "rename-tab": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const name = takeOption(tokens, "--name");
      if (!name) {
        throw new Error("rename-tab requires --name.");
      }
      assertNoExtraArgs(tokens);
      return { type: command, json, tab, name };
    }
    case "open-layout": {
      const layout = takeOption(tokens, "--layout");
      const names = takeRepeatedOptions(tokens, "--name");
      const columns = parseOptionalNumber(
        takeOption(tokens, "--columns"),
        "--columns",
      );
      const shell = takeOption(tokens, "--shell");
      const cwd = takeOption(tokens, "--cwd");
      if (!layout) {
        throw new Error("open-layout requires --layout.");
      }
      if (!isValidLayout(layout)) {
        throw new Error("--layout must be one of: horizontal, vertical, grid.");
      }
      if (names.length < 2) {
        throw new Error("open-layout requires at least two --name values.");
      }
      assertNoExtraArgs(tokens);
      return { type: command, json, layout, names, columns, shell, cwd };
    }
    case "read": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const chars = parseOptionalNumber(
        takeOption(tokens, "--chars"),
        "--chars",
      );
      const lines = parseOptionalNumber(
        takeOption(tokens, "--lines"),
        "--lines",
      );
      const from = takeOption(tokens, "--from") ?? "bottom";
      assertNoExtraArgs(tokens);
      return {
        type: "read",
        json,
        tab,
        chars,
        lines,
        from,
      };
    }
    case "wait-for-text": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const text = takeOption(tokens, "--text");
      const chars = parseOptionalNumber(
        takeOption(tokens, "--chars"),
        "--chars",
      );
      const timeoutMs =
        parseOptionalNumber(
          takeOption(tokens, "--timeout-ms"),
          "--timeout-ms",
        ) ?? 15_000;
      const intervalMs =
        parseOptionalNumber(
          takeOption(tokens, "--interval-ms"),
          "--interval-ms",
        ) ?? 250;
      const ignoreCase = takeFlag(tokens, "--ignore-case");
      if (!text) {
        throw new Error("wait-for-text requires --text.");
      }
      assertNoExtraArgs(tokens);
      return {
        type: "wait-for-text",
        json,
        tab,
        text,
        chars,
        timeoutMs,
        intervalMs,
        ignoreCase,
      };
    }
    case "wait-for-quiet": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const chars =
        parseOptionalNumber(takeOption(tokens, "--chars"), "--chars") ?? 12_000;
      const quietMs =
        parseOptionalNumber(takeOption(tokens, "--quiet-ms"), "--quiet-ms") ??
        1_200;
      const timeoutMs =
        parseOptionalNumber(
          takeOption(tokens, "--timeout-ms"),
          "--timeout-ms",
        ) ?? 15_000;
      const intervalMs =
        parseOptionalNumber(
          takeOption(tokens, "--interval-ms"),
          "--interval-ms",
        ) ?? 250;
      assertNoExtraArgs(tokens);
      return {
        type: "wait-for-quiet",
        json,
        tab,
        chars,
        quietMs,
        timeoutMs,
        intervalMs,
      };
    }
    case "send": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const delivery = parseDeliveryMode(takeOption(tokens, "--delivery"));
      const delayMs = parseOptionalNumber(
        takeOption(tokens, "--delay-ms"),
        "--delay-ms",
      );
      const text = takeOption(tokens, "--text");
      const base64 = takeOption(tokens, "--base64");
      const file = takeOption(tokens, "--file");
      const stdin = takeFlag(tokens, "--stdin");
      const enter = takeFlag(tokens, "--enter");
      if (tokens.some((token) => token.startsWith("--"))) {
        throw new Error(`Unexpected argument '${tokens[0]}'.`);
      }

      const positionalText = tokens.length > 0 ? tokens.join(" ") : undefined;
      const sourceCount = [
        text ? 1 : 0,
        base64 ? 1 : 0,
        positionalText ? 1 : 0,
        file ? 1 : 0,
        stdin ? 1 : 0,
      ].reduce((total, value) => total + value, 0);
      if (sourceCount !== 1) {
        throw new Error(
          "send requires exactly one payload source: positional text, --text, --base64, --file, or --stdin.",
        );
      }
      return {
        type: "send",
        json,
        tab,
        delivery,
        delayMs,
        text,
        base64,
        positionalText,
        file,
        stdin,
        enter,
      };
    }
    case "run": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const delivery = parseDeliveryMode(takeOption(tokens, "--delivery"));
      const delayMs = parseOptionalNumber(
        takeOption(tokens, "--delay-ms"),
        "--delay-ms",
      );
      const text = takeOption(tokens, "--text");
      const base64 = takeOption(tokens, "--base64");
      const file = takeOption(tokens, "--file");
      const stdin = takeFlag(tokens, "--stdin");
      const chars = parseOptionalNumber(
        takeOption(tokens, "--chars"),
        "--chars",
      );
      const quietMs =
        parseOptionalNumber(takeOption(tokens, "--quiet-ms"), "--quiet-ms") ??
        1_200;
      const timeoutMs =
        parseOptionalNumber(
          takeOption(tokens, "--timeout-ms"),
          "--timeout-ms",
        ) ?? 8_000;
      const intervalMs =
        parseOptionalNumber(
          takeOption(tokens, "--interval-ms"),
          "--interval-ms",
        ) ?? 250;
      const waitForText = takeOption(tokens, "--wait-for-text");
      const ignoreCase = takeFlag(tokens, "--ignore-case");
      const noEnter = takeFlag(tokens, "--no-enter");
      if (tokens.some((token) => token.startsWith("--"))) {
        throw new Error(`Unexpected argument '${tokens[0]}'.`);
      }

      const positionalText = tokens.length > 0 ? tokens.join(" ") : undefined;
      const sourceCount = [
        text ? 1 : 0,
        base64 ? 1 : 0,
        positionalText ? 1 : 0,
        file ? 1 : 0,
        stdin ? 1 : 0,
      ].reduce((total, value) => total + value, 0);
      if (sourceCount !== 1) {
        throw new Error(
          "run requires exactly one payload source: positional text, --text, --base64, --file, or --stdin.",
        );
      }

      return {
        type: "run",
        json,
        tab,
        delivery,
        delayMs,
        text,
        base64,
        positionalText,
        file,
        stdin,
        chars,
        quietMs,
        timeoutMs,
        intervalMs,
        waitForText,
        ignoreCase,
        enter: !noEnter,
      };
    }
    case "key": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const key = tokens.shift();
      if (!key) {
        throw new Error("key requires a key name like ctrl+c.");
      }
      assertNoExtraArgs(tokens);
      return { type: "key", json, tab, key };
    }
    case "preset-run": {
      const tab = takeOption(tokens, "--tab") ?? "active";
      const label = takeOption(tokens, "--label");
      if (!label) {
        throw new Error("preset-run requires --label.");
      }
      assertNoExtraArgs(tokens);
      return { type: "preset-run", json, tab, label };
    }
    default:
      throw new Error(
        `Unknown command '${command}'. Use --help to see available commands.`,
      );
  }
}

function takeFlag(tokens, flag) {
  const index = tokens.indexOf(flag);
  if (index === -1) {
    return false;
  }
  tokens.splice(index, 1);
  return true;
}

function takeOption(tokens, flag) {
  const index = tokens.indexOf(flag);
  if (index === -1) {
    return undefined;
  }
  const value = tokens[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value.`);
  }
  tokens.splice(index, 2);
  return value;
}

function parseOptionalNumber(rawValue, label) {
  if (rawValue === undefined) {
    return undefined;
  }
  const parsed = Number(rawValue);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${label} must be a positive number.`);
  }
  return parsed;
}

function parseDeliveryMode(rawValue) {
  if (rawValue === undefined) {
    return undefined;
  }

  if (rawValue === "raw" || rawValue === "bracketed" || rawValue === "slow-typed") {
    return rawValue;
  }

  throw new Error("--delivery must be one of: raw, bracketed, slow-typed.");
}

function takeRepeatedOptions(tokens, flag) {
  const values = [];

  while (true) {
    const index = tokens.indexOf(flag);
    if (index === -1) {
      break;
    }

    const value = tokens[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`${flag} requires a value.`);
    }

    values.push(value);
    tokens.splice(index, 2);
  }

  return values;
}

function isValidLayout(value) {
  return value === "horizontal" || value === "vertical" || value === "grid";
}

function assertNoExtraArgs(tokens) {
  if (tokens.length > 0) {
    throw new Error(`Unexpected argument '${tokens[0]}'.`);
  }
}

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

async function readStdinText() {
  return await new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

async function resolveSendText(parsed) {
  if (parsed.base64) {
    return {
      payloadBase64: parsed.base64,
    };
  }

  if (parsed.text) {
    return {
      text: parsed.text,
    };
  }

  if (parsed.positionalText) {
    return {
      text: parsed.positionalText,
    };
  }

  if (parsed.file) {
    return {
      text: await readFile(parsed.file, "utf8"),
    };
  }

  if (parsed.stdin) {
    return {
      text: await readStdinText(),
    };
  }

  throw new Error("No send payload source was provided.");
}

function buildReadQuery({ chars, lines, from }) {
  const query = new URLSearchParams();
  if (chars !== undefined) {
    query.set("chars", String(chars));
  }
  if (lines !== undefined) {
    query.set("lines", String(lines));
  }
  if (from) {
    query.set("from", from);
  }
  return query;
}

async function readTabSnapshot(connection, { tab, chars, lines, from }, timeoutMs) {
  const query = buildReadQuery({ chars, lines, from });
  return apiRequest(
    connection,
    "GET",
    `/v1/tabs/${encodeURIComponent(tab)}/read?${query.toString()}`,
    undefined,
    timeoutMs,
  );
}

function writeSnapshotText(text) {
  if (!text) {
    return;
  }
  process.stdout.write(text);
  if (!text.endsWith("\n")) {
    process.stdout.write("\n");
  }
}

function textMatches(text, needle, ignoreCase) {
  if (!ignoreCase) {
    return text.includes(needle);
  }
  return text.toLowerCase().includes(needle.toLowerCase());
}

async function waitForText(connection, parsed) {
  const startedAt = Date.now();
  const deadline = startedAt + parsed.timeoutMs;

  while (true) {
    const payload = await readTabSnapshot(connection, {
      tab: parsed.tab,
      chars: parsed.chars,
      from: "bottom",
    }, Math.max(1, Math.min(10_000, deadline - Date.now())));

    if (textMatches(payload.text || "", parsed.text, parsed.ignoreCase)) {
      return {
        ok: true,
        type: "wait-for-text",
        needle: parsed.text,
        ignoreCase: parsed.ignoreCase,
        elapsedMs: Date.now() - startedAt,
        tab: payload.tab,
        text: payload.text || "",
      };
    }

    if (Date.now() >= deadline) {
      throw new Error(
        `Timed out after ${parsed.timeoutMs}ms waiting for text '${parsed.text}' on tab '${parsed.tab}'.`,
      );
    }

    await sleep(Math.max(1, Math.min(parsed.intervalMs, deadline - Date.now())));
  }
}

async function waitForQuiet(connection, parsed) {
  const startedAt = Date.now();
  const deadline = startedAt + parsed.timeoutMs;
  let lastSignature = null;
  let stableSince = null;

  while (true) {
    const payload = await readTabSnapshot(connection, {
      tab: parsed.tab,
      chars: parsed.chars,
      from: "bottom",
    }, Math.max(1, Math.min(10_000, deadline - Date.now())));
    const now = Date.now();
    const signature = `${payload.tab.lastUpdatedAtMs}:${payload.text || ""}`;

    if (signature !== lastSignature) {
      lastSignature = signature;
      stableSince = now;
    }

    if (stableSince !== null && now - stableSince >= parsed.quietMs) {
      return {
        ok: true,
        type: "wait-for-quiet",
        elapsedMs: now - startedAt,
        quietMs: parsed.quietMs,
        tab: payload.tab,
        text: payload.text || "",
      };
    }

    if (now >= deadline) {
      throw new Error(
        `Timed out after ${parsed.timeoutMs}ms waiting for quiet on tab '${parsed.tab}'.`,
      );
    }

    await sleep(Math.max(1, Math.min(parsed.intervalMs, deadline - Date.now())));
  }
}

async function loadConnectionInfo() {
  const explicitPath = process.env.BUTTONSCLI_CONTROL_INFO_PATH;
  if (explicitPath) {
    return await readNativeConnectionInfo(explicitPath);
  }

  const controlDir = defaultControlInfoPath();
  const names = await readdir(controlDir).catch(() => []);
  const candidatePaths = names
    .filter((name) => name.toLowerCase().endsWith(".json"))
    .map((name) => path.join(controlDir, name));
  const active = [];
  for (const infoPath of candidatePaths) {
    const connection = await readNativeConnectionInfo(infoPath).catch(() => null);
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
  const raw = await readFile(absolutePath, "utf8");
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

function formatTimestamp(value) {
  if (!Number.isFinite(value)) {
    return null;
  }

  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) {
    return null;
  }

  return timestamp.toLocaleString();
}

function buildConnectionFailureMessage(connection, error) {
  const pieces = [
    `Failed to reach ButtonsCLI control API at ${connection.baseUrl}.`,
  ];
  const updatedAt = formatTimestamp(connection.updatedAtMs);

  if (connection.infoPath) {
    pieces.push(`Connection info: ${connection.infoPath}.`);
  }
  if (updatedAt) {
    pieces.push(`Recorded at: ${updatedAt}.`);
  }

  pieces.push(
    "The local connection file is probably stale or ButtonsCLI is not currently running. Start or restart ButtonsCLI so it rewrites the control API listener info.",
  );

  if (error?.message) {
    pieces.push(`Fetch error: ${error.message}.`);
  }

  return pieces.join(" ");
}

// Server-side run waits cap at 60 seconds and paced delivery at 30 seconds.
// Give those operations time to finish while bounding a stalled connection.
function requestTimeoutMs(pathname, body) {
  const delivery = body?.delivery === "slow-typed" ? 30_000 : 0;
  const wait = pathname.endsWith("/run") && pathname.startsWith("/v1/tabs/")
    ? Math.min(body?.maxWaitMs ?? 8_000, 60_000) : 0;
  return 10_000 + delivery + wait;
}

async function apiRequest(connection, method, pathname, body, timeoutMs) {
  let response;
  let text;
  try {
    response = await fetch(`${connection.baseUrl}${pathname}`, {
      method,
      headers: {
        Authorization: `Bearer ${connection.authToken}`,
        "Content-Type": "application/json",
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(Math.max(1, Math.ceil(timeoutMs ?? requestTimeoutMs(pathname, body)))),
    });
    text = await response.text();
  } catch (error) {
    if (error?.name === "TimeoutError" || error?.name === "AbortError") {
      const advice = method === "POST"
        ? " Delivery may have begun; check the target before repeating the request." : "";
      throw new Error(`Control API request timed out.${advice}`);
    }
    throw new Error(buildConnectionFailureMessage(connection, error));
  }
  let payload;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    throw new Error(`Control API returned invalid JSON (HTTP ${response.status}).`);
  }
  if (!response.ok) {
    throw new Error(payload?.error || `Control API returned ${response.status}.`);
  }
  return payload;
}

function renderJson(payload) {
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function pad(value, width) {
  return String(value).padEnd(width, " ");
}

function renderTabs(payload) {
  const header = [
    pad("A", 2),
    pad("PTY", 6),
    pad("TITLE", 24),
    pad("SHELL", 16),
    pad("STORED", 8),
    "TAB ID",
  ].join(" ");
  const rows = payload.tabs.map((tab) =>
    [
      pad(tab.isActive ? "*" : "", 2),
      pad(tab.ptyId, 6),
      pad(tab.title, 24),
      pad(tab.shell || "default", 16),
      pad(tab.storedOutputChars, 8),
      tab.tabId || "-",
    ].join(" "),
  );
  process.stdout.write(`${header}\n${rows.join("\n")}\n`);
}

function renderPresets(payload) {
  const header = [
    pad("KIND", 12),
    pad("ENTER", 7),
    pad("LABEL", 24),
    "COMMAND",
  ].join(" ");
  const rows = payload.presets.map((preset) =>
    [
      pad(preset.kind, 12),
      pad(preset.sendEnter ? "yes" : "no", 7),
      pad(preset.label, 24),
      preset.command,
    ].join(" "),
  );
  process.stdout.write(`${header}\n${rows.join("\n")}\n`);
}

function renderAction(payload) {
  process.stdout.write(`${payload.detail}\n`);
  if (Number.isFinite(payload.bytesSent)) {
    process.stdout.write(`bytes sent: ${payload.bytesSent}\n`);
  }
  if (payload.delivery) {
    process.stdout.write(`delivery: ${payload.delivery}\n`);
  }
  if (payload.tab) {
    process.stdout.write(
      `${payload.tab.title} (${payload.tab.shell || "default"}, PTY ${payload.tab.ptyId})\n`,
    );
  }
}

function renderLayout(payload) {
  process.stdout.write(`${payload.detail}\n`);
  renderTabs({ tabs: payload.tabs });
}

function renderRun(payload) {
  const reason = payload.completionReason;
  if (payload.timedOut || reason === "timeout") {
    process.stdout.write(
      `${payload.tab.title} timed out after ${payload.elapsedMs}ms. Returning the latest captured output.\n`,
    );
  } else if (reason === "matched-text" && payload.matchedText) {
    process.stdout.write(
      `Matched '${payload.matchedText}' on ${payload.tab.title} after ${payload.elapsedMs}ms.\n`,
    );
  } else {
    process.stdout.write(
      `${payload.tab.title} stayed quiet after ${payload.elapsedMs}ms total.\n`,
    );
  }

  if (Number.isFinite(payload.bytesSent)) {
    process.stdout.write(`bytes sent: ${payload.bytesSent}\n`);
  }
  if (payload.delivery) {
    process.stdout.write(`delivery: ${payload.delivery}\n`);
  }

  writeSnapshotText(payload.text || "");
}

async function run(argv) {
  const parsed = parseArgs(argv);
  if (parsed.type === "help") {
    process.stdout.write(HELP_TEXT);
    return 0;
  }

  const connection = await loadConnectionInfo();

  switch (parsed.type) {
    case "status": {
      const payload = await apiRequest(connection, "GET", "/v1/status");
      if (parsed.json) {
        renderJson(payload);
      } else {
        process.stdout.write(`baseUrl: ${payload.baseUrl}\n`);
        process.stdout.write(`infoPath: ${payload.infoPath}\n`);
        process.stdout.write(`connectedTabs: ${payload.connectedTabs}\n`);
      }
      return 0;
    }
    case "tabs": {
      const payload = await apiRequest(connection, "GET", "/v1/tabs");
      parsed.json ? renderJson(payload) : renderTabs(payload);
      return 0;
    }
    case "create-tab": {
      const payload = await apiRequest(connection, "POST", "/v1/tabs", {
        name: parsed.name,
        shell: parsed.shell,
        cwd: parsed.cwd,
      });
      parsed.json ? renderJson(payload) : renderAction(payload);
      return 0;
    }
    case "rename-tab": {
      const payload = await apiRequest(
        connection,
        "POST",
        `/v1/tabs/${encodeURIComponent(parsed.tab)}/rename`,
        {
          name: parsed.name,
        },
      );
      parsed.json ? renderJson(payload) : renderAction(payload);
      return 0;
    }
    case "open-layout": {
      const payload = await apiRequest(connection, "POST", "/v1/layout/open", {
        names: parsed.names,
        layout: parsed.layout,
        columns: parsed.columns,
        shell: parsed.shell,
        cwd: parsed.cwd,
      });
      parsed.json ? renderJson(payload) : renderLayout(payload);
      return 0;
    }
    case "read": {
      const payload = await readTabSnapshot(connection, parsed);
      if (parsed.json) {
        renderJson(payload);
      } else {
        writeSnapshotText(payload.text || "");
      }
      return 0;
    }
    case "wait-for-text": {
      const payload = await waitForText(connection, parsed);
      if (parsed.json) {
        renderJson(payload);
      } else {
        process.stdout.write(
          `Matched '${parsed.text}' on ${payload.tab.title} after ${payload.elapsedMs}ms.\n`,
        );
        writeSnapshotText(payload.text);
      }
      return 0;
    }
    case "wait-for-quiet": {
      const payload = await waitForQuiet(connection, parsed);
      if (parsed.json) {
        renderJson(payload);
      } else {
        process.stdout.write(
          `${payload.tab.title} stayed quiet for ${payload.quietMs}ms after ${payload.elapsedMs}ms total.\n`,
        );
        writeSnapshotText(payload.text);
      }
      return 0;
    }
    case "send": {
      const requestPayload = await resolveSendText(parsed);
      const payload = await apiRequest(
        connection,
        "POST",
        `/v1/tabs/${encodeURIComponent(parsed.tab)}/send`,
        {
          ...requestPayload,
          delivery: parsed.delivery,
          delayMs: parsed.delayMs,
          enter: parsed.enter,
        },
      );
      parsed.json
        ? renderJson(payload)
        : process.stdout.write(`${payload.detail} ${payload.tab.title}\n`);
      return 0;
    }
    case "run": {
      const requestPayload = await resolveSendText(parsed);
      const payload = await apiRequest(
        connection,
        "POST",
        `/v1/tabs/${encodeURIComponent(parsed.tab)}/run`,
        {
          ...requestPayload,
          delivery: parsed.delivery,
          delayMs: parsed.delayMs,
          enter: parsed.enter,
          chars: parsed.chars,
          quietMs: parsed.quietMs,
          maxWaitMs: parsed.timeoutMs,
          intervalMs: parsed.intervalMs,
          waitForText: parsed.waitForText,
          ignoreCase: parsed.ignoreCase,
        },
      );
      parsed.json ? renderJson(payload) : renderRun(payload);
      return 0;
    }
    case "key": {
      const payload = await apiRequest(
        connection,
        "POST",
        `/v1/tabs/${encodeURIComponent(parsed.tab)}/key`,
        {
          key: parsed.key,
        },
      );
      parsed.json
        ? renderJson(payload)
        : process.stdout.write(`${payload.detail} ${payload.tab.title}\n`);
      return 0;
    }
    case "presets": {
      const payload = await apiRequest(connection, "GET", "/v1/presets");
      parsed.json ? renderJson(payload) : renderPresets(payload);
      return 0;
    }
    case "preset-run": {
      const payload = await apiRequest(connection, "POST", "/v1/presets/run", {
        label: parsed.label,
        tab: parsed.tab,
      });
      parsed.json
        ? renderJson(payload)
        : process.stdout.write(`${payload.detail} ${payload.tab.title}\n`);
      return 0;
    }
    default:
      throw new Error(`Unhandled command type '${parsed.type}'.`);
  }
}

const entryUrl = process.argv[1] ? pathToFileURL(process.argv[1]).href : null;

if (entryUrl && import.meta.url === entryUrl) {
  run(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}

export { HELP_TEXT, run };
