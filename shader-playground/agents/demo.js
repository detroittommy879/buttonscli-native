// Scripted "video" of agent control. Nothing here talks to a shell or a network
// service: the agent, the commands and their output are all written below.
// Pane backgrounds are the playground's exported WGSL shaders (../data/).
const $ = id => document.getElementById(id);
const stage = $('stage'), log = $('agent-log'), workspace = $('workspace'), svg = $('arrows');
const NS = 'http://www.w3.org/2000/svg';
const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e; };
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;

// ---------- virtual clock: pause, speed and instant seeking ----------
const CANCEL = Symbol('cancel');
const clock = {vt: 0, logical: 0, waiters: [], skipping: false, playing: !reduced, speed: 1};
const channel = new MessageChannel();
let pumping = false, onscreen = true;
function earliest(limit = Infinity) {
  let best = -1;
  clock.waiters.forEach((w, i) => { if (w.at <= limit && (best < 0 || w.at < clock.waiters[best].at)) best = i; });
  return best < 0 ? null : clock.waiters.splice(best, 1)[0];
}
function pump() { if (!pumping && clock.skipping) { pumping = true; channel.port2.postMessage(0); } }
channel.port1.onmessage = () => {
  pumping = false;
  if (!clock.skipping) return;
  const w = earliest();
  if (!w) return;
  clock.vt = clock.logical = w.at; w.res(); pump();
};
function sleep(ms) {
  return new Promise((res, rej) => { clock.waiters.push({at: clock.logical + ms, res, rej}); pump(); });
}
function cancel() {
  const old = clock.waiters; clock.waiters = []; clock.vt = clock.logical = 0;
  old.forEach(w => w.rej(CANCEL));
}
const spawn = fn => fn().catch(e => { if (e !== CANCEL) console.error(e); });

// ---------- simulated terminals ----------
const INK = {green: '#55d9ad', red: '#ff7b8c', blue: '#71caff', yellow: '#ffe080', purple: '#e9a0ef'};
const ACCENTS = ['#9aa0b5', '#489cff', '#ff9c38', '#23cc78', '#ed489e'];
const NAMES = ['term1', 'server', 'tests', 'git', 'logs'];
const panes = [], P = {};
function runs(row, text, color) {
  for (const part of text.split(/( {2,})/)) {
    if (!part) continue;
    if (!part.trim()) { row.append(part); continue; }
    const span = el('span', 'text-run', part);
    if (color) span.style.setProperty('--ink', INK[color] || color);
    row.append(span);
  }
}
function trim(p) {
  const t = p.terminal;
  if (!t.clientHeight) return;
  while (t.children.length > 1 && t.scrollHeight > t.clientHeight + 1) t.firstChild.remove();
}
function line(p, ...segments) {
  const row = el('div', 'term-line');
  for (const s of segments) Array.isArray(s) ? runs(row, s[0], s[1]) : runs(row, s);
  p.terminal.append(row); trim(p);
  return row;
}
function prompt(p) {
  const row = line(p, ['~/shop ❯', 'blue']);
  row.append(' ', el('span', 'text-run cursor', '▌'));
  p.promptRow = row;
}
async function typeCommand(p, text) {
  const row = p.promptRow || line(p);
  row.querySelector('.cursor')?.remove();
  const span = el('span', 'text-run'), cursor = el('span', 'text-run cursor', '▌');
  row.append(span, cursor);
  p.tab.classList.add('busy');
  if (clock.skipping) { span.textContent = text; await sleep(text.length * 34); }
  else for (const ch of text) { span.textContent += ch; await sleep(34); }
  await sleep(220);
  cursor.remove(); p.promptRow = null; p.tab.classList.remove('busy');
}
async function output(p, lines, gap = 170) {
  for (const l of lines) { await sleep(gap); line(p, ...(Array.isArray(l) && Array.isArray(l[0]) ? l : [l])); }
}

