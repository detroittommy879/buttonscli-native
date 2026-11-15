"""Regression fixtures for the Rust UI-text scanner (no catalogs modified)."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('audit_i18n', Path(__file__).with_name('audit-i18n.py'))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def candidates(source, path='src/app.rs'):
    return [site[0] for site in audit.direct_sites(source, path)]


class AuditUiTextTests(unittest.TestCase):
    def test_all_widget_text_arguments_across_lines(self):
        # Include misleading English in the non-text arguments to ensure that
        # the scanner respects each signature instead of searching whole calls.
        for method, index in audit.TEXT_ARGUMENT.items():
            if method == 'folding_section':
                call = 'folding_section(ui, base, 0, open,\n"Preview", |ui| {})'
            elif method == 'spaced_text':
                call = 'crate::ui_text::spaced_text(ui,\n"Preview")'
            else:
                args = ['"Not display text"'] * index + ['"Preview"']
                call = 'ui.' + method + '(\n' + ',\n'.join(args) + ')'
            with self.subTest(method=method):
                self.assertEqual(candidates(call), ['Preview'])
        for constructor in audit.CONSTRUCTORS:
            with self.subTest(constructor=constructor):
                self.assertEqual(candidates('egui::' + constructor + '::new(\n"Preview")'), ['Preview'])

    def test_format_strings_are_text_but_their_arguments_are_data(self):
        source = '''ui.selectable_value(&mut value, Mode::One,
            format!(
                "Pane {n}", value("User name"),
            ));
            ui.on_hover_text(format!("Open {}", "File name"));'''
        self.assertEqual(candidates(source), ['Pane {n}', 'Open {}'])

    def test_menu_and_combo_ids_are_never_text(self):
        source = '''ui.solo_menu_button("Look menu state", "Look", |ui| {});
            crate::ui_text::solo_menu_button(ui, "Look menu state", "Look", |ui| {});
            egui::ComboBox::new("API format state", "API format");
            ui.push_id("English looking ID", |ui| {});'''
        self.assertEqual(candidates(source), ['Look', 'Look', 'API format'])

    def test_comments_strings_and_test_modules_do_not_leak(self):
        source = '''// ui.button("Comment text");
            let example = "ui.button(\\"String data\\")";
            #[cfg(test)]
            #[allow(dead_code)]
            mod tests { fn nested() { ui.button("Test only"); } }
            ui.button("Production text");'''
        self.assertEqual(candidates(source), ['Production text'])

    def test_developer_sections_are_excluded_but_public_power_options_remain(self):
        source = '''fn shader_options() { ui.checkbox(&mut x, "Feedback"); }
            fn debug_settings() { ui.label("Debug only"); }
            if power::debug_tools_available() { ui.button("Gallery"); }
            if release_picks::available() { ui.button("Release picks"); }
            ui.button("Public settings");'''
        self.assertEqual(candidates(source, 'src/app/power.rs'), ['Feedback', 'Public settings'])
        for path in ['src/app/gallery.rs', 'src/app/release_picks.rs', 'src/web_showcase.rs',
                     'src/app/ui_tests.rs', 'src/tests/helpers.rs', 'src/plugins/effects.rs']:
            with self.subTest(path=path):
                self.assertEqual(candidates('ui.button("Excluded text")', path), [])

    def test_raw_escaped_and_continued_strings(self):
        source = 'ui.label(r###"Raw text"###); ui.label("First \\\n    second"); ui.label("Open \\"file\\"");'
        self.assertEqual(candidates(source), ['Raw text', 'First second', 'Open "file"'])

    def test_branch_results_and_nested_builders(self):
        source = '''ui.selected_text(match value {
            Kind::One => "First choice".into(),
            Kind::Two => { "Second choice" },
        });
        ui.button(if ready { "Send" } else { format!("Waiting {n}") });
        ui.label(RichText::new("Bold text").strong());
        ui.button(crate::i18n::literal(locale, "Already translated"));'''
        self.assertEqual(candidates(source), ['First choice', 'Second choice', 'Send', 'Waiting {n}', 'Bold text'])

    def test_units_code_and_short_text_are_not_candidates(self):
        for text in ['A', '+', ' s', ' ms', ' px', ' min', 'MiB', 'WGSL', 'ButtonsCLI',
                     'shader-preview', 'settings.json', 'Ctrl+Shift+T', '--tabs', '{} · {}']:
            with self.subTest(text=text):
                self.assertFalse(audit.english_candidate(text))
        self.assertTrue(audit.english_candidate('Y up'))

    def test_deferred_key_markers_are_wrapped_including_multiline_calls(self):
        source = '''let labels = [crate::i18n::key("Save to shader list"),
            i18n::key(
                "Update in shader list",
            ), crate::i18n::key("5 s")];
            unrelated::key("Not a translation key");
            ui.button(crate::i18n::literal(locale, labels[0]));'''
        self.assertEqual(candidates(source), [])
        self.assertEqual([site[0] for site in audit.direct_sites(source, 'src/app.rs', translated=True)],
                         ['Save to shader list', 'Update in shader list', '5 s'])

    def test_wrapped_conditional_keys_are_still_catalog_work(self):
        source = '''ui.label(crate::i18n::formatted_literal(locale,
            if saved { "Editing: {draft}" } else { "Editing: {draft} (new theme, not saved yet)" },
            &[("draft", "User supplied name")],
        ));'''
        self.assertEqual(candidates(source), [])
        self.assertEqual([site[0] for site in audit.direct_sites(source, 'src/app.rs', translated=True)],
                         ['Editing: {draft}', 'Editing: {draft} (new theme, not saved yet)'])


if __name__ == '__main__':
    unittest.main()
