use crate::app::{FilterMode, SearchItem, State};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Window {
    window: gtk::ApplicationWindow,
    entry: gtk::SearchEntry,
    listbox: gtk::ListBox,
    state: Arc<RwLock<State>>,
    selected_items: Arc<RwLock<Vec<SearchItem>>>,
    filter_mode: Arc<RwLock<FilterMode>>,
}

impl Window {
    pub fn new(app: &gtk::Application, state: Arc<RwLock<State>>) -> Self {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .default_width(600)
            .default_height(400)
            .decorated(false)
            .build();

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Search applications and commands..."));
        entry.set_margin_top(10);
        entry.set_margin_bottom(10);
        entry.set_margin_start(10);
        entry.set_margin_end(10);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let listbox = gtk::ListBox::new();
        listbox.set_selection_mode(gtk::SelectionMode::Single);
        scrolled.set_child(Some(&listbox));

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.append(&entry);
        vbox.append(&scrolled);
        window.set_child(Some(&vbox));

        let mut win = Self {
            window,
            entry,
            listbox,
            state,
            selected_items: Arc::new(RwLock::new(Vec::new())),
            filter_mode: Arc::new(RwLock::new(FilterMode::All)),
        };

        win.setup_handlers(scrolled);
        win
    }

    pub fn show(&self) {
        self.entry.grab_focus();
        self.window.present();
    }

    fn setup_handlers(&mut self, scrolled: gtk::ScrolledWindow) {
        self.setup_search();
        self.setup_activation();
        self.setup_keyboard(scrolled);
        self.setup_focus();
    }

    fn setup_search(&self) {
        let entry = self.entry.clone();
        let listbox = self.listbox.clone();
        let state = self.state.clone();
        let selected_items = self.selected_items.clone();
        let filter_mode = self.filter_mode.clone();

        self.entry.connect_search_changed(move |_| {
            let query = entry.text().to_string();
            let listbox = listbox.clone();
            let state = state.clone();
            let selected_items = selected_items.clone();
            let filter_mode = filter_mode.clone();

            glib::spawn_future_local(async move {
                let state = state.read().await;
                let mode = filter_mode.read().await;
                let results = state.search(&query, &mode);

                // Clear and populate
                while let Some(row) = listbox.first_child() {
                    listbox.remove(&row);
                }

                *selected_items.write().await = results.clone();

                for (i, item) in results.iter().enumerate() {
                    let row = create_row(item);
                    listbox.append(&row);
                    if i == 0 {
                        listbox.select_row(Some(&row));
                    }
                }
            });
        });
    }

    fn setup_activation(&self) {
        let window = self.window.clone();
        let entry = self.entry.clone();
        let selected_items = self.selected_items.clone();

        // Row activation
        self.listbox.connect_row_activated(move |_, row| {
            activate_item(row.index() as usize, &window, &entry, &selected_items);
        });

        // Enter key
        let window = self.window.clone();
        let listbox = self.listbox.clone();
        let entry = self.entry.clone();
        let selected_items = self.selected_items.clone();

        self.entry.connect_activate(move |_| {
            if let Some(row) = listbox.selected_row() {
                activate_item(row.index() as usize, &window, &entry, &selected_items);
            }
        });
    }

    fn setup_keyboard(&self, scrolled: gtk::ScrolledWindow) {
        let listbox = self.listbox.clone();
        let window = self.window.clone();
        let entry = self.entry.clone();
        let filter_mode = self.filter_mode.clone();

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            match key {
                gdk::Key::Escape => {
                    window.close();
                    glib::Propagation::Stop
                }
                gdk::Key::Down => {
                    navigate(&listbox, &scrolled, 1);
                    glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    navigate(&listbox, &scrolled, -1);
                    glib::Propagation::Stop
                }
                gdk::Key::a if modifier.contains(gdk::ModifierType::CONTROL_MASK) => {
                    set_mode(&filter_mode, &entry, FilterMode::Apps,
                             "Search applications... (Ctrl+A)");
                    glib::Propagation::Stop
                }
                gdk::Key::c if modifier.contains(gdk::ModifierType::CONTROL_MASK) => {
                    set_mode(&filter_mode, &entry, FilterMode::Commands,
                             "Search commands... (Ctrl+C)");
                    glib::Propagation::Stop
                }
                gdk::Key::r if modifier.contains(gdk::ModifierType::CONTROL_MASK) => {
                    set_mode(&filter_mode, &entry, FilterMode::All,
                             "Search applications and commands...");
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.entry.add_controller(key_controller);
    }

    fn setup_focus(&self) {
        let window = self.window.clone();
        let focus_controller = gtk::EventControllerFocus::new();

        focus_controller.connect_leave(move |_| {
            let window = window.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                if !window.is_active() {
                    window.close();
                }
            });
        });
        self.window.add_controller(focus_controller);
    }
}

fn activate_item(
    idx: usize,
    window: &gtk::ApplicationWindow,
    entry: &gtk::SearchEntry,
    selected_items: &Arc<RwLock<Vec<SearchItem>>>,
) {
    let window = window.clone();
    let entry = entry.clone();
    let selected_items = selected_items.clone();

    glib::spawn_future_local(async move {
        if let Some(item) = selected_items.read().await.get(idx) {
            match item {
                SearchItem::CustomCommand(cmd) if !cmd.subcommands.is_empty() => {
                    entry.set_text(&format!("{} > ", cmd.name));
                    entry.set_position(-1);
                }
                _ => {
                    item.execute();
                    window.close();
                }
            }
        }
    });
}

fn navigate(listbox: &gtk::ListBox, scrolled: &gtk::ScrolledWindow, dir: i32) {
    if let Some(row) = listbox.selected_row() {
        let new_idx = row.index() + dir;
        if let Some(new_row) = listbox.row_at_index(new_idx) {
            listbox.select_row(Some(&new_row));

            let vadj = scrolled.vadjustment();
            let row_height = 60.0;
            let target = (new_idx as f64) * row_height;
            let current = vadj.value();
            let page_size = vadj.page_size();

            if dir > 0 && target > current + page_size - row_height {
                vadj.set_value(target - page_size + row_height);
            } else if dir < 0 && target < current {
                vadj.set_value(target);
            }
        }
    }
}

fn set_mode(
    filter_mode: &Arc<RwLock<FilterMode>>,
    entry: &gtk::SearchEntry,
    mode: FilterMode,
    placeholder: &str,
) {
    let filter_mode = filter_mode.clone();
    let entry = entry.clone();
    let placeholder = placeholder.to_string();

    glib::spawn_future_local(async move {
        *filter_mode.write().await = mode;
        entry.set_placeholder_text(Some(&placeholder));
        entry.emit_by_name::<()>("search-changed", &[]);
    });
}

fn create_row(item: &SearchItem) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);

    let title = gtk::Label::new(Some(item.name()));
    title.set_halign(gtk::Align::Start);
    title.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(item.name())));

    let desc = gtk::Label::new(Some(&item.description()));
    desc.set_halign(gtk::Align::Start);
    desc.add_css_class("dim-label");

    row_box.append(&title);
    row_box.append(&desc);
    row.set_child(Some(&row_box));
    row
}