use crate::snippet::{load_snippets, save_snippets, Snippet, SnippetList};
use crate::embeddings::{EmbeddingsStore, generate_embedding, search_snippets};
use gtk::prelude::*;
use gtk::{gio, gdk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use sourceview5 as sv;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Window {
    window: adw::ApplicationWindow,
    snippets: SnippetList,
    list_box: gtk::ListBox,
    source_view: sv::View,
    split_view: adw::OverlaySplitView,
    embeddings_store: Rc<RefCell<EmbeddingsStore>>,
    search_bar: gtk::SearchBar,
    search_entry: gtk::SearchEntry,
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        let snippets = load_snippets();
        let embeddings_store = Rc::new(RefCell::new(EmbeddingsStore::load()));

        // Create main window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Corkboard")
            .default_width(1000)
            .default_height(700)
            .build();

        // Create header bar
        let header_bar = adw::HeaderBar::new();

        // Sidebar toggle button
        let sidebar_toggle = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Toggle Sidebar")
            .active(true)
            .build();

        header_bar.pack_start(&sidebar_toggle);

        // Add button
        let add_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add Snippet")
            .build();

        header_bar.pack_end(&add_button);

        // Create split view with auto-collapse behavior
        let split_view = adw::OverlaySplitView::builder()
            .sidebar_position(gtk::PackType::Start)
            .show_sidebar(true)
            .min_sidebar_width(250.0)
            .max_sidebar_width(400.0)
            .enable_show_gesture(true)
            .enable_hide_gesture(true)
            .build();

        // Auto-collapse sidebar when window width is below threshold
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 800px").unwrap());
        breakpoint.add_setter(&split_view, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        // Bind sidebar toggle button to split view's show_sidebar property
        split_view
            .bind_property("show-sidebar", &sidebar_toggle, "active")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        // Create search bar and entry
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search snippets...")
            .hexpand(true)
            .build();

        let search_bar = gtk::SearchBar::builder()
            .search_mode_enabled(false)
            .child(&search_entry)
            .build();

        // Create sidebar with list of snippets
        let scrolled_sidebar = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let list_box = gtk::ListBox::builder()
            .css_classes(vec!["navigation-sidebar"])
            .build();

        scrolled_sidebar.set_child(Some(&list_box));

        // Create sidebar container with search bar
        let sidebar_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        sidebar_box.append(&search_bar);
        sidebar_box.append(&scrolled_sidebar);

        split_view.set_sidebar(Some(&sidebar_box));

        // Create content area with SourceView
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let source_buffer = sv::Buffer::new(None);
        let source_view = sv::View::builder()
            .buffer(&source_buffer)
            .editable(false)
            .show_line_numbers(true)
            .monospace(true)
            .vexpand(true)
            .hexpand(true)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();

        // Ensure monospace font by setting CSS
        source_view.add_css_class("monospace");

        // Apply monospace font family via CSS
        let font_css = gtk::CssProvider::new();
        font_css.load_from_data("textview.monospace { font-family: monospace; font-size: 11pt; }");
        source_view.style_context().add_provider(&font_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        // Enable syntax highlighting with Classic theme
        let scheme_manager = sv::StyleSchemeManager::default();
        if let Some(scheme) = scheme_manager.scheme("Adwaita-dark") {
            source_buffer.set_style_scheme(Some(&scheme));
        }

        // Get the gutter and set background color programmatically
        use sourceview5::prelude::ViewExt as SvViewExt;
        let gutter = SvViewExt::gutter(&source_view, gtk::TextWindowType::Left);
        // Apply CSS to the gutter widget
        let gutter_css = gtk::CssProvider::new();
        gutter_css.load_from_data("* { background-color: shade(@view_bg_color, 0.90); }");
        gutter.style_context().add_provider(&gutter_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        // Override SourceView background to use system background color
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            "textview.view, textview.view > text { background-color: @view_bg_color; background-image: none; }"
        );
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // Disable background pattern
        source_view.set_background_pattern(sv::BackgroundPatternType::None);

        let scrolled_content = gtk::ScrolledWindow::builder()
            .child(&source_view)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Create overlay for floating copy button
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scrolled_content));

        // Create floating copy button
        let copy_button = gtk::Button::builder()
            .label("Copy")
            .icon_name("edit-copy-symbolic")
            .css_classes(vec!["pill", "accent"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .margin_bottom(24)
            .build();

        overlay.add_overlay(&copy_button);

        content_box.append(&overlay);

        split_view.set_content(Some(&content_box));

        // Create toolbar view
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&split_view));

        window.set_content(Some(&toolbar_view));

        let mut win = Self {
            window,
            snippets,
            list_box,
            source_view,
            split_view: split_view.clone(),
            embeddings_store,
            search_bar: search_bar.clone(),
            search_entry: search_entry.clone(),
        };

        // Populate the list
        win.populate_list();

        // Setup keyboard accelerators
        win.setup_actions(app);

        // Setup search functionality
        win.setup_search();

        // Generate missing embeddings on startup
        win.generate_missing_embeddings();

        // Connect copy button
        let source_view_clone = win.source_view.clone();
        copy_button.connect_clicked(move |_| {
            if let Some(buffer) = source_view_clone.buffer().downcast_ref::<sv::Buffer>() {
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                let text = buffer.text(&start, &end, false);

                if let Some(display) = gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&text);
                }
            }
        });

        // Connect add button
        let snippets_clone = win.snippets.clone();
        let list_box_clone = win.list_box.clone();
        let source_view_clone = win.source_view.clone();
        let window_clone = win.window.clone();
        let embeddings_store_clone = win.embeddings_store.clone();

        add_button.connect_clicked(move |_| {
            // Present dialog and wait for result
            // Note: This is a simplified approach. In a real app, you'd want to use async/await
            // For now, we'll use a simpler callback approach
            Self::show_add_dialog(
                &window_clone,
                &snippets_clone,
                &list_box_clone,
                &source_view_clone,
                &embeddings_store_clone,
            );
        });

        // Connect list selection
        let snippets_clone = win.snippets.clone();
        let source_view_clone = win.source_view.clone();
        let split_view_clone = win.split_view.clone();

        win.list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index() as usize;
                let snippets = snippets_clone.borrow();

                if let Some(snippet) = snippets.get(index) {
                    Self::display_snippet(&source_view_clone, snippet);

                    // Auto-hide sidebar if in collapsed/overlay mode for better UX
                    if split_view_clone.is_collapsed() {
                        split_view_clone.set_show_sidebar(false);
                    }
                }
            }
        });

        // Select first item by default
        if let Some(first_row) = win.list_box.row_at_index(0) {
            win.list_box.select_row(Some(&first_row));
        }

        win
    }

    fn setup_actions(&self, app: &adw::Application) {
        // Create action for adding new snippet (Ctrl+T)
        let add_action = gio::SimpleAction::new("new-snippet", None);
        let window_clone = self.window.clone();
        let snippets_clone = self.snippets.clone();
        let list_box_clone = self.list_box.clone();
        let source_view_clone = self.source_view.clone();
        let embeddings_store_clone = self.embeddings_store.clone();

        add_action.connect_activate(move |_, _| {
            Self::show_add_dialog(
                &window_clone,
                &snippets_clone,
                &list_box_clone,
                &source_view_clone,
                &embeddings_store_clone,
            );
        });

        self.window.add_action(&add_action);
        app.set_accels_for_action("win.new-snippet", &["<Ctrl>t"]);

        // Create action for closing window (Ctrl+Q)
        let close_action = gio::SimpleAction::new("close", None);
        let window_clone = self.window.clone();

        close_action.connect_activate(move |_, _| {
            window_clone.close();
        });

        self.window.add_action(&close_action);
        app.set_accels_for_action("win.close", &["<Ctrl>q"]);

        // Create action for toggling search (Ctrl+F)
        let search_action = gio::SimpleAction::new("toggle-search", None);
        let search_bar_clone = self.search_bar.clone();
        let search_entry_clone = self.search_entry.clone();

        search_action.connect_activate(move |_, _| {
            let current_mode = search_bar_clone.is_search_mode();
            search_bar_clone.set_search_mode(!current_mode);

            if !current_mode {
                // Focus the search entry when opening
                search_entry_clone.grab_focus();
            }
        });

        self.window.add_action(&search_action);
        app.set_accels_for_action("win.toggle-search", &["<Ctrl>f"]);
    }

    fn setup_search(&self) {
        let search_entry = self.search_entry.clone();
        let list_box = self.list_box.clone();
        let snippets = self.snippets.clone();
        let embeddings_store = self.embeddings_store.clone();

        // Connect to search entry's search-changed signal
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();

            if query.is_empty() {
                // Show all items when search is empty
                list_box.set_filter_func(|_| true);
            } else {
                // Perform async search using embeddings
                let query_clone = query.clone();
                let embeddings_store_clone = embeddings_store.clone();
                let snippets_clone = snippets.clone();
                let list_box_clone = list_box.clone();

                glib::MainContext::default().spawn_local(async move {
                    // Perform the search
                    if let Ok(results) = search_snippets(&query_clone, &embeddings_store_clone.borrow()).await {
                        // Create a set of matching IDs with scores above threshold
                        let matching_ids: std::collections::HashMap<String, f32> = results
                            .into_iter()
                            .filter(|(_, score)| *score > 0.3) // Threshold for relevance
                            .collect();

                        // Filter the list box based on matching IDs
                        list_box_clone.set_filter_func(move |row| {
                            let index = row.index() as usize;
                            let snippets = snippets_clone.borrow();

                            if let Some(snippet) = snippets.get(index) {
                                matching_ids.contains_key(&snippet.id)
                            } else {
                                false
                            }
                        });
                    }
                });
            }
        });
    }

    fn generate_missing_embeddings(&self) {
        let snippets = self.snippets.clone();
        let embeddings_store = self.embeddings_store.clone();

        // Delay embedding generation to not block UI startup
        glib::timeout_add_seconds_local(2, move || {
            let snippets_clone = snippets.clone();
            let embeddings_store_clone = embeddings_store.clone();

            glib::MainContext::default().spawn_local(async move {
                let snippets_list = snippets_clone.borrow().clone();

                for snippet in snippets_list.iter() {
                    // Check if embedding already exists
                    let has_embedding = embeddings_store_clone.borrow().get_embedding(&snippet.id).is_some();

                    if !has_embedding {
                        // Generate embedding for this snippet
                        let text = snippet.get_searchable_text();

                        match generate_embedding(&text).await {
                            Ok(embedding) => {
                                embeddings_store_clone.borrow_mut().set_embedding(snippet.id.clone(), embedding);
                                println!("Generated embedding for snippet: {}", snippet.title);
                            }
                            Err(e) => {
                                eprintln!("Failed to generate embedding for {}: {}", snippet.title, e);
                            }
                        }
                    }
                }

                // Save embeddings to disk
                if let Err(e) = embeddings_store_clone.borrow().save() {
                    eprintln!("Failed to save embeddings: {}", e);
                }
            });

            glib::ControlFlow::Break
        });
    }

    fn populate_list(&mut self) {
        // Clear existing rows
        while let Some(row) = self.list_box.first_child() {
            self.list_box.remove(&row);
        }

        // Add snippets to list
        let snippets = self.snippets.borrow();
        for (index, snippet) in snippets.iter().enumerate() {
            let row = self.create_snippet_row(snippet, index);
            self.list_box.append(&row);
        }
    }

    fn create_snippet_row(&self, snippet: &Snippet, _index: usize) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();

        let box_ = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let title_label = gtk::Label::builder()
            .label(&snippet.title)
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading"])
            .build();

        let lang_label = gtk::Label::builder()
            .label(&snippet.language)
            .halign(gtk::Align::Start)
            .css_classes(vec!["dim-label", "caption"])
            .build();

        box_.append(&title_label);
        box_.append(&lang_label);

        row.set_child(Some(&box_));

        // Add right-click context menu
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3); // Right-click

        let snippets_clone = self.snippets.clone();
        let list_box_clone = self.list_box.clone();
        let source_view_clone = self.source_view.clone();

        gesture.connect_released(move |gesture, _, x, y| {
            let menu = gio::Menu::new();
            menu.append(Some("Delete"), Some("snippet.delete"));

            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            if let Some(widget) = gesture.widget() {
                popover.set_parent(&widget);
            }
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

            // Create action group for this row
            let action_group = gio::SimpleActionGroup::new();
            let delete_action = gio::SimpleAction::new("delete", None);

            let snippets_clone2 = snippets_clone.clone();
            let list_box_clone2 = list_box_clone.clone();
            let source_view_clone2 = source_view_clone.clone();
            let row_clone = gesture.widget().unwrap().downcast::<gtk::ListBoxRow>().unwrap();

            delete_action.connect_activate(move |_, _| {
                let index = row_clone.index() as usize;

                // Remove from data
                snippets_clone2.borrow_mut().remove(index);

                // Save to disk
                if let Err(e) = save_snippets(&snippets_clone2.borrow()) {
                    eprintln!("Failed to save snippets: {}", e);
                }

                // Remove from UI
                list_box_clone2.remove(&row_clone);

                // Clear source view if this was the selected item
                if let Some(buffer) = source_view_clone2.buffer().downcast_ref::<sv::Buffer>() {
                    buffer.set_text("");
                }

                // Select first item if available
                if let Some(first_row) = list_box_clone2.row_at_index(0) {
                    list_box_clone2.select_row(Some(&first_row));
                }
            });

            action_group.add_action(&delete_action);

            if let Some(widget) = gesture.widget() {
                widget.insert_action_group("snippet", Some(&action_group));
            }

            popover.popup();
        });

        row.add_controller(gesture);
        row
    }

    fn display_snippet(source_view: &sv::View, snippet: &Snippet) {
        if let Some(buffer) = source_view.buffer().downcast_ref::<sv::Buffer>() {
            buffer.set_text(&snippet.code);

            // Set language for syntax highlighting
            let lang_manager = sv::LanguageManager::default();
            if let Some(language) = lang_manager.language(&snippet.language) {
                buffer.set_language(Some(&language));
            }
        }
    }

    fn show_add_dialog(
        window: &adw::ApplicationWindow,
        snippets: &SnippetList,
        list_box: &gtk::ListBox,
        source_view: &sv::View,
        embeddings_store: &Rc<RefCell<EmbeddingsStore>>,
    ) {
        let dialog = adw::Dialog::builder()
            .title("Add Code Snippet")
            .content_width(700)
            .content_height(600)
            .build();

        // Create toolbar header
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();

        // Create form fields
        let title_entry = adw::EntryRow::builder()
            .title("Title")
            .build();

        let language_entry = adw::EntryRow::builder()
            .title("Language")
            .text("rust")
            .build();

        // Create SourceView for code editing
        let source_buffer = sv::Buffer::new(None);
        let dialog_source_view = sv::View::builder()
            .buffer(&source_buffer)
            .show_line_numbers(true)
            .monospace(true)
            .auto_indent(true)
            .indent_on_tab(true)
            .indent_width(4)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Ensure monospace font by setting CSS
        dialog_source_view.add_css_class("monospace");

        // Apply monospace font family via CSS
        let font_css = gtk::CssProvider::new();
        font_css.load_from_data("textview.monospace { font-family: monospace; font-size: 11pt; }");
        dialog_source_view.style_context().add_provider(&font_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        // Enable syntax highlighting
        let scheme_manager = sv::StyleSchemeManager::default();
        if let Some(scheme) = scheme_manager.scheme("Adwaita-dark") {
            source_buffer.set_style_scheme(Some(&scheme));
        }

        // Disable background pattern
        dialog_source_view.set_background_pattern(sv::BackgroundPatternType::None);

        // Wrap SourceView in a scrolled window
        let scrolled_window = gtk::ScrolledWindow::builder()
            .child(&dialog_source_view)
            .vexpand(true)
            .hexpand(true)
            .height_request(300)
            .has_frame(true)
            .build();

        // Create a box for the code editor with label
        let code_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(12)
            .build();

        let code_label = gtk::Label::builder()
            .label("Code")
            .halign(gtk::Align::Start)
            .css_classes(vec!["title-4"])
            .build();

        code_box.append(&code_label);
        code_box.append(&scrolled_window);

        // Create preferences group for title and language
        let prefs_group = adw::PreferencesGroup::new();
        prefs_group.add(&title_entry);
        prefs_group.add(&language_entry);

        // Create main content box
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();

        content_box.append(&prefs_group);
        content_box.append(&code_box);

        // Create clamp for proper width
        let clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(600)
            .child(&content_box)
            .build();

        // Add button
        let add_button = gtk::Button::builder()
            .label("Add")
            .css_classes(vec!["suggested-action"])
            .build();

        let snippets_clone = snippets.clone();
        let list_box_clone = list_box.clone();
        let source_view_clone = source_view.clone();
        let dialog_clone = dialog.clone();
        let title_entry_clone = title_entry.clone();
        let language_entry_clone = language_entry.clone();
        let dialog_source_view_clone = dialog_source_view.clone();
        let embeddings_store_clone = embeddings_store.clone();

        add_button.connect_clicked(move |_| {
            let title = title_entry_clone.text().to_string();
            let language = language_entry_clone.text().to_string();

            let code = if let Some(buffer) = dialog_source_view_clone.buffer().downcast_ref::<sv::Buffer>() {
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                buffer.text(&start, &end, false).to_string()
            } else {
                String::new()
            };

            if !title.is_empty() && !code.is_empty() {
                let snippet = Snippet::new(title, language, code);

                // Add to list
                snippets_clone.borrow_mut().push(snippet.clone());

                // Save to disk
                if let Err(e) = save_snippets(&snippets_clone.borrow()) {
                    eprintln!("Failed to save snippets: {}", e);
                }

                // Create and add row
                let row = Self::create_snippet_row_static(
                    &snippet,
                    &snippets_clone,
                    &list_box_clone,
                    &source_view_clone,
                );
                list_box_clone.append(&row);

                // Select and display the new snippet
                let index = snippets_clone.borrow().len() - 1;
                if let Some(new_row) = list_box_clone.row_at_index(index as i32) {
                    list_box_clone.select_row(Some(&new_row));
                    Self::display_snippet(&source_view_clone, &snippet);
                }

                // Generate embedding for the new snippet asynchronously
                let snippet_clone = snippet.clone();
                let embeddings_store_clone2 = embeddings_store_clone.clone();
                glib::MainContext::default().spawn_local(async move {
                    let text = snippet_clone.get_searchable_text();
                    match generate_embedding(&text).await {
                        Ok(embedding) => {
                            embeddings_store_clone2.borrow_mut().set_embedding(snippet_clone.id.clone(), embedding);
                            if let Err(e) = embeddings_store_clone2.borrow().save() {
                                eprintln!("Failed to save embeddings: {}", e);
                            }
                            println!("Generated embedding for new snippet: {}", snippet_clone.title);
                        }
                        Err(e) => {
                            eprintln!("Failed to generate embedding for new snippet: {}", e);
                        }
                    }
                });

                dialog_clone.close();
            }
        });

        // Cancel button
        let cancel_button = gtk::Button::builder()
            .label("Cancel")
            .build();

        let dialog_clone = dialog.clone();
        cancel_button.connect_clicked(move |_| {
            dialog_clone.close();
        });

        // Add buttons to header
        header_bar.pack_start(&cancel_button);
        header_bar.pack_end(&add_button);

        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&clamp));
        dialog.set_child(Some(&toolbar_view));

        dialog.present(Some(window));
    }

    fn create_snippet_row_static(
        snippet: &Snippet,
        snippets: &SnippetList,
        list_box: &gtk::ListBox,
        source_view: &sv::View,
    ) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();

        let box_ = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let title_label = gtk::Label::builder()
            .label(&snippet.title)
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading"])
            .build();

        let lang_label = gtk::Label::builder()
            .label(&snippet.language)
            .halign(gtk::Align::Start)
            .css_classes(vec!["dim-label", "caption"])
            .build();

        box_.append(&title_label);
        box_.append(&lang_label);

        row.set_child(Some(&box_));

        // Add right-click context menu
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3); // Right-click

        let snippets_clone = snippets.clone();
        let list_box_clone = list_box.clone();
        let source_view_clone = source_view.clone();

        gesture.connect_released(move |gesture, _, x, y| {
            let menu = gio::Menu::new();
            menu.append(Some("Delete"), Some("snippet.delete"));

            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            if let Some(widget) = gesture.widget() {
                popover.set_parent(&widget);
            }
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

            // Create action group for this row
            let action_group = gio::SimpleActionGroup::new();
            let delete_action = gio::SimpleAction::new("delete", None);

            let snippets_clone2 = snippets_clone.clone();
            let list_box_clone2 = list_box_clone.clone();
            let source_view_clone2 = source_view_clone.clone();
            let row_clone = gesture.widget().unwrap().downcast::<gtk::ListBoxRow>().unwrap();

            delete_action.connect_activate(move |_, _| {
                let index = row_clone.index() as usize;

                // Remove from data
                snippets_clone2.borrow_mut().remove(index);

                // Save to disk
                if let Err(e) = save_snippets(&snippets_clone2.borrow()) {
                    eprintln!("Failed to save snippets: {}", e);
                }

                // Remove from UI
                list_box_clone2.remove(&row_clone);

                // Clear source view if this was the selected item
                if let Some(buffer) = source_view_clone2.buffer().downcast_ref::<sv::Buffer>() {
                    buffer.set_text("");
                }

                // Select first item if available
                if let Some(first_row) = list_box_clone2.row_at_index(0) {
                    list_box_clone2.select_row(Some(&first_row));
                }
            });

            action_group.add_action(&delete_action);

            if let Some(widget) = gesture.widget() {
                widget.insert_action_group("snippet", Some(&action_group));
            }

            popover.popup();
        });

        row.add_controller(gesture);
        row
    }

    pub fn present(&self) {
        self.window.present();
    }
}