// ---------- simulated agent ----------
const scroll = () => { log.scrollTop = log.scrollHeight; };
function user(text, pasted) {
  const m = el('div', 'msg user', text);
  if (pasted) m.append(el('span', 'pasted', pasted));
  log.append(m); scroll();
}
async function say(text) {
  const m = el('div', 'msg say'); log.append(m);
  if (clock.skipping) { m.textContent = text; scroll(); return sleep(text.split(' ').length * 55); }
  for (const word of text.split(' ')) { m.textContent += (m.textContent ? ' ' : '') + word; scroll(); await sleep(55); }
}
function note(text) { log.append(el('div', 'msg note', text)); scroll(); }
function toolCard(name, args, kind = 'act') {
  const card = el('div', `tool ${kind}`), head = el('div', 'tool-head'), state = el('span', 'tool-state');
  head.append(el('span', 'mcp', 'MCP'), el('b', '', name), state);
  state.append(el('span', 'spin'), 'running');
  card.append(head);
  if (args) card.append(el('div', 'tool-args', args));
  log.append(card); scroll();
  return {
    el: card,
    done(text, result) {
      card.classList.add('done'); state.textContent = `✓ ${text}`;
      if (result) {
        const box = el('div', 'tool-result');
        for (const [t, bad] of result) box.append(el('span', bad ? 'bad' : '', t));
        card.append(box);
      }
      scroll();
    },
  };
}

// ---------- arrows ----------
function arrowPoints(from, to) {
  const s = stage.getBoundingClientRect(), view = $('agent').getBoundingClientRect();
  const rel = r => ({l: r.left - s.left, t: r.top - s.top, r: r.right - s.left, b: r.bottom - s.top, cx: (r.left + r.right) / 2 - s.left, cy: (r.top + r.bottom) / 2 - s.top, w: r.width});
  const A = rel(from.getBoundingClientRect()), B = rel(to.getBoundingClientRect()), V = rel(view);
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  if (A.r <= B.l + 4) { // side by side
    const p0 = [A.r - 2, clamp(A.cy, V.t + 30, V.b - 30)], p1 = [B.l + Math.min(34, B.w * 0.25), B.cy];
    const d = Math.max(50, (p1[0] - p0[0]) * 0.55);
    return [p0, [p0[0] + d, p0[1]], [p1[0] - d, p1[1]], p1];
  }
  const p0 = [clamp(B.cx, A.l + 24, A.r - 24), Math.min(A.b, V.b) - 2], p1 = [B.cx, B.t + 16]; // stacked
  const d = Math.max(34, (p1[1] - p0[1]) * 0.5);
  return [p0, [p0[0], p0[1] + d], [p1[0], p1[1] - d], p1];
}
async function arrow(from, to, {kind = 'send', label = '', reverse = false, hold = 1500} = {}) {
  if (clock.skipping) return sleep(650);
  let pts = arrowPoints(from, to);
  if (reverse) pts = pts.reverse();
  const [a, b, c, d] = pts, f = n => n.toFixed(1);
  const path = `M${f(a[0])} ${f(a[1])} C${f(b[0])} ${f(b[1])} ${f(c[0])} ${f(c[1])} ${f(d[0])} ${f(d[1])}`;
  const g = document.createElementNS(NS, 'g'); g.setAttribute('class', `arrow ${kind}`);
  const make = (tag, cls, attrs) => { const n = document.createElementNS(NS, tag); n.setAttribute('class', cls); for (const k in attrs) n.setAttribute(k, attrs[k]); g.append(n); return n; };
  make('path', 'under', {d: path, pathLength: 1});
  make('path', 'line', {d: path, pathLength: 1});
  make('path', 'flow', {d: path});
  const angle = Math.atan2(d[1] - c[1], d[0] - c[0]) * 180 / Math.PI;
  make('polygon', 'head', {points: '3,0 -11,-7 -11,7', transform: `translate(${f(d[0])} ${f(d[1])}) rotate(${f(angle)})`});
  make('circle', 'packet', {r: 4.5}).style.offsetPath = `path("${path}")`;
  if (label) {
    const mid = i => (a[i] + 3 * b[i] + 3 * c[i] + d[i]) / 8;
    const group = make('g', 'label', {transform: `translate(${f(mid(0))} ${f(mid(1))})`});
    const rect = document.createElementNS(NS, 'rect'), text = document.createElementNS(NS, 'text');
    text.textContent = label; group.append(rect, text);
    svg.append(g);
    const w = text.getComputedTextLength() + 16;
    for (const [k, v] of Object.entries({x: -w / 2, y: -10, width: w, height: 20, rx: 10})) rect.setAttribute(k, v);
  }
  svg.append(g);
  spawn(async () => { await sleep(650 + hold); g.classList.add('out'); await sleep(450); g.remove(); });
  await sleep(650);
}
function hit(p, reading = false, ms = 1500) {
  if (clock.skipping) return;
  p.element.classList.add('hit'); p.element.classList.toggle('reading', reading);
  spawn(async () => { await sleep(ms); p.element.classList.remove('hit', 'reading'); });
}
// One tool call: card, arrow out, the pane reacts, optional arrow back, result.
async function call(name, args, target, {kind = 'act', work, back, done = 'done', result, to} = {}) {
  const card = toolCard(name, args, kind);
  await sleep(380);
  if (kind === 'read') hit(target, true, 2300);
  else { await arrow(card.el, to || target.element, {label: name, kind: kind === 'layout' ? 'copy' : 'send'}); if (!to) hit(target); }
  if (work) await work();
  if (kind === 'read' || back) await arrow(card.el, target.element, {label: back || name, kind: 'read', reverse: true});
  card.done(done, result);
  await sleep(420);
}

