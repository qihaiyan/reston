use egui::{Key, KeyboardShortcut, Modifiers};

/// All the commands we support.
///
/// Most are available in the GUI,
/// some have keyboard shortcuts,
/// and all are visible in the [`crate::CommandPalette`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, strum_macros::EnumIter)]
pub enum Command {
    AddApi,
    DelApi,
    RenameApi,
}

impl Command {
    pub fn text(self) -> &'static str {
        self.text_and_tooltip().0
    }

    pub fn tooltip(self) -> &'static str {
        self.text_and_tooltip().1
    }

    pub fn text_and_tooltip(self) -> (&'static str, &'static str) {
        match self {
            Command::AddApi => ("add", "add api"),
            Command::DelApi => ("del", "del api"),
            Command::RenameApi => ("rename", "rename api"),
        }
    }

    pub fn kb_shortcut(self) -> Option<KeyboardShortcut> {
        fn key(key: Key) -> KeyboardShortcut {
            KeyboardShortcut::new(Modifiers::NONE, key)
        }

        fn cmd(key: Key) -> KeyboardShortcut {
            KeyboardShortcut::new(Modifiers::COMMAND, key)
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn cmd_shift(key: Key) -> KeyboardShortcut {
            KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), key)
        }

        fn ctrl_shift(key: Key) -> KeyboardShortcut {
            KeyboardShortcut::new(Modifiers::CTRL.plus(Modifiers::SHIFT), key)
        }

        match self {
            Command::AddApi => Some(cmd(Key::A)),
            Command::DelApi => Some(cmd(Key::D)),
            Command::RenameApi => Some(cmd(Key::R)),
        }
    }

    // #[must_use = "Returns the Command that was triggered by some keyboard shortcut"]
    // pub fn listen_for_kb_shortcut(egui_ctx: &egui::Context) -> Option<Command> {
    //     use strum::IntoEnumIterator as _;

    //     let anything_has_focus = egui_ctx.memory(|mem| mem.focus().is_some());
    //     if anything_has_focus {
    //         return None; // e.g. we're typing in a TextField
    //     }

    //     egui_ctx.input_mut(|input| {
    //         for command in Command::iter() {
    //             if let Some(kb_shortcut) = command.kb_shortcut() {
    //                 if input.consume_shortcut(&kb_shortcut) {
    //                     return Some(command);
    //                 }
    //             }
    //         }
    //         None
    //     })
    // }

    /// Show this command as a menu-button.
    ///
    /// If clicked, enqueue the command.
    pub fn menu_button_ui(
        self,
        ui: &mut egui::Ui,
        pending_commands: &mut Vec<Command>,
    ) -> egui::Response {
        let button = egui::Button::new(self.text());
        // let button = self.menu_button(ui.ctx());
        let response = ui.add(button);
        if response.clicked() {
            pending_commands.push(self);
            ui.close_menu();
        }
        response
    }

    pub fn menu_button(self, egui_ctx: &egui::Context) -> egui::Button {
        let mut button = egui::Button::new(self.text());
        if let Some(shortcut) = self.kb_shortcut() {
            button = button.shortcut_text(egui_ctx.format_shortcut(&shortcut));
        }
        button
    }

    /// Add e.g. " (Ctrl+F11)" as a suffix
    pub fn format_shortcut_tooltip_suffix(self, egui_ctx: &egui::Context) -> String {
        if let Some(kb_shortcut) = self.kb_shortcut() {
            format!(" ({})", egui_ctx.format_shortcut(&kb_shortcut))
        } else {
            Default::default()
        }
    }
}
