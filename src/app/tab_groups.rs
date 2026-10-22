//! Saved tab groups: named sets of open terminals (order, names, themes, fonts,
//! layout and a startup command each) that reopen in one click.
use super::*;
use crate::settings::{SavedTab, TabGroup};

fn layout_name(layout: PaneLayout) -> &'static str {
    match layout {
        PaneLayout::Single => "single",
        PaneLayout::Columns => "columns",
        PaneLayout::Rows => "rows",
        PaneLayout::Grid => "grid",
    }
}

fn parse_layout(name: &str) -> PaneLayout {
    match name {
        "columns" => PaneLayout::Columns,
        "rows" => PaneLayout::Rows,
        "grid" => PaneLayout::Grid,
        _ => PaneLayout::Single,
    }
}

impl ButtonsApp {
    pub(super) fn capture_tab_group(&self, name: &str) -> TabGroup {
        let tabs = self
            .tabs
            .iter()
            .map(|tab| SavedTab {
                title: tab.title.clone(),
                custom_title: tab.custom_title.is_some(),
                profile_id: tab.profile_id.clone(),
                working_directory: tab
                    .working_directory
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                theme_id: self.theme_overrides.get(&tab.id).cloned(),
                font: self.pane_fonts.get(&tab.id).cloned(),
                auto_tile: self.auto_tile.includes(tab.id),
                startup_command: self
                    .tab_launch_commands
                    .get(&tab.id)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect();
        TabGroup {
            name: name.trim().to_owned(),
            layout: layout_name(self.pane_layout).into(),
            visible_panes: self.visible_panes.len(),
            focused: self.focused,
            tabs,
        }
    }

    /// Saves the open terminals as `name`, replacing a group with the same name.
    pub(super) fn save_tab_group(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A group name is required.".into());
        }
        if self.tabs.is_empty() {
            return Err("Open at least one terminal first.".into());
        }
        let group = self.capture_tab_group(name);
        let groups = &mut self.preferences.tab_groups.groups;
        match groups.iter_mut().find(|existing| existing.name == name) {
            Some(existing) => *existing = group,
            None => groups.push(group),
        }
        if self.preferences.tab_groups.launch_group.is_empty() {
            self.preferences.tab_groups.launch_group = name.to_owned();
        }
        self.persist_native_preferences();
        Ok(())
    }

    /// Opens every terminal in the group next to the current ones. Returns false
    /// when the group is missing or no terminal could start.
    pub(super) fn open_tab_group(&mut self, ctx: &egui::Context, name: &str) -> bool {
        let Some(group) = self
            .preferences
            .tab_groups
            .groups
            .iter()
            .find(|group| group.name == name)
            .cloned()
        else {
            return false;
        };
        let first = self.tabs.len();
        for saved in &group.tabs {
            let directory = Some(saved.working_directory.trim())
                .filter(|directory| !directory.is_empty())
                .and_then(|directory| resolve_working_directory(directory).ok().flatten())
                .filter(|directory| directory.is_dir());
            let before = self.tabs.len();
            self.open_tab_in(ctx.clone(), &saved.profile_id, directory);
            if self.tabs.len() == before {
                continue;
            }
            let index = self.tabs.len() - 1;
            let id = self.tabs[index].id;
            if saved.custom_title {
                self.tabs[index].rename(saved.title.clone());
            }
            if let Some(theme_id) = &saved.theme_id {
                if self.themes.all().iter().any(|theme| &theme.id == theme_id) {
                    self.theme_overrides.insert(id, theme_id.clone());
                }
            }
            if let Some(font) = &saved.font {
                self.pane_fonts.insert(id, font.clone());
            }
            if !saved.auto_tile {
                self.auto_tile.set_included(id, false);
            }
            let command = saved.startup_command.trim();
            if !command.is_empty() {
                self.tab_launch_commands.insert(id, command.to_owned());
                self.startup_commands
                    .push((id, command.to_owned(), std::time::Instant::now()));
            }
        }
        let opened = self.tabs.len() - first;
        if opened == 0 {
            self.notice = Some(format!("Could not open tab group \"{name}\"."));
            return false;
        }
        self.focused = first + group.focused.min(opened - 1);
        match parse_layout(&group.layout) {
            PaneLayout::Single => self.set_pane_layout(PaneLayout::Single, ctx),
            layout => {
                self.pane_layout = layout;
                self.auto_tile.requested_count = group.visible_panes.clamp(2, 10);
                self.refresh_auto_tiles();
            }
        }
        self.preferences.tab_groups.launch_group = name.to_owned();
        true
    }

    pub(super) fn open_group_save(&mut self) {
        self.group_name_draft = self.preferences.tab_groups.launch_group.clone();
        self.group_save_error = None;
        self.show_group_save = true;
    }

    /// File menu entries: save the open terminals, or open a saved group.
    pub(super) fn tab_group_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui
            .add_enabled(
                !self.tabs.is_empty(),
                crate::ui_text::button(
                    ui,
                    crate::i18n::literal(&self.locale, "Save tabs as group…"),
                ),
            )
            .on_hover_text("Save these terminals, their order, themes and layout under a name")
            .clicked()
        {
            self.open_group_save();
            ui.close_menu();
        }
        let names: Vec<String> = self
            .preferences
            .tab_groups
            .groups
            .iter()
            .map(|group| group.name.clone())
            .collect();
        if names.is_empty() {
            return;
        }
        let mut open = None;
        ui.spaced_menu_button(crate::i18n::literal(&self.locale, "Open tab group"), |ui| {
            for name in names {
                if ui.spaced_button(name.as_str()).clicked() {
                    open = Some(name);
                    ui.close_menu();
                }
            }
        });
        if let Some(name) = open {
            self.open_tab_group(ctx, &name);
            ui.close_menu();
        }
    }

