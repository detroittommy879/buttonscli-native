import http from 'node:http';

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://localhost');
  const anthropic = url.pathname.startsWith('/anthropic/');
  const authorized = anthropic
    ? req.headers['x-api-key'] === 'fixture-key' && req.headers['anthropic-version'] === '2023-06-01'
    : req.headers.authorization === 'Bearer fixture-key';
  const send = (status, body) => { res.writeHead(status, {'content-type': 'application/json'}); res.end(JSON.stringify(body)); };
  if (!authorized) return send(401, {error: {message: 'fixture authentication failed'}});
  if (url.pathname.endsWith('/models')) {
    if (url.pathname === '/discover/models') return send(404, {});
    return send(200, {data: ['fixture-chat', 'fixture-responses', 'fixture-claude'].map(id => ({id}))});
  }
  let raw = '';
  for await (const chunk of req) raw += chunk;
  const body = JSON.parse(raw);
  const messages = url.pathname.endsWith('/messages');
  const responses = url.pathname.endsWith('/responses');
  if (anthropic && !messages) return send(404, {});
  if (messages && (!body.system || body.messages[0].role === 'system' || !body.max_tokens)) return send(400, {error: {message: 'wrong Messages shape'}});
  if (responses && (!body.instructions || !Array.isArray(body.input) || body.store !== false)) return send(400, {error: {message: 'wrong Responses shape'}});
  if (!messages && !responses && (!body.messages || body.messages[0].role !== 'system')) return send(400, {error: {message: 'wrong Chat shape'}});
  const text = messages ? 'Messages ✓' : responses ? 'Responses ✓' : 'Chat ✓';
  if (!body.stream) {
    return send(200, messages
      ? {type: 'message', role: 'assistant', content: [{type: 'text', text}], stop_reason: 'end_turn'}
      : responses ? {status: 'completed', output: [{type: 'message', role: 'assistant', content: [{type: 'output_text', text}]}]}
      : {choices: [{message: {content: text}}]});
  }
  res.writeHead(200, {'content-type': 'text/event-stream'});
  const event = value => res.write(`data: ${JSON.stringify(value)}\n\n`);
  if (messages) {
    event({type: 'message_start', message: {content: [], model: 'fixture-claude'}});
    event({type: 'content_block_start', content_block: {type: 'text', text: ''}});
    event({type: 'content_block_delta', delta: {type: 'thinking_delta', thinking: 'hidden reasoning'}});
    event({type: 'content_block_delta', delta: {type: 'text_delta', text}});
    event({type: 'message_delta', delta: {stop_reason: 'end_turn'}});
    event({type: 'message_stop'});
  } else if (responses) {
    event({type: 'response.output_text.delta', delta: text});
    event({type: 'response.completed', response: {status: 'completed'}});
  } else {
    event({choices: [{delta: {content: text}}]});
    res.write('data: [DONE]\n\n');
  }
  res.end();
});
server.listen(0, '127.0.0.1', () => console.log(JSON.stringify({base: `http://127.0.0.1:${server.address().port}`})));
