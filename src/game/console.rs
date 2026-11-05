#[path = "console_commands.rs"]
mod commands;
#[path = "console_ui.rs"]
mod console_ui;

use crate::game::world;
use crate::input::InputEvent;
use crate::model::GameState;
use crate::presentation::ConsoleView;
use std::io;
use std::path::Path;

pub(crate) struct ConsoleSession {
    state: console_ui::ConsoleState,
}

impl Default for ConsoleSession {
    fn default() -> Self {
        let mut state = console_ui::ConsoleState::default();
        state
            .output
            .push("The Ever-Unfolding Saga developer console".into());
        state
            .output
            .push("help for commands | Tab completion | Esc closes".into());
        Self { state }
    }
}

impl ConsoleSession {
    pub(crate) fn view(&self) -> ConsoleView {
        console_ui::build_view(&self.state)
    }

    pub(crate) fn push_text(&mut self, text: &str) {
        for character in text.chars().filter(|character| !character.is_control()) {
            self.state.input.push(character);
            self.state.history_index = None;
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn set_text(&mut self, text: &str) {
        self.state.input = text.to_string();
        self.state.history_index = None;
    }

    pub(crate) fn edit(&mut self, key: InputEvent) {
        console_ui::edit_input(&mut self.state, key);
    }

    pub(crate) fn history_previous(&mut self) {
        console_ui::history_previous(&mut self.state);
    }

    pub(crate) fn history_next(&mut self) {
        console_ui::history_next(&mut self.state);
    }

    pub(crate) fn scroll_up(&mut self, amount: usize) {
        console_ui::scroll_up(&mut self.state, amount);
    }

    pub(crate) fn scroll_down(&mut self, amount: usize) {
        console_ui::scroll_down(&mut self.state, amount);
    }

    pub(crate) fn jump_home(&mut self) {
        console_ui::jump_home(&mut self.state);
    }

    pub(crate) fn jump_end(&mut self) {
        console_ui::jump_end(&mut self.state);
    }

    pub(crate) fn is_autocomplete(&self) -> bool {
        self.state.autocomplete
    }

    pub(crate) fn start_completion(&mut self, game_state: &GameState) {
        console_ui::refresh_completion(&mut self.state, game_state);
        if self.state.candidates.is_empty() {
            self.state.autocomplete = false;
            return;
        }
        self.state.autocomplete = true;
        self.state.selected = 0;
        self.state.completion_scroll = 0;
        console_ui::keep_completion_selection_visible(&mut self.state, 8);
    }

    pub(crate) fn select_previous_completion(&mut self) {
        console_ui::select_previous(&mut self.state);
    }

    pub(crate) fn select_next_completion(&mut self) {
        console_ui::select_next(&mut self.state);
    }

    pub(crate) fn accept_completion(&mut self) {
        console_ui::accept_completion(&mut self.state);
    }

    pub(crate) fn cancel_completion(&mut self) {
        console_ui::cancel_completion(&mut self.state);
    }

    pub(crate) fn execute_line(
        &mut self,
        state: &mut GameState,
        save_path: &Path,
    ) -> io::Result<()> {
        commands::execute_line(state, save_path, &mut self.state)
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.state.exit
    }

    pub(crate) fn output_error(&mut self, message: &str) {
        self.state.output.push(message.to_string());
    }
}

pub(crate) fn bootstrap_after_console(state: &mut GameState) {
    world::bootstrap_campaign_content(state);
}