    pub(super) fn group_save_window(&mut self, ctx: &egui::Context) {
        if !self.show_group_save {
            return;
        }
        let mut open = self.show_group_save;
        let mut save = false;
        let mut cancel = false;
        let replacing = self
            .preferences
            .tab_groups
            .groups
            .iter()
            .any(|group| group.name == self.group_name_draft.trim());
        egui::Window::new(crate::i18n::literal(&self.locale, "Save tab group"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                apply_zone_style(
                    ui,
                    &self.font_catalog,
                    &self.preferences.typography.settings,
                );
                ui.spaced_label(format!("Group name ({} terminals)", self.tabs.len()));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.group_name_draft)
                        .hint_text("e.g. VPS servers")
                        .desired_width(f32::INFINITY),
                );
                if !response.has_focus() && self.group_name_draft.is_empty() {
                    response.request_focus();
                }
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if replacing {
                    ui.spaced_label("Replaces the saved group with this name.");
                }
                if let Some(error) = &self.group_save_error {
                    ui.spaced_colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .spaced_button(crate::i18n::literal(&self.locale, "Cancel"))
                        .clicked()
                    {
                        cancel = true;
                    }
                    if ui
                        .spaced_button(crate::i18n::literal(&self.locale, "Save"))
                        .clicked()
                        || submitted
                    {
                        save = true;
                    }
                });
            });
        if save {
            let name = self.group_name_draft.clone();
            match self.save_tab_group(&name) {
                Ok(()) => {
                    open = false;
                    self.notice = Some(format!("Saved tab group \"{}\".", name.trim()));
                }
                Err(error) => self.group_save_error = Some(error),
            }
        }
        if cancel {
            open = false;
        }
        self.show_group_save = open;
    }

    pub(super) fn tab_group_settings(&mut self, ui: &mut egui::Ui) {
        ui.spaced_heading(crate::i18n::literal(&self.locale, "Tab groups"));
        ui.spaced_label("Save your open terminals (order, names, themes, fonts and layout) and reopen them in one click. Each terminal can run a startup command, such as ssh root@my-server. Running a Left command saves it as that terminal's startup command.");
        if ui
            .add_enabled(
                !self.tabs.is_empty(),
                crate::ui_text::button(ui, "Save current tabs as group…"),
            )
            .clicked()
        {
            self.open_group_save();
        }
        ui.separator();

        let groups = &mut self.preferences.tab_groups;
        ui.spaced_checkbox(
            &mut groups.restore_on_launch,
            "Open a tab group when ButtonsCLI starts",
        )
        .on_hover_text("Command-line options such as --tabs still win.");
        ui.add_enabled_ui(
            groups.restore_on_launch && !groups.groups.is_empty(),
            |ui| {
                ui.horizontal(|ui| {
                    let label = ui.spaced_label("Group to open");
                    egui::ComboBox::from_id_salt("tab-group-launch")
                        .selected_text(crate::ui_text::spaced_text(
                            ui,
                            if groups.launch_group.is_empty() {
                                "Choose a group".to_owned()
                            } else {
                                groups.launch_group.clone()
                            },
                        ))
                        .show_ui(ui, |ui| {
                            for group in &groups.groups {
                                ui.spaced_selectable_value(
                                    &mut groups.launch_group,
                                    group.name.clone(),
                                    group.name.as_str(),
                                );
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
            },
        );
        if groups.groups.is_empty() {
            ui.spaced_label(
                "No saved groups yet. Use File → Save tabs as group… or the button above.",
            );
            return;
        }
        ui.separator();

        let mut open = None;
        let mut delete = None;
        let mut update = None;
        for (index, group) in groups.groups.iter_mut().enumerate() {
            ui.push_id(("tab-group", index), |ui| {
                ui.horizontal(|ui| {
                    ui.spaced_label(RichText::new(group.name.as_str()).strong());
                    ui.spaced_label(format!("{} terminals · {}", group.tabs.len(), group.layout));
                    if ui.spaced_button("Open").clicked() {
                        open = Some(group.name.clone());
                    }
                    if ui
                        .spaced_button("Update")
                        .on_hover_text("Replace with the terminals open now")
                        .clicked()
                    {
                        update = Some(group.name.clone());
                    }
                    if ui.spaced_button("Delete").clicked() {
                        delete = Some(index);
                    }
                });
                egui::CollapsingHeader::new("Terminals and startup commands")
                    .id_salt("tabs")
                    .show(ui, |ui| {
                        egui::Grid::new("saved-tabs")
                            .num_columns(3)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.spaced_label(RichText::new("Name").strong());
                                ui.spaced_label(RichText::new("Folder").strong());
                                ui.spaced_label(RichText::new("Startup command").strong());
                                ui.end_row();
                                for tab in &mut group.tabs {
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(&mut tab.title)
                                                .desired_width(120.0),
                                        )
                                        .changed()
                                    {
                                        tab.custom_title = true;
                                    }
                                    ui.add(
                                        egui::TextEdit::singleline(&mut tab.working_directory)
                                            .hint_text("default")
                                            .desired_width(180.0),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut tab.startup_command)
                                            .hint_text("e.g. ssh root@my-server")
                                            .desired_width(220.0),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
            });
            ui.add_space(4.0);
        }
        if let Some(index) = delete {
            let removed = self.preferences.tab_groups.groups.remove(index);
            if self.preferences.tab_groups.launch_group == removed.name {
                self.preferences.tab_groups.launch_group = self
                    .preferences
                    .tab_groups
                    .groups
                    .first()
                    .map(|group| group.name.clone())
                    .unwrap_or_default();
            }
            self.persist_native_preferences();
        }
        if let Some(name) = update {
            if let Err(error) = self.save_tab_group(&name) {
                self.notice = Some(error);
            }
        }
        if let Some(name) = open {
            let ctx = ui.ctx().clone();
            self.open_tab_group(&ctx, &name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_names_round_trip() {
        for layout in [
            PaneLayout::Single,
            PaneLayout::Columns,
            PaneLayout::Rows,
            PaneLayout::Grid,
        ] {
            assert_eq!(parse_layout(layout_name(layout)), layout);
        }
        assert_eq!(parse_layout("unknown"), PaneLayout::Single);
    }

    #[test]
    fn saved_group_reopens_order_names_themes_layout_and_commands() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        let simulated = crate::assistant::virtual_terminal::PROFILE_ID;
        app.open_tab_with_profile(ctx.clone(), simulated);
        app.open_tab_with_profile(ctx.clone(), simulated);
        app.tabs[0].rename("vps1".into());
        let theme = app.themes.all().last().unwrap().id.clone();
        app.theme_overrides.insert(app.tabs[1].id, theme.clone());
        app.tab_launch_commands
            .insert(app.tabs[1].id, "ssh root@vps2".into());
        app.set_pane_layout(PaneLayout::Columns, &ctx);
        app.save_tab_group("servers").unwrap();
        assert_eq!(app.preferences.tab_groups.launch_group, "servers");

        while !app.tabs.is_empty() {
            app.close_tab(0);
        }
        assert!(app.open_tab_group(&ctx, "servers"));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.tabs[0].title, "vps1");
        assert_eq!(app.theme_overrides.get(&app.tabs[1].id), Some(&theme));
        assert_eq!(app.pane_layout, PaneLayout::Columns);
        assert_eq!(app.visible_panes.len(), 2);
        assert!(app
            .startup_commands
            .iter()
            .any(|(id, command, _)| *id == app.tabs[1].id && command == "ssh root@vps2"));
        assert!(!app.open_tab_group(&ctx, "missing"));
    }

    #[test]
    fn tab_groups_default_and_load_from_older_settings() {
        let preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert!(preferences.tab_groups.groups.is_empty());
        assert!(!preferences.tab_groups.restore_on_launch);
        let saved: SavedTab =
            serde_json::from_str(r#"{"title":"vps1","startup_command":"ssh root@vps1"}"#).unwrap();
        assert!(saved.auto_tile);
        assert_eq!(saved.profile_id, "system");
        assert_eq!(saved.startup_command, "ssh root@vps1");
    }
}