// ---------- the story ----------
const AGENTS = ['Claude Code', 'Codex', 'Antigravity', 'any MCP client'];
let loops = 0;
const logLines = [
  'GET /cart 200 12ms', 'GET /api/products 200 31ms', 'POST /api/cart 201 18ms', 'GET /checkout 200 22ms', 'POST /api/discount 200 9ms',
  'GET /api/cart 200 7ms', 'GET /checkout 200 19ms', 'POST /api/checkout 200 44ms', 'GET /orders/1042 200 15ms',
];
let logIndex = 0;
async function tail(count, gap) {
  for (let i = 0; i < count; i++) { await sleep(gap); const s = 241 + logIndex * 3, two = n => String(n).padStart(2, '0');
    line(P.logs, [`12:${two(Math.floor(s / 60))}:${two(s % 60)}`, 'purple'], ` ${logLines[logIndex++ % logLines.length]}`);
  }
}
const devServer = [[['> shop dev', null]], [['> vite', null]], '', [['  VITE ', 'green'], ['ready in 412 ms', null]], [['  ➜  Local: ', null], ['http://localhost:5173/', 'blue']]];

const scenes = [
  {title: 'Connect', text: 'Click Agent Inst. in the status bar and paste what it copies into your agent. It now has 14 terminal tools.', async run() {
    const inst = $('agent-inst');
    await sleep(1100);
    inst.classList.add('pulse'); await sleep(800);
    inst.classList.add('press'); await sleep(160); inst.classList.remove('press');
    toast(inst, 'Instructions copied'); await sleep(1000);
    await arrow($('agent'), inst, {kind: 'copy', label: 'paste', reverse: true});
    inst.classList.remove('pulse'); $('toast').hidden = true;
    user('Use ButtonsCLI for anything you run in a terminal.', '📋 ButtonsCLI agent instructions');
    await sleep(1000);
    note('⚡ MCP server buttonscli-native connected · 14 tools');
    $('agent-dot').classList.add('on'); $('agent-status').textContent = 'buttonscli-native · 14 tools';
    await sleep(800);
    await call('status', '', P.term1, {kind: 'read', back: 'running · 1 tab', done: 'running · 1 tab'});
    $('control-state').textContent = 'control: 127.0.0.1 · agent connected'; $('control-state').classList.add('on');
    await sleep(900);
  }},
  {title: 'Lay out the work', text: 'One call opens a named tab for each job and tiles them. Every pane keeps its own look, so you can tell them apart at a glance.', async run() {
    user('The checkout test is failing. Fix it, and keep the dev server and logs up so I can watch.');
    await sleep(900);
    await say('I’ll give each job its own tab so you can follow along.');
    await call('open_layout', 'layout: "grid"\nnames: server, tests, git, logs', null, {kind: 'layout', to: $('tabs'), done: '4 tabs, 2×2 grid', async work() {
      workspace.classList.add('grid'); P.term1.element.hidden = true; P.term1.tab.classList.add('idle');
      for (const p of panes.slice(1)) { p.tab.hidden = false; p.element.hidden = false; p.dirty = true; prompt(p); await sleep(300); }
      await sleep(500);
    }});
    await sleep(700);
  }},
  {title: 'Run in every pane', text: 'Each command is typed into a real terminal tab. Long-running jobs stay up in their pane instead of hiding inside a tool call.', async run() {
    await Promise.all([
      call('run', 'tab: "server"\nnpm run dev', P.server, {done: 'quiet after 1.2 s', work: async () => { await typeCommand(P.server, 'npm run dev'); await output(P.server, devServer); }}),
      (async () => { await sleep(1500); await call('run', 'tab: "logs"\ntail -f logs/app.log', P.logs, {done: 'streaming', work: async () => { await typeCommand(P.logs, 'tail -f logs/app.log'); await tail(3, 260); }}); })(),
    ]);
    await Promise.all([tail(5, 700), call('run', 'tab: "tests"\nnpm test', P.tests, {done: 'quiet after 3.1 s', work: async () => {
      await typeCommand(P.tests, 'npm test');
      await output(P.tests, [[[' ✓ cart.test.ts (18)', 'green']], [[' ✓ catalog.test.ts (16)', 'green']]], 420);
      P.failRows = [];
      for (const l of [[[' ✗ checkout.test.ts (8)', 'red']], [['   applies discount code', 'red']], [['   expected 80.00, got 100.00', null]]]) { await sleep(260); P.failRows.push(line(P.tests, ...l)); }
      await output(P.tests, [[[' Tests: ', null], ['1 failed', 'red'], [', 41 passed', null]]], 380);
      prompt(P.tests);
    }})]);
    await sleep(600);
  }},
  {title: 'Read what happened', text: 'The agent reads the pane back to decide what to do next. You are looking at the same lines it is.', async run() {
    await Promise.all([tail(7, 900), (async () => {
      await call('read', 'tab: "tests", lines: 40', P.tests, {kind: 'read', back: '40 lines', done: '40 lines',
        result: [['✗ checkout.test.ts › applies discount code\n', true], ['expected 80.00, got 100.00', false]],
        work: async () => { for (const r of P.failRows || []) r.classList.add('flag'); await sleep(500); }});
      await say('The discount is subtracted from the wrong total. Fixing src/cart.ts.');
      const diff = el('div', 'diff'); diff.append(el('b', '', 'src/cart.ts'), el('span', 'del', '- return subtotal;'), el('span', 'add', '+ return subtotal - discount;'));
      log.append(diff); scroll();
      await sleep(1700);
    })()]);
  }},
  {title: 'Fix, rerun, restart', text: 'Rerun the tests, press Ctrl+C in the server tab, start it again. You could do any of these yourself: click a pane and type.', async run() {
    await Promise.all([tail(4, 800), call('run', 'tab: "tests"\nnpm test', P.tests, {done: 'quiet after 2.8 s', work: async () => {
      for (const r of P.failRows || []) r.classList.remove('flag');
      await typeCommand(P.tests, 'npm test');
      await output(P.tests, [[[' ✓ cart.test.ts (18)', 'green']], [[' ✓ catalog.test.ts (16)', 'green']], [[' ✓ checkout.test.ts (8)', 'green']], [[' Tests: ', null], ['42 passed', 'green']]], 380);
      prompt(P.tests);
    }})]);
    await call('key', 'tab: "server", key: "ctrl+c"', P.server, {done: 'sent', work: async () => { line(P.server, ['^C', 'yellow']); prompt(P.server); await sleep(500); }});
    await Promise.all([
      call('run', 'tab: "server"\nnpm run dev', P.server, {done: 'quiet after 1.1 s', work: async () => { await typeCommand(P.server, 'npm run dev'); await output(P.server, devServer.slice(2)); }}),
      (async () => { await sleep(1300); await call('run', 'tab: "git"\ngit diff --stat', P.git, {done: 'quiet after 0.4 s', work: async () => {
        await typeCommand(P.git, 'git diff --stat');
        await output(P.git, [[[' src/cart.ts | 2 ', null], ['+', 'green'], ['-', 'red']], ' 1 file changed, 1 insertion(+), 1 deletion(-)']);
        prompt(P.git);
      }}); })(),
      tail(4, 750),
    ]);
  }},
  {title: 'You saw every step', text: 'Nothing ran out of sight. Scroll any pane, copy from it, or take over the keyboard.', async run() {
    await say('Fixed. All 42 tests pass and the dev server is back on localhost:5173.');
    panes.slice(1).forEach(p => hit(p, false, 2600));
    await Promise.all([tail(3, 800), sleep(2800)]);
    if (!clock.skipping) $('endcard').hidden = false;
    await sleep(7000);
    $('endcard').hidden = true;
  }},
];
function toast(anchor, text) {
  if (clock.skipping) return;
  const s = stage.getBoundingClientRect(), r = anchor.getBoundingClientRect(), t = $('toast');
  t.textContent = text; t.hidden = false;
  t.style.left = `${Math.max(90, r.left + r.width / 2 - s.left)}px`; t.style.top = `${r.top - s.top}px`;
}

