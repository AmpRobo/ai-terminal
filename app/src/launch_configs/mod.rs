pub mod launch_config;
pub mod save_modal;

use ai_terminal_ui::AppContext;

pub fn init(app: &mut AppContext) {
    save_modal::init(app);
}
