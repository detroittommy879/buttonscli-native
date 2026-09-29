# Window appearance

On Windows, open **Settings → Workspace** and adjust **Window opacity**. The
setting applies to the main ButtonsCLI window and is saved in the active
native profile. Opacity ranges from 25% to 100%; 100% is fully opaque. The
value is applied to the native window, so terminal text and controls fade
together.

Window opacity is available on Windows. Linux and macOS currently show an
unsupported-capability message because the native renderer does not expose a
reliable window-opacity API for those platforms. The stored preference remains
available if a future build adds support. Imported `window.opacity` values are
clamped to the supported range.

This feature is free and registered as `windowTransparency` in the central
feature catalog.
