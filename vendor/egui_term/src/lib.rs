mod backend;
mod bindings;
mod font;
mod text_backing;
mod theme;
mod types;
mod view;
pub use text_backing::TextBacking;

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
