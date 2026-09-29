# Remaining legacy effects audit

Source audit: 2026-09-29. This note records the disposition of the visual
items that do not have a direct, bounded native implementation in V02.

| Legacy item | Finding | Disposition |
| --- | --- | --- |
| TV noise | `tvNoiseEnabled` and `tvNoiseAmount` appear in source data, defaults, and theme fixtures. No runtime renderer or Settings control was found. | Preserve imported values as unsupported data. Do not claim renderer parity without an original runtime behavior to port. |
| Glow | Glow math appears inside Shader Lab GLSL presets and shader prompt guidance. It is not a standalone terminal effect setting. | Keep it in V04 Shader Lab scope, where shader compatibility and a safe native rendering path can be designed together. |
| Wallpaper | No wallpaper setting, image loader, or terminal wallpaper renderer was found. | Treat as new feature work, not legacy parity. Before implementation, define file selection, persistence, supported formats, size/dimension bounds, scaling, and fallback behavior. |
| HSync | The original effect composites canvas layers and shifts physical scanlines. The native cell renderer has no post-process texture stage. | Pixel-faithful implementation is a no-go until a bounded offscreen/GPU renderer spike demonstrates a safe path; see [HSync renderer decision](HSYNC-DECISION.md). |

Row banding and bounded simple noise are implemented in native source and
covered by automated tests. Their live appearance and frame cost at 1, 4, and
10 panes remain unverified; those checks are still part of V02 acceptance.
