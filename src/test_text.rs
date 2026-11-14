//! Look → Test text: a sample screen for checking colors, styles and glyphs
//! in the real terminal renderer (theme, font, text backing, shader).

/// Profile ID of the test pane. It has no shell: reopening a closed test pane
/// or a saved tab group that holds one opens a new test pane.
pub(crate) const PROFILE_ID: &str = "buttonscli-test-text";

/// The sample, as terminal output (CRLF line ends, SGR colors).
pub(crate) fn sample() -> String {
    let mut out = String::new();
    let line = |out: &mut String, text: &str| {
        out.push_str(text);
        out.push_str("\x1b[0m\r\n");
    };
    line(
        &mut out,
        "\x1b[1mButtonsCLI test text\x1b[0m  (no shell; close the tab when done)",
    );
    line(&mut out, "");

    // The 16 ANSI colors as text, then as backgrounds.
    let mut normal = String::from("ANSI    ");
    let mut bright = String::from("bright  ");
    for color in 0..8 {
        normal.push_str(&format!("\x1b[{}m {color:<2}", 30 + color));
        bright.push_str(&format!("\x1b[{}m {:<2}", 90 + color, color + 8));
    }
    line(&mut out, &normal);
    line(&mut out, &bright);
    let mut normal_bg = String::from("bg      ");
    let mut bright_bg = String::from("bright  ");
    for color in 0..8 {
        normal_bg.push_str(&format!("\x1b[{}m   \x1b[0m", 40 + color));
        bright_bg.push_str(&format!("\x1b[{}m   \x1b[0m", 100 + color));
    }
    line(&mut out, &normal_bg);
    line(&mut out, &bright_bg);
    line(
        &mut out,
        "names   \x1b[31mred \x1b[32mgreen \x1b[33myellow \x1b[34mblue \x1b[35mmagenta \x1b[36mcyan \x1b[37mwhite",
    );

    // 256-color cube slice, grey ramp and a truecolor gradient.
    let mut cube = String::from("256     ");
    for color in (16..232).step_by(6).take(24) {
        cube.push_str(&format!("\x1b[48;5;{color}m "));
    }
    line(&mut out, &cube);
    let mut grey = String::from("grey    ");
    for color in 232..256 {
        grey.push_str(&format!("\x1b[48;5;{color}m "));
    }
    line(&mut out, &grey);
    let mut rgb = String::from("rgb     ");
    for step in 0..24u32 {
        let red = 255 - step * 10;
        let blue = step * 10;
        rgb.push_str(&format!("\x1b[48;2;{red};80;{blue}m "));
    }
    line(&mut out, &rgb);
    line(&mut out, "");

    // Text styles.
    line(
        &mut out,
        "styles  \x1b[1mbold\x1b[0m \x1b[2mdim\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munderline\x1b[0m \x1b[7minverse\x1b[0m \x1b[9mstrike\x1b[0m \x1b[1;3mbold italic",
    );
    line(&mut out, "default normal text on the default background");
    line(
        &mut out,
        "\x1b[2mdim text\x1b[0m and \x1b[1;31mbold red\x1b[0m and \x1b[1;32mbold green",
    );
    line(&mut out, "");

    // Glyphs that commonly break: lookalikes, accents, boxes, symbols.
    line(&mut out, "ascii   0O 1lI| {}[]() <>=!=-> ~`'\" $#@%&*");
    line(&mut out, "accents àéîõü ñç ß ÆØÅ Ελληνικά Кириллица");
    line(&mut out, "boxes   ┌─┬─┐ ╔═╦═╗ ╭─╮ ▏▎▍▌▋▊▉█ ░▒▓");
    line(&mut out, "        ├─┼─┤ ╠═╬═╣ ╰─╯ ▁▂▃▄▅▆▇█ ⣿⡇⠿");
    line(&mut out, "        └─┴─┘ ╚═╩═╝ ←↑→↓ ✓✗ •·… ±×÷ °");
    line(&mut out, "wide    日本語 中文 한국어 (each 2 cells)");
    line(&mut out, "emoji   🙂 🚀 ✨ 🔥 ⚠️");
    line(
        &mut out,
        "nerd     \u{e0b0} \u{e0b2} \u{f115} \u{e702} (blank if the font has none)",
    );
    line(&mut out, "");

    // A prompt and some listing-like output.
    line(
        &mut out,
        "\x1b[32muser@host\x1b[0m:\x1b[34m~/project\x1b[0m$ git status",
    );
    line(&mut out, "On branch \x1b[36mmain\x1b[0m");
    line(&mut out, "\x1b[32m\tmodified:   src/app.rs");
    line(&mut out, "\x1b[31m\tdeleted:    old.txt");
    line(
        &mut out,
        "\x1b[33mwarning:\x1b[0m something to check   \x1b[31;1merror:\x1b[0m something failed",
    );
    out.push_str("\x1b[32muser@host\x1b[0m:\x1b[34m~/project\x1b[0m$ ");
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn sample_resets_colors_and_ends_lines_for_the_terminal() {
        let sample = super::sample();
        assert!(sample.lines().count() > 20);
        assert!(sample.contains("\x1b[0m\r\n"));
        assert!(
            !sample.contains("\n\n"),
            "bare LF would not return the cursor"
        );
    }
}
