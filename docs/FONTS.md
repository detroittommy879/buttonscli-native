# System and custom fonts

ButtonsCLI Native offers bundled font faces and discovers fonts already
installed on the computer. Discovery is local and works offline. On Windows it
checks the system Fonts folder and the current user's Windows Fonts folder;
Linux and macOS use their standard system and user font folders.
Startup scans at most 8,192 directory entries and indexes up to 512 local font
files and 256 MiB of face data; each file must be at most 32 MiB. The catalog
does not currently benchmark startup time across large font installations.

Open **Settings → Fonts** to choose a font for the shell, tabs, command dock,
Settings, AI Help, status bar, or terminal. Installed system faces show
**(System)** or **(Custom)** beside the family name. Each zone lists available weights; the
terminal's bold face is chosen separately, while the regular face determines
terminal cell sizing. Missing fonts fall back to the bundled UI or terminal
font and keep the saved selection so it can be restored if the font becomes
available again.

To add a font for this ButtonsCLI profile, enter the path to a `.ttf` or `.otf`
file and choose **Import local font**. The app validates the font before
copying it into
`~/.buttonscli-native/profiles/<profile>/fonts/`, then makes it available in
the selectors immediately. Existing files are never replaced; a numbered
filename is used when needed. Import is local, free, and independent of the
Settings **Revert & Close** action. It does not install fonts into Windows or
download anything.

The native renderer supports TrueType `.ttf` and OpenType `.otf` files up to
32 MiB each. Collections (`.ttc`) and web fonts (`.woff` / `.woff2`) are not
supported. Invalid or unsupported files are skipped during system discovery
and rejected during import. Text uses bundled symbol and Noto fallbacks when
the selected face lacks a glyph; exact glyph coverage depends on the font.

Online font downloads are not part of this feature. The `customFonts` access
tier is free.
