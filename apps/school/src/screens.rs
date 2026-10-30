//! Product screens for the school app.

use ui_shell::Screen;
use ui_shell::egui;

pub struct State {
    pub store: storage::Store,
    pub enrolled: usize,
}

impl State {
    pub fn new(store: storage::Store) -> Self {
        Self { store, enrolled: 0 }
    }
}

pub struct StudentsScreen;

impl Screen<State> for StudentsScreen {
    fn title(&self) -> &str {
        "Students"
    }

    fn icon(&self) -> Option<&'static str> {
        Some("👤")
    }

    fn draw(&mut self, ui: &mut egui::Ui, state: &mut State) {
        ui_kit::components::section_title(ui, "Enrolment");
        ui.label(format!("{} student(s) enrolled", state.enrolled));
        ui.separator();
        ui.label(format!(
            "schema version: {}",
            state.store.user_version().unwrap_or(-1)
        ));
    }
}

pub struct ClassesScreen;

impl Screen<State> for ClassesScreen {
    fn title(&self) -> &str {
        "Classes"
    }

    fn icon(&self) -> Option<&'static str> {
        Some("🗂")
    }

    fn draw(&mut self, ui: &mut egui::Ui, _state: &mut State) {
        ui_kit::components::empty_state(
            ui,
            "No classes yet",
            "Timetables will appear here once terms are set up.",
        );
    }
}
