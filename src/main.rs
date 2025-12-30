mod app;
mod config;
mod ui;

use gtk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(app::State::new()));

    // Load data in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.write().await.load().await;
    });

    let app = gtk::Application::builder()
        .application_id("com.example.launcher")
        .build();

    let state_for_ui = state.clone();
    app.connect_activate(move |app| {
        ui::Window::new(app, state_for_ui.clone()).show();
    });

    app.run();
}