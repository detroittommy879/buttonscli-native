import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFakeProvider, models } from './fake-ai-provider.mjs';

test('local fixture discovers models, streams Unicode and reviewed actions, and handles retry/failure/cancel', async () => {
  const server = createFakeProvider({ delayMs: 1 });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const request = (model, options = {}) => fetch(`${base}/v1/chat/completions`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model, messages: [{ role: 'user', content: 'Terminal output snapshot: test' }], stream: true }), ...options,
  });
  try {
    assert.deepEqual((await (await fetch(`${base}/v1/models`)).json()).data.map(model => model.id), models);
    const reply = await (await request('fake-ok')).text();
    const content = reply.split('\n\n').filter(event => event && !event.includes('[DONE]'))
      .map(event => JSON.parse(event.slice(6)).choices[0].delta.content).join('');
    assert.match(content, /Local test response 🦇/);
    assert.match(content, /context detected: yes/);
    assert.match(content, /"sendEnter":false/);
    assert.match(reply, /data: \[DONE\]/);
    assert.equal((await request('fake-error-once')).status, 503);
    assert.equal((await request('fake-error-once')).status, 200);
    assert.equal((await request('fake-quota')).status, 429);
    assert.match(await (await request('fake-malformed')).text(), /invalid json/);
    await assert.rejects(async () => (await request('fake-disconnect')).text());
    const cancel = new AbortController();
    const slow = await request('fake-slow', { signal: cancel.signal });
    const reader = slow.body.getReader();
    assert.equal((await reader.read()).done, false);
    cancel.abort();
    await assert.rejects(() => reader.read());
    const status = await (await fetch(`${base}/status`)).json();
    assert.equal(status.requests, 7);
    assert.equal(status.contextRequests, 7);
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
});
