# Read-only guides

Open **Help → Read-only guides** to view the bundled Quick start and AI Help
guides. The viewer uses separate display tabs and has no connection to the
terminal input actions, so reading a guide cannot type into a shell.

The AI Help guide covers **Settings → AI providers**, where you can add named
providers, configure compatible endpoint URLs and model IDs, and save a key in
the operating system credential store or for the current app session. AI Help
itself remains a separate window and requires explicit review before sending a
suggested command or key to a terminal.

The **Online guide** tab loads a single Markdown file from the ButtonsCLI
`/dsp/` path after you click **Load online guide**. The request uses HTTPS,
rejects redirects, times out after eight seconds, and accepts only Markdown or
plain text up to 256 KiB and 8,192 lines. Rendering handles headings, lists,
paragraphs and code blocks as text; HTML and Markdown links are never executed.
Only the ButtonsCLI website button can open a browser, and it uses a fixed
HTTPS URL.

The bundled guides work offline. If the online request fails, the viewer shows
an error while the local tabs remain available. No arbitrary webpage embedding
or browsing is included. Read-only guides are free and have no network activity
until you request the online guide or open the website yourself.
