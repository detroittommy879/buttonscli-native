// Local OpenAI-compatible fixture. No real credentials, external calls, or prompt logs.
import http from 'node:http';
import { pathToFileURL } from 'node:url';

export const models = ['fake-ok', 'fake-slow', 'fake-error-once', 'fake-quota', 'fake-malformed', 'fake-disconnect'];

export function createFakeProvider({ delayMs = 25 } = {}) {
  const counts = new Map();
  const stats = { requests: 0, contextRequests: 0, disconnected: 0 };
  const server = http.createServer(async (req, res) => {
    const send = (status, data) => {
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(data));
    };
    if (req.method === 'GET' && req.url === '/v1/models') {
      return send(200, { data: models.map(id => ({ id, object: 'model' })) });
    }
    if (req.method === 'GET' && req.url === '/status') return send(200, stats);
    if (req.method !== 'POST' || req.url !== '/v1/chat/completions') return send(404, { error: 'Unknown fixture route' });
    let size = 0;
    const buffers = [];
    try {
      for await (const chunk of req) {
        size += chunk.length;
        if (size > 1024 * 1024) { send(413, { error: 'Request too large' }); req.destroy(); return; }
        buffers.push(chunk);
      }
    } catch { return; }
    let body;
    try { body = JSON.parse(Buffer.concat(buffers).toString('utf8')); }
    catch { return send(400, { error: 'Invalid JSON' }); }
    if (!models.includes(body.model) || !Array.isArray(body.messages)) return send(400, { error: 'Unknown model or missing messages' });
    stats.requests++;
    const count = (counts.get(body.model) ?? 0) + 1;
    counts.set(body.model, count);
    const context = body.messages.some(message => message.role === 'user' && /selected terminal context is read-only|terminal output snapshot|terminal snapshot/i.test(message.content ?? ''));
    if (context) stats.contextRequests++;
    if (body.model === 'fake-quota') return send(429, { error: { message: 'Fixture quota rejection' } });
    if (body.model === 'fake-error-once' && count === 1) return send(503, { error: { message: 'Fixture failure; retry succeeds' } });
    const answer = `<answer>Local test response 🦇. This is a deterministic fixture, not an AI answer. Terminal context detected: ${context ? 'yes' : 'no'}. Review the harmless echo suggestion before inserting it.</answer><commands>${JSON.stringify([{ label: 'Local echo test', command: 'echo ButtonsCLI-local-test', description: 'Print a test marker in the reviewed target only.', sendEnter: false }])}</commands>`;
    if (!body.stream) return send(200, { model: body.model, choices: [{ message: { role: 'assistant', content: answer } }] });
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store' });
    res.flushHeaders();
    if (body.model === 'fake-malformed') { res.end('data: {invalid json}\n\n'); return; }
    const chars = Array.from(answer);
    let cursor = 0;
    const timer = setInterval(() => {
      if (body.model === 'fake-disconnect' && cursor > 0) { res.destroy(); return; }
      const content = chars.slice(cursor, cursor + 12).join('');
      cursor += 12;
      res.write(`data: ${JSON.stringify({ model: body.model, choices: [{ delta: { content } }] })}\n\n`);
      if (cursor >= chars.length) {
        clearInterval(timer);
        res.end('data: [DONE]\n\n');
      }
    }, body.model === 'fake-slow' ? 250 : delayMs);
    res.on('close', () => { clearInterval(timer); if (!res.writableFinished) stats.disconnected++; });
  });
  server.requestTimeout = 5000;
  server.headersTimeout = 5000;
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.argv[2] ?? 0);
  if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Port must be 0–65535');
  const server = createFakeProvider();
  server.listen(port, '127.0.0.1', () => {
    process.stdout.write(`${JSON.stringify({ endpoint: `http://127.0.0.1:${server.address().port}/v1/chat/completions`, model: 'fake-ok' })}\n`);
  });
  const close = () => { server.closeAllConnections(); server.close(); };
  process.on('SIGTERM', close);
  process.on('SIGINT', close);
}
