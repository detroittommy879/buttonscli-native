// The native bridge is optional. Keys and native settings never enter browser storage.
export async function connectSimulator(app, status) {
  const promptButton = document.getElementById('prompt-settings');
  const offlineButton = document.getElementById('offline-mode');
  const aiButton = document.getElementById('ai-mode');
  const desktopButton = document.getElementById('desktop-prompts');
  let session = null;
  let active = null;
  let statusText = '';
  const clientId = globalThis.crypto?.randomUUID?.() || `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  const headers = () => ({'Content-Type':'application/json','X-ButtonsCLI-Preview':session?.token || ''});
  const report = error => { status.textContent = String(error).slice(0, 512); };
  const cancelNetwork = () => {
    if (!active) return;
    const request = active;
    active = null;
    request.controller.abort();
    if (session) fetch(`./api/simulation/${encodeURIComponent(clientId)}/${request.id}`, {method:'DELETE',headers:headers(),keepalive:true}).catch(()=>{});
  };
  try {
    const response = await fetch('./api/session', {cache:'no-store'});
    if (response.ok && response.headers.get('content-type')?.includes('application/json')) {
      session = await response.json();
      app.configure_simulator(Boolean(session.available), session.model || '');
      aiButton.disabled = !session.available;
      aiButton.title = session.available ? `Uses ${session.model}. Provider requests may use credits.` : 'Choose a simulation model in native AI models, then restart the preview server.';
      desktopButton.hidden = false;
    }
  } catch { /* Static hosting keeps the offline simulator available. */ }
  promptButton.disabled = offlineButton.disabled = false;
  promptButton.addEventListener('click',()=>app.open_simulator_settings());
  offlineButton.addEventListener('click',()=> { cancelNetwork(); app.use_ai_simulator(false); });
  aiButton.addEventListener('click',()=> { try { cancelNetwork(); app.use_ai_simulator(true); } catch(error) { report(error); } });
  desktopButton.addEventListener('click',()=> { try { app.import_simulator_setup(JSON.stringify(session.config)); app.open_simulator_settings(); } catch(error) { report(error); } });
  async function send(request) {
    const task = {id:request.id,controller:new AbortController()};
    active = task;
    const timeout = setTimeout(()=>task.controller.abort(),190000);
    try {
      const response = await fetch('./api/simulation',{method:'POST',headers:headers(),body:JSON.stringify({...request,client_id:clientId}),signal:task.controller.signal});
      const result = await response.json();
      if (!response.ok) throw new Error(result.error || `Simulator returned HTTP ${response.status}`);
      app.finish_simulator_request(request.id,JSON.stringify(result.turn),result.model || '');
    } catch(error) {
      if (active === task) {
        app.fail_simulator_request(request.id,error.name === 'AbortError' ? 'Request timed out; virtual state was kept.' : String(error));
        fetch(`./api/simulation/${encodeURIComponent(clientId)}/${request.id}`,{method:'DELETE',headers:headers(),keepalive:true}).catch(()=>{});
      }
    } finally {
      clearTimeout(timeout);
      if (active === task) active = null;
    }
  }
  const timer = setInterval(()=> {
    try {
      const state = JSON.parse(app.simulator_status());
      if (active && state.pendingId !== active.id) cancelNetwork();
      const text = state.error || (state.ai ? `${state.model} · ${state.busy ? 'Thinking…' : state.cwd}` : `Offline practice · ${state.cwd}`);
      if (text !== statusText) { status.textContent = statusText = text; }
      aiButton.setAttribute('aria-pressed', String(state.ai));
      offlineButton.setAttribute('aria-pressed', String(!state.ai));
      const raw = app.take_simulator_request();
      if (raw) {
        const request = JSON.parse(raw);
        if (!session) app.fail_simulator_request(request.id,'Start the local AI preview server first.');
        else void send(request);
      }
    } catch { clearInterval(timer); cancelNetwork(); }
  },100);
  addEventListener('pagehide',()=> { clearInterval(timer); cancelNetwork(); app.cancel_simulator(); },{once:true});
}
