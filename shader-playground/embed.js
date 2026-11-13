/* Embed with data-x/data-y or --buttonscli-x/--buttonscli-y on the parent. */
(() => {
  const script = document.currentScript;
  if (!script) return;
  const size = (value, fallback) => {
    if (/^\d+(?:\.\d+)?$/.test(value || '')) return `${value}px`;
    return /^\d+(?:\.\d+)?(?:px|%|vw|vh|dvh|rem)$/.test(value || '') ? value : fallback;
  };
  const url = new URL('./', script.src);
  url.searchParams.set('embed', '1');
  for (const key of ['fps', 'bundle', 'layout', 'controls'])
    if (script.dataset[key]) url.searchParams.set(key, script.dataset[key]);
  const frame = document.createElement('iframe');
  frame.src = url.href;
  frame.title = 'ButtonsCLI shader playground';
  frame.loading = 'lazy'; frame.allow = 'fullscreen';
  frame.style.cssText = 'display:block;max-width:100%;border:0;border-radius:8px;color-scheme:dark;';
  frame.style.width = size(script.dataset.x, 'var(--buttonscli-x, 100%)');
  frame.style.height = size(script.dataset.y, 'var(--buttonscli-y, 720px)');
  script.before(frame);
  // Offscreen embeds stop drawing, even when their parent page remains visible.
  let visible = false;
  const notify = () => frame.contentWindow?.postMessage({type:'buttonscli-visibility', visible}, url.origin);
  const observer = new IntersectionObserver(entries => {visible = entries[0].isIntersecting; notify();});
  observer.observe(frame); frame.addEventListener('load', notify);
})();
