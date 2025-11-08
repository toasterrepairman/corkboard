mod snippet;
mod snippet_dialog;
mod window;
mod embeddings;

use gtk::prelude::*;
use libadwaita as adw;

const APP_ID: &str = "com.github.corkboard";

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &adw::Application) {
    let window = window::Window::new(app);
    window.present();
}