function reset() {
  log.replaceChildren(); svg.replaceChildren(); logIndex = 0; P.failRows = null;
  $('endcard').hidden = true; $('toast').hidden = true; $('chapter').hidden = true;
  $('agent-inst').className = ''; $('agent-dot').classList.remove('on');
  $('agent-status').textContent = 'No terminal tools connected';
  $('control-state').textContent = 'control: 127.0.0.1 · waiting'; $('control-state').classList.remove('on');
  $('agent-name').textContent = AGENTS[loops % AGENTS.length];
  workspace.classList.remove('grid');
  panes.forEach((p, i) => {
    p.terminal.replaceChildren(); p.promptRow = null; p.element.classList.remove('hit', 'reading');
    p.element.hidden = p.tab.hidden = i > 0; p.tab.classList.remove('idle', 'busy'); p.dirty = true;
  });
  line(P.term1, 'ButtonsCLI Native'); prompt(P.term1);
}
const starts = [];
let total = 1, current = -1;
function setScene(i) {
  current = i;
  $('scene-no').textContent = `${i + 1}/${scenes.length}`;
  $('scene-title').textContent = scenes[i].title; $('scene-text').textContent = scenes[i].text;
  [...$('chips').children].forEach((li, n) => li.firstChild.setAttribute('aria-current', String(n === i)));
}
async function play(target = 0, measure = false) {
  cancel(); reset();
  clock.skipping = measure || target > 0;
  try {
    for (let i = 0; i < scenes.length; i++) {
      if (!measure && i === target) clock.skipping = false;
      if (measure) starts[i] = clock.logical;
      setScene(i);
      await chapter(i);
      await scenes[i].run();
      await sleep(900);
    }
    if (measure) { total = clock.logical; clock.skipping = false; return; }
    loops++; shuffle(); play(0);
  } catch (e) { if (e !== CANCEL) throw e; }
}
// A short title card between scenes, so each change of subject gets a beat.
async function chapter(i) {
  const card = $('chapter');
  if (!clock.skipping && i > 0) { card.textContent = `${i + 1} · ${scenes[i].title}`; card.hidden = false; }
  await sleep(i > 0 ? 1500 : 300);
  card.hidden = true;
}
function setPlaying(on) {
  clock.playing = on; stage.classList.toggle('paused', !on);
  $('play').textContent = on ? '❚❚ Pause' : '▶ Play'; $('play').setAttribute('aria-label', on ? 'Pause' : 'Play');
}

