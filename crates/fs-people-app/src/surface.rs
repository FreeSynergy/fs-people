//! The program as a screen of the shell.
//!
//! Pattern: Adapter — the workbench as a `Surface` of the application shell; the only screen, so the
//! shell draws no navigation.

use std::sync::Arc;

use fs_components_app::{AppError, AppWidget, Element, Screen, Surface};
use fs_components_workbenches::people::{PeopleSource, PeopleWidget};

use crate::words::{TITLE, labels};

/// The people workbench as a surface.
pub struct PeopleScreen(pub PeopleWidget);

impl Surface for PeopleScreen {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        Box::new(self.0.build(width, height))
    }

    fn styles(&self) -> String {
        self.0.styles()
    }
}

/// The shell around the workbench over `source`.
///
/// # Errors
/// [`AppError`] if the shell cannot be made.
pub fn app_widget(source: Arc<dyn PeopleSource>) -> Result<AppWidget, AppError> {
    let widget = PeopleWidget::new(source, labels());
    let screen = Screen::new("people", TITLE, Arc::new(PeopleScreen(widget)));
    AppWidget::new(TITLE, vec![screen])
}
