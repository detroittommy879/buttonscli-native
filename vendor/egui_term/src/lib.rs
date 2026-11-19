mod backend;
mod bindings;
mod font;
mod letter_rings;
mod text_backing;
mod theme;
mod types;
mod view;
pub use text_backing::{
    backing_shapes, contrast_ratio, ensure_contrast, last_copied, note_copied,
    relative_luminance, BackingPalette, BackingStyle, RingStyle, TextBacking,
    MAX_FEATHER,
};

pub use alacritty_terminal::event_loop::SensitiveInput;
pub use alacritty_terminal::term::ClipboardType;
pub use backend::settings::BackendSettings;
pub use backend::{
    set_output_repaint_interval, BackendCommand, ByteObserver, PtyEvent,
    ScrollbackState, TerminalBackend, TerminalMode,
};
pub use bindings::{Binding, BindingAction, InputKind, KeyboardBinding};
pub use font::{FontSettings, TerminalFont};
pub use theme::{ColorPalette, TerminalTheme};
pub use view::{BackgroundGradient, TerminalView};
