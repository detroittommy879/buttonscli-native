// Text stays in the DOM; backing is a separate layer below each run or line.
export function terminalLines(terminal, segments) {
  terminal.replaceChildren();
  let row;
  const newRow = () => {
    row = document.createElement('div'); row.className = 'term-line';
    terminal.append(row);
  };
  newRow();
  for (const [text, color] of segments) {
    text.split('\n').forEach((line, index) => {
      if (index) newRow();
      for (const part of line.split(/( {2,}|\t+)/)) {
        if (!part) continue;
        if (!part.trim()) { row.append(document.createTextNode(part)); continue; }
        row.classList.add('has-text');
        const span = document.createElement('span'); span.className = 'text-run';
        span.textContent = part;
        if (color) span.style.setProperty('--ink', color);
        row.append(span);
      }
    });
  }
}

export function installBacking(params) {
  const $ = id => document.getElementById(id);
  const number = (name, fallback, min, max) => {
    const value = params.has(name) ? Number(params.get(name)) : fallback;
    return Number.isFinite(value) ? Math.max(min, Math.min(max, value)) : fallback;
  };
  $('backing').checked = params.get('backing') !== '0';
  $('backing-invert').checked = params.get('invert') === '1';
  $('backing-lines').checked = params.get('lines') === '1';
  $('backing-opacity').value = number('opacity', 85, 0, 100);
  $('backing-feather').value = number('feather', 6, 0, 20);
  const update = () => {
    const enabled = $('backing').checked && Number($('backing-opacity').value) > 0;
    const workspace = $('workspace');
    workspace.classList.toggle('backing-on', enabled);
    workspace.classList.toggle('backing-inverted', enabled && $('backing-invert').checked);
    workspace.classList.toggle('backing-lines', $('backing-lines').checked);
    workspace.style.setProperty('--backing-opacity', Number($('backing-opacity').value) / 100);
    workspace.style.setProperty('--backing-feather', `${$('backing-feather').value}px`);
    $('backing-opacity-value').textContent = `${$('backing-opacity').value}%`;
    $('backing-feather-value').textContent = $('backing-feather').value;
  };
  for (const id of ['backing', 'backing-invert', 'backing-lines', 'backing-opacity', 'backing-feather'])
    $(id).addEventListener('input', update);
  update();
}
