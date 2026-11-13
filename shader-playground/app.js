const $ = id => document.getElementById(id);
const accents = ['#489cff', '#ff9c38', '#23cc78', '#ed489e'];
let library, device, template, helpers, format, selected = 0, paused = matchMedia('(prefers-reduced-motion: reduce)').matches;
let layout = 'grid', paneCount = 4, lastFrame = 0, compileQueue = Promise.resolve();
const params = new URLSearchParams(location.search);
let fps = Math.max(1, Math.min(120, Number(params.get('fps')) || 30)), nextFrame = 0, embedVisible = true;
fps = Math.round(fps);
document.body.classList.toggle('embedded', params.has('embed'));
addEventListener('message', event => {
  if (event.source === parent && event.data?.type === 'buttonscli-visibility' && typeof event.data.visible === 'boolean') embedVisible = event.data.visible;
});
const panes = [], pipelineCache = new Map();
const rgb = (hex = '#080b16') => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255);
const stripComments = code => code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
const chromeKeys = new Set(['shell-background','shell-backgroundSecondary','shell-border','shell-textMain','shell-textDim','shell-accent','shell-accentHover','shell-buttonBackground','presetDock-background','presetDock-accent','presetDock-buttonBackground','presetDock-buttonHover','presetDock-buttonText','tabs-background','tabs-idleBackground','tabs-activeBackground','tabs-activeBorder','statusBar-background','statusBar-border','statusBar-text','statusBar-warning']);
function applyChrome(colors) {
  if (!colors || !Object.keys(colors).length) return;
  for (const key of chromeKeys) document.documentElement.style.removeProperty(`--${key}`);
  for (const [key, color] of Object.entries(colors))
    if (chromeKeys.has(key) && /^#[0-9a-f]{6}$/i.test(color)) document.documentElement.style.setProperty(`--${key}`, color);
}
function fpsLabel() {
  $('fps').value = fps; $('fps-value').textContent = fps;
  $('total').textContent = `${library.shaders.length} shaders · ${fps} fps target`;
}

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
      if (errors.length) throw new Error(errors.map(m => m.message).join('\n'));
      return await device.createRenderPipelineAsync({layout:'auto', vertex:{module, entryPoint:'vertex'}, fragment:{module, entryPoint:'fragment', targets:[{format}]}, primitive:{topology:'triangle-list'}});
    } finally {
      const error = await device.popErrorScope();
      if (error) console.warn(error.message);
    }
  });
  compileQueue = pending.catch(() => {});
  pipelineCache.set(index, pending);
  return pending;
}

async function apply(pane, index, updateChrome = true) {
  const revision = ++pane.revision;
  pane.index = index;
  const shader = library.shaders[index];
  if (updateChrome && panes[selected] === pane) applyChrome(shader.chrome);
  pane.values = {speed:shader.speed, scale:shader.scale, strength:shader.strength, background:shader.background, colors:[...shader.colors], knobs:shader.variables.map(v => v.value)};
  pane.terminal.style.color = shader.foreground;
  pane.time = 0;
  pane.pipeline = null;
  pane.name.textContent = shader.name;
  pane.error.textContent = device ? 'Loading shader…' : 'WebGPU is unavailable. Open this preview in a WebGPU-capable browser on HTTPS or localhost.';
  if (panes[selected] === pane) controls();
  if (!device) return;
  try {
    const ready = await pipeline(index);
    if (revision !== pane.revision) return;
    pane.pipeline = ready;
    pane.bind = device.createBindGroup({layout:ready.getBindGroupLayout(0), entries:[{binding:0, resource:{buffer:pane.buffer}}]});
    pane.error.textContent = '';
    pane.dirty = true;
  } catch (error) {
    if (revision === pane.revision) pane.error.textContent = `This shader could not render: ${error.message}`;
  }
}

