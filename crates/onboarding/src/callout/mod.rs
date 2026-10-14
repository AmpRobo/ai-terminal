mod model;
mod view;

pub use model::{FinalState, OnboardingQuery};
pub use view::{OnboardingCalloutView, OnboardingCalloutViewEvent, OnboardingKeybindings};

pub fn init(app: &mut ai_terminal_ui_core::AppContext) {
    view::init(app);
}