// ---------- shaders (same WGSL and uniform layout as the playground) ----------
let library, device, template, helpers, format, lastFrame = 0, nextFrame = 0, compileQueue = Promise.resolve();
const pipelineCache = new Map(), FPS = 30;
const rgb = (hex = '#080b16') => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255);
const stripComments = code => code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
function source(shader) {
  const code = stripComments(shader.expression);
  const body = /\breturn\b/.test(code) ? shader.expression : `return ${shader.expression.replace(/;\s*$/, '')}\n;`;
  const fn = `fn custom_pattern(p: vec2<f32>, t: f32, knobs: vec4<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>, c4: vec3<f32>, c5: vec3<f32>, strength: f32, bg: vec3<f32>) -> vec3<f32> {\n${shader.variables.map((v, i) => `let ${v.name} = knobs[${i}];`).join('\n')}\n${body}\n}`;
  let result = template.replace('let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);', `let color = clamp(custom_pattern(${shader.y_up ? 'vec2<f32>(p.x, -p.y)' : 'p'}, t, u.knobs, u.first.rgb, u.second.rgb, u.third.rgb, u.fourth.rgb, u.fifth.rgb, u.settings.y, u.base.rgb), vec3<f32>(0.0), vec3<f32>(1.0));\n glow = 1.0; vivid = vec3<f32>(-1.0);`);
  if (/\bstrength\b/.test(code)) result = result.replace('clamp(glow * u.settings.y, 0.0, 1.0)', 'clamp(glow, 0.0, 1.0)');
  return `${helpers}\n${result}\n${fn}`;
}
function pipeline(index) {
  if (pipelineCache.has(index)) return pipelineCache.get(index);
  const pending = compileQueue.then(async () => {
    device.pushErrorScope('validation');
    try {
      const module = device.createShaderModule({code: source(library.shaders[index])});
      const info = await module.getCompilationInfo();
      const errors = info.messages.filter(m => m.type === 'error');
      if (errors.length) throw new Error(errors[0].message);
      return await device.createRenderPipelineAsync({layout: 'auto', vertex: {module, entryPoint: 'vertex'}, fragment: {module, entryPoint: 'fragment', targets: [{format}]}, primitive: {topology: 'triangle-list'}});
    } finally {
      const error = await device.popErrorScope();
      if (error) console.warn(error.message);
    }
  });
  compileQueue = pending.catch(() => {});
  pipelineCache.set(index, pending);
  return pending;
}
async function applyShader(p, index, attempt = 0) {
  const revision = ++p.revision, shader = library.shaders[index];
  p.shader = shader; p.time = Math.random() * 20;
  p.terminal.style.setProperty('--terminal-fg', shader.foreground);
  p.element.style.setProperty('--terminal-fg', shader.foreground);
  p.element.style.setProperty('--terminal-bg', shader.background);
  p.name.textContent = shader.name;
  shader.colors.slice(0, 3).forEach((c, i) => p.element.style.setProperty(`--f${i + 1}`, c));
  if (!device) return;
  try {
    const ready = await pipeline(index);
    if (revision !== p.revision) return;
    p.pipeline = ready;
    p.bind = device.createBindGroup({layout: ready.getBindGroupLayout(0), entries: [{binding: 0, resource: {buffer: p.buffer}}]});
    p.dirty = true;
  } catch (error) { // a shader this GPU rejects: quietly try another
    if (revision === p.revision && attempt < 4) applyShader(p, Math.floor(Math.random() * library.shaders.length), attempt + 1);
  }
}
function shuffle() {
  if (!library) return;
  const picks = new Set();
  while (picks.size < panes.length) picks.add(Math.floor(Math.random() * library.shaders.length));
  [...picks].forEach((index, i) => applyShader(panes[i], index));
  // The window's own colors follow one of the chosen themes, like a bundle's app theme.
  const chrome = [...picks].map(i => library.shaders[i].chrome).find(c => c && Object.keys(c).length);
  if (chrome) for (const [key, color] of Object.entries(chrome)) if (/^#[0-9a-f]{6}$/i.test(color)) $('app').style.setProperty(`--${key}`, color);
}
function draw(now) {
  const interval = 1000 / FPS;
  nextFrame = Math.max(nextFrame + interval, now);
  const dt = Math.min((now - lastFrame) / 1000, 0.25);
  lastFrame = now;
  if (!device) return;
  for (const p of panes) {
    if (p.element.hidden || !p.pipeline) continue;
    const ratio = Math.min(devicePixelRatio, 1.5);
    const width = Math.max(1, Math.round(p.element.clientWidth * ratio)), height = Math.max(1, Math.round(p.element.clientHeight * ratio));
    if (p.canvas.width !== width || p.canvas.height !== height) { p.canvas.width = width; p.canvas.height = height; p.dirty = true; }
    if (clock.playing && p.shader.speed > 0) { p.time += dt * p.shader.speed; p.dirty = true; }
    if (!p.dirty) continue;
    const s = p.shader, data = new Float32Array(36);
    data.set([0, 0, width, height, p.time, s.strength, s.scale, 0]);
    data.set([...rgb(s.background), 1], 8);
    for (let i = 0; i < 3; i++) data.set([...rgb(s.colors[i]), 1], 12 + i * 4);
    data.set(s.variables.slice(0, 4).map(v => v.value), 24);
    data.set([...rgb(s.colors[3]), 1], 28); data.set([...rgb(s.colors[4]), 1], 32);
    device.queue.writeBuffer(p.buffer, 0, data);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({colorAttachments: [{view: p.context.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store', clearValue: {r: 0, g: 0, b: 0, a: 1}}]});
    pass.setPipeline(p.pipeline); pass.setBindGroup(0, p.bind); pass.draw(3); pass.end();
    device.queue.submit([encoder.finish()]); p.dirty = false;
  }
}

// ---------- one frame loop for the clock, the progress chips and the GPU ----------
let lastTick = 0;
function frame(now) {
  requestAnimationFrame(frame);
  const dt = Math.min(now - lastTick, 100); lastTick = now;
  if (document.hidden || !onscreen) return;
  if (clock.playing && !clock.skipping) {
    clock.vt += dt * clock.speed;
    for (let w; (w = earliest(clock.vt));) { clock.logical = w.at; w.res(); }
  }
  if (starts.length && current >= 0) [...$('chips').children].forEach((li, i) => {
    const end = starts[i + 1] ?? total;
    li.firstChild.style.setProperty('--p', Math.max(0, Math.min(1, (clock.vt - starts[i]) / (end - starts[i]))).toFixed(3));
  });
  if (now + 0.5 >= nextFrame) draw(now);
}

async function start() {
  NAMES.forEach((name, i) => {
    const element = el('section', 'pane'); element.style.setProperty('--pane', ACCENTS[i]);
    const canvas = el('canvas'), terminal = el('div', 'terminal'), shaderName = el('span', 'pane-name');
    element.append(canvas, terminal, el('span', 'pane-tag', name), shaderName); workspace.append(element);
    const tab = el('span', 'tab', name); tab.style.setProperty('--pane', ACCENTS[i]); $('tabs').append(tab);
    panes.push(P[name] = {element, canvas, terminal, name: shaderName, tab, revision: 0, dirty: true, time: 0});
  });
  $('tabs').append(panes[0].tab); // term1 stays as a background tab after the layout opens
  scenes.forEach((scene, i) => {
    const li = el('li'), button = el('button'); button.type = 'button'; button.append(el('span', '', scene.title));
    button.title = `Jump to: ${scene.title}`; button.onclick = () => { setPlaying(true); play(i); };
    li.append(button); $('chips').append(li);
  });
  await play(0, true); // dry run: measures each scene so the chips can show progress
  reset(); setScene(0);
  try {
    const responses = await Promise.all(['../data/library.json?v=20261113-5', '../data/shader_helpers.wgsl', '../data/shader_background.wgsl'].map(url => fetch(url).then(r => { if (!r.ok) throw new Error(url); return r; })));
    [library, helpers, template] = await Promise.all([responses[0].json(), responses[1].text(), responses[2].text()]);
    const adapter = await navigator.gpu?.requestAdapter(); device = await adapter?.requestDevice();
  } catch (error) { console.warn(error); }
  if (device) {
    format = navigator.gpu.getPreferredCanvasFormat();
    device.lost.then(() => { device = null; });
    for (const p of panes) {
      p.context = p.canvas.getContext('webgpu'); p.context.configure({device, format, alphaMode: 'opaque'});
      p.buffer = device.createBuffer({size: 144, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST});
    }
  } else {
    for (const p of panes) p.canvas.replaceWith(el('div', 'fallback'));
    $('gpu-note').textContent = 'A simulation: nothing here runs a command. This browser has no WebGPU, so the panes show moving gradients in each shader’s colors instead of the shaders themselves.';
  }
  shuffle();
  $('play').onclick = () => setPlaying(!clock.playing);
  $('restart').onclick = $('again').onclick = () => { setPlaying(true); play(0); };
  $('shuffle').onclick = shuffle;
  $('speed').onchange = () => { clock.speed = Number($('speed').value); stage.style.setProperty('--speed', clock.speed); };
  $('theme').onclick = () => {
    const next = document.documentElement.dataset.theme === 'light' ? 'dark' : 'light';
    document.documentElement.dataset.theme = next; themeLabel();
    try { localStorage.setItem('buttonscli-agents-theme', next); } catch {}
  };
  new IntersectionObserver(entries => { onscreen = entries[0].isIntersecting; }).observe(stage);
  addEventListener('resize', () => svg.replaceChildren());
  setPlaying(clock.playing);
  stage.classList.remove('booting');
  play(0);
  requestAnimationFrame(frame);
}
function themeLabel() { $('theme').textContent = document.documentElement.dataset.theme === 'light' ? '☾ Dark' : '☀ Light'; }
themeLabel();
start().catch(error => { stage.classList.remove('booting'); console.error(error); });
