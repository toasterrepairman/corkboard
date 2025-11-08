use crate::snippet::Snippet;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use sourceview5 as sv;
use sourceview5::prelude::*;

pub struct SnippetDialog {
    dialog: adw::Dialog,
    title_entry: adw::EntryRow,
    language_entry: adw::EntryRow,
    source_view: sv::View,
}

impl SnippetDialog {
    pub fn new() -> Self {
        let dialog = adw::Dialog::builder()
            .title("Add Code Snippet")
            .build();

        // Create toolbar header
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

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
        let source_view = sv::View::builder()
            .buffer(&source_buffer)
            .show_line_numbers(true)
            .monospace(true)
            .auto_indent(true)
            .indent_on_tab(true)
            .indent_width(4)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Enable syntax highlighting
        let scheme_manager = sv::StyleSchemeManager::default();
        if let Some(scheme) = scheme_manager.scheme("classic") {
            source_buffer.set_style_scheme(Some(&scheme));
        }

        // Wrap SourceView in a scrolled window
        let scrolled_window = gtk::ScrolledWindow::builder()
            .child(&source_view)
            .vexpand(true)
            .hexpand(true)
            .height_request(300)
            .has_frame(true)
            .build();

        // Create a clamp for the code editor with label
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

        toolbar_view.set_content(Some(&clamp));
        dialog.set_child(Some(&toolbar_view));

        // Set dialog size
        dialog.set_content_width(700);
        dialog.set_content_height(600);

        Self {
            dialog,
            title_entry,
            language_entry,
            source_view,
        }
    }

    pub fn present<W: IsA<gtk::Widget>>(&self, parent: &W) -> Option<Snippet> {
        // Clear previous values
        self.title_entry.set_text("");
        self.language_entry.set_text("rust");

        if let Some(buffer) = self.source_view.buffer().downcast_ref::<sv::Buffer>() {
            buffer.set_text("");
        }

        // Use a channel to get the result
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();

        // Add button
        let add_button = gtk::Button::builder()
            .label("Add")
            .css_classes(vec!["suggested-action"])
            .build();

        let title_entry = self.title_entry.clone();
        let language_entry = self.language_entry.clone();
        let source_view = self.source_view.clone();
        let dialog = self.dialog.clone();

        add_button.connect_clicked(move |_| {
            let title = title_entry.text().to_string();
            let language = language_entry.text().to_string();

            let code = if let Some(buffer) = source_view.buffer().downcast_ref::<sv::Buffer>() {
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                buffer.text(&start, &end, false).to_string()
            } else {
                String::new()
            };

            if !title.is_empty() && !code.is_empty() {
                let snippet = Snippet::new(title, language, code);
                let _ = tx.send(Some(snippet));
                dialog.close();
            }
        });

        // Cancel button
        let cancel_button = gtk::Button::builder()
            .label("Cancel")
            .build();

        let dialog_clone = self.dialog.clone();
        cancel_button.connect_clicked(move |_| {
            let _ = tx_clone.send(None);
            dialog_clone.close();
        });

        // Add buttons to header
        if let Some(toolbar_view) = self.dialog.child().and_then(|c| c.downcast::<adw::ToolbarView>().ok()) {
            // Get the first child which should be the header bar we added
            let mut child = toolbar_view.first_child();
            while let Some(widget) = child {
                if let Ok(header_bar) = widget.clone().downcast::<adw::HeaderBar>() {
                    header_bar.pack_start(&cancel_button);
                    header_bar.pack_end(&add_button);
                    break;
                }
                child = widget.next_sibling();
            }
        }

        self.dialog.present(Some(parent));

        // Wait for the dialog to be closed
        rx.recv().ok().flatten()
    }
}
