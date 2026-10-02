# Remaining legacy effects audit

Source audit: 2026-09-29. This note records the disposition of the visual
items that do not have a direct, bounded native implementation in V02.

| Legacy item | Finding | Disposition |
| --- | --- | --- |
| TV noise | `tvNoiseEnabled` and `tvNoiseAmount` appear in source data, defaults, and theme fixtures. No runtime renderer or Settings control was found. | Preserve imported values as unsupported data. Do not claim renderer parity without an original runtime behavior to port. |
| Analog static | The original `analogStatic.ts` renders animated full-surface WebGL noise with density, drift, brightness, intensity, and opacity controls. Native source previously approximated this with at most 500 sparse dots. | Ported the original procedural shader to WGSL on 2026-09-30, including screen blending and all five controls. Windows adapter validation and an isolated screenshot pass; broader visual and frame-cost review remain open. |
| Glow | Glow math appears inside Shader Lab GLSL presets and shader prompt guidance. It is not a standalone terminal effect setting. | Keep it in V04 Shader Lab scope, where shader compatibility and a safe native rendering path can be designed together. |
| Wallpaper | No wallpaper setting, image loader, or terminal wallpaper renderer was found. | Treat as new feature work, not legacy parity. Before implementation, define file selection, persistence, supported formats, size/dimension bounds, scaling, and fallback behavior. |
| HSync | The original effect composites canvas layers and shifts physical scanlines. The native cell renderer has no post-process texture stage. | Pixel-faithful implementation is a no-go until a bounded offscreen/GPU renderer spike demonstrates a safe path; see [HSync renderer decision](HSYNC-DECISION.md). |

Row banding and bounded simple noise are implemented in native source and
covered by automated tests. Simple noise now uses a bounded texture at the
configured pixel resolution. Analog static uses a native GPU callback, with a bounded texture fallback.
Windows screenshots confirm fine grain at one pane. Frame cost at 1, 4, and
10 panes and broader platform/DPI appearance remain part of V02 acceptance.