function focus(index) {
  selected = index;
  panes.forEach((p,i) => {p.element.classList.toggle('selected', i === index); p.tab.classList.toggle('selected', i === index); p.tab.setAttribute('aria-pressed', String(i === index));});
  controls();
}
function range(parent, title, min, max, value, change, step = 0.01) {
  const label = document.createElement('label'); label.className = 'slider';
  const text = document.createElement('span'); text.textContent = title;
  const output = document.createElement('output'); output.textContent = Number(value).toFixed(2);
  const input = document.createElement('input'); Object.assign(input,{type:'range', min, max, step, value});
  input.addEventListener('input', () => {output.textContent = Number(input.value).toFixed(2); change(Number(input.value)); panes[selected].dirty = true;});
  label.append(text,output,input); parent.append(label);
}
function options() {
  const query = $('search').value.toLowerCase();
  const matches = library.shaders.map((s,i) => ({s,i})).filter(({s}) => s.name.toLowerCase().includes(query));
  $('shader').replaceChildren(...matches.map(({s,i}) => new Option(s.name,i)));
  $('shader').value = panes[selected].index;
  $('count').textContent = `${matches.length} shaders${query ? ' matching search' : ' · native WGSL'}`;
}
function controls() {
  if (!library || !panes[selected]) return;
  const p = panes[selected], s = library.shaders[p.index];
  $('target').textContent = `P${selected + 1}`; options();
  $('common').replaceChildren(); $('knobs').replaceChildren(); $('colors').replaceChildren();
  for (const [key,title,min,max] of [['speed','Speed',0,10],['scale','Scale',0.1,10],['strength','Strength',0,1]])
    range($('common'),title,min,max,p.values[key],v => p.values[key] = v);
  s.variables.forEach((v,i) => range($('knobs'),v.name.replaceAll('_',' '),v.min,v.max,p.values.knobs[i],n => p.values.knobs[i] = n,(v.max-v.min)/200));
  if (!s.variables.length) $('knobs').textContent = 'This shader has no custom sliders.';
  [...p.values.colors,p.values.background].forEach((value,i) => {
    const label = document.createElement('label'); label.textContent = i === 5 ? 'BG' : `C${i+1}`;
    const input = document.createElement('input'); input.type = 'color'; input.value = value;
    input.addEventListener('input',() => {if(i === 5) p.values.background = input.value; else p.values.colors[i] = input.value; p.dirty = true;});
    label.append(input); $('colors').append(label);
  });
}
function changeLayout(next, count = next === 'grid' ? 4 : next === 'one' ? 1 : 2) {
  paneCount = count;
  layout = next; $('workspace').className = next;
  panes.forEach((p,i) => {p.visible = next === 'one' ? i === selected : i < paneCount; p.element.hidden = !p.visible; p.element.style.display = p.visible ? '' : 'none'; p.element.style.gridColumn = count === 3 && i === 2 ? '1 / -1' : ''; p.dirty = true;});
  if(next !== 'one' && !panes[selected].visible) focus(0);
  document.querySelectorAll('[data-layout]').forEach(b => b.classList.toggle('active', b.dataset.layout === next));
}
function draw(now) {
  requestAnimationFrame(draw);
  if(now + 0.5 < nextFrame) return;
  const dt = Math.min((now-lastFrame)/1000,1);
  const interval = 1000/fps;
  nextFrame = Math.max(nextFrame + interval, now);
  lastFrame = now;
  if (!device || document.hidden || !embedVisible) return;
  for (const p of panes) {
    if (!p.visible || !p.pipeline) continue;
    const rect = p.element.getBoundingClientRect(), ratio = Math.min(devicePixelRatio,1.5);
    const width = Math.max(1,Math.round(rect.width*ratio)), height = Math.max(1,Math.round(rect.height*ratio));
    if(p.canvas.width !== width || p.canvas.height !== height){p.canvas.width = width; p.canvas.height = height; p.dirty = true;}
    if(!paused && p.values.speed > 0){p.time += dt*p.values.speed; p.dirty = true;}
    if(!p.dirty) continue;
    const v = p.values, data = new Float32Array(36);
    data.set([0,0,width,height,p.time,v.strength,v.scale,0]);
    data.set([...rgb(v.background),1],8);
    for(let i=0;i<3;i++) data.set([...rgb(v.colors[i]),1],12+i*4);
    data.set(v.knobs,24); data.set([...rgb(v.colors[3]),1],28); data.set([...rgb(v.colors[4]),1],32);
    device.queue.writeBuffer(p.buffer,0,data);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({colorAttachments:[{view:p.context.getCurrentTexture().createView(),loadOp:'clear',storeOp:'store',clearValue:{r:0,g:0,b:0,a:1}}]});
    pass.setPipeline(p.pipeline); pass.setBindGroup(0,p.bind); pass.draw(3); pass.end(); device.queue.submit([encoder.finish()]); p.dirty = false;
  }
}
function cycle(direction) {
  const indexes = [...$('shader').options].map(o => Number(o.value));
  if(!indexes.length) return;
  const pos = indexes.indexOf(panes[selected].index);
  apply(panes[selected],indexes[(pos+direction+indexes.length)%indexes.length]);
}
async function start() {
  const responses = await Promise.all(['data/library.json?v=20261113-3','data/shader_helpers.wgsl','data/shader_background.wgsl'].map(url => fetch(url).then(r => {if(!r.ok) throw new Error(`Cannot load ${url}`); return r;})));
  [library,helpers,template] = await Promise.all([responses[0].json(),responses[1].text(),responses[2].text()]);
  try {const adapter = await navigator.gpu?.requestAdapter(); device = await adapter?.requestDevice();} catch(error) {console.warn(error);}
  if(device) {
    format = navigator.gpu.getPreferredCanvasFormat();
    device.lost.then(info => {$('status').textContent = 'GPU disconnected · reload to retry'; panes.forEach(p => p.error.textContent = info.message || 'GPU disconnected. Reload to retry.'); device = null;});
  }
  for(let i=0;i<4;i++) {
    const element = document.createElement('section'); element.className = 'pane'; element.style.setProperty('--pane',accents[i]); element.setAttribute('aria-label',`Terminal pane ${i+1}`);
    const canvas = document.createElement('canvas');
    const terminal = document.createElement('div'); terminal.className = 'terminal';
    terminal.textContent = `PowerShell 7.6.10\nPS C:\\ButtonsCLI\\demo>\nPS C:\\ButtonsCLI\\demo> ▌`;
    const name = document.createElement('span'); name.className = 'pane-name';
    const error = document.createElement('div'); error.className = 'pane-error'; error.setAttribute('role','status');
    element.append(canvas,terminal,name,error); $('workspace').append(element);
    const tab = document.createElement('button'); tab.textContent = `term${i+1} · P${i+1}`; tab.style.setProperty('--pane',accents[i]); $('tabs').append(tab);
    const p = {element,canvas,terminal,name,error,tab,revision:0,visible:true,dirty:true};
    if(device){p.context = canvas.getContext('webgpu'); p.context.configure({device,format,alphaMode:'opaque'}); p.buffer = device.createBuffer({size:144,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});}
    panes.push(p);
    const choose = () => {focus(i); if(layout === 'one') changeLayout('one');}; element.addEventListener('click',choose); tab.addEventListener('click',choose);
  }
  const preferred = ['Moonlit Swell','Filtered Squarewave','Bubble Parade','Acid Waveform'];
  await Promise.all(panes.map((p,i) => {const index = library.shaders.findIndex(s => s.name === preferred[i]); return apply(p,index < 0 ? i : index);}));
  library.bundles.forEach((b,i) => $('bundle').add(new Option(b.name,i)));
  function applyBundle() {
    if($('bundle').value === '') return;
    const bundle = library.bundles[Number($('bundle').value)];
    bundle.panes.forEach((index,i) => apply(panes[i],index,false));
    applyChrome(bundle.chrome); focus(0);
    changeLayout(bundle.panes.length === 1 ? 'one' : bundle.panes.length === 2 ? 'col' : 'grid',bundle.panes.length);
  }
  $('bundle').addEventListener('change',applyBundle);
  $('shader').addEventListener('change',() => apply(panes[selected],Number($('shader').value)));
  $('search').addEventListener('input',options);
  $('prev').onclick = () => cycle(-1); $('next').onclick = () => cycle(1);
  $('reset').onclick = () => apply(panes[selected],panes[selected].index);
  $('shuffle').onclick = () => panes.filter(p => p.visible).forEach(p => apply(p,Math.floor(Math.random()*library.shaders.length)));
  $('pause').onclick = () => {paused = !paused; $('pause').textContent = paused ? 'Resume motion' : 'Pause motion';};
  $('pause').textContent = paused ? 'Resume motion' : 'Pause motion';
  $('text').onchange = () => document.body.classList.toggle('text-hidden',!$('text').checked);
  $('brightness').oninput = () => $('workspace').style.setProperty('--brightness',$('brightness').value);
  $('fps').oninput = () => {fps = Number($('fps').value); nextFrame = 0; fpsLabel();};
  $('settings').onclick = $('dock-toggle').onclick = () => document.body.classList.toggle('dock-hidden');
  $('help').onclick = () => $('help-dialog').showModal(); $('close-help').onclick = () => $('help-dialog').close();
  $('fullscreen').onclick = async () => {try {if(document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen();} catch { $('status').textContent = 'Fullscreen unavailable';}};
  document.querySelectorAll('[data-layout]').forEach(b => b.onclick = () => changeLayout(b.dataset.layout));
  focus(0); $('status').textContent = device ? 'SHADERS READY' : 'WEBGPU UNAVAILABLE'; fpsLabel();
  const initialBundle = library.bundles.findIndex(b => b.name === params.get('bundle'));
  if(initialBundle >= 0) {$('bundle').value = initialBundle; applyBundle();}
  if(['one','col','row','grid'].includes(params.get('layout'))) changeLayout(params.get('layout'));
  if(params.get('controls') === '0' || (innerWidth <= 600 && params.get('controls') !== '1')) document.body.classList.add('dock-hidden');
  requestAnimationFrame(draw);
}
start().catch(error => {$('status').textContent = `Preview failed: ${error.message}`; console.error(error);});
