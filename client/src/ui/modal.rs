use crate::ui::style::{PANEL_WIDTH_SMALL, SPACE_MEDIUM};
use eframe::emath::Align;
use egui::{Context, Id, Layout, RichText, Ui, WidgetText};

/// Wraps an [`egui::Modal`] with app-specific layout and formatting options.
pub struct Modal {
    /// The internal name of this modal, should be unique
    name: String,

    /// The heading to display on the modal, defaults to the internal name
    heading: RichText,

    /// The body text of this modal
    body: Option<WidgetText>,

    /// Whether the user can dismiss this modal without consequences. Adds a
    /// "Cancel" button which simply closes the modal without taking action.
    cancellable: bool,

    /// Whether the modal should close.
    should_close: bool,
}

impl Modal {
    /// Creates a modal warning the user about their unsaved changes. Returns
    /// [`Some`] if the modal should close, which contains `true` only if the
    /// action should proceed. Otherwise, you should just close the modal.
    pub fn dirty(ctx: &Context) -> Option<bool> {
        let mut res = None;

        let modal = Modal::new("dirty")
            .with_heading("Discard Unsaved Changes")
            .with_body(
                "Are you sure you want to continue and discard your current changes? There's no going back after this!",
            )
            .show_with_extras(ctx, |ui| {
                if ui.button("✔ Yes").clicked() {
                    res = Some(true);
                }
            });

        if modal.should_close() {
            res = Some(false);
        }

        res
    }

    /// Creates a modal informing the user the item they requested could not be
    /// found. Returns `true` when the modal should close.
    pub fn not_found(ctx: &Context) -> bool {
        Modal::new("not_found")
            .with_heading("Not Found")
            .with_body("We couldn't find anything with that id.")
            .show(ctx)
            .should_close()
    }

    /// Creates a new modal with the given name. This sets both the internal
    /// name and the default heading.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            heading: RichText::from(name),
            body: None,
            cancellable: true,
            should_close: false,
        }
    }

    /// Sets the heading text
    pub fn with_heading(mut self, heading: impl Into<RichText>) -> Self {
        self.heading = heading.into();
        self
    }

    /// Sets the body text of this modal
    pub fn with_body(mut self, body: impl Into<WidgetText>) -> Self {
        self.body = Some(body.into()); // ...once told me, the world is gonna roll me
                                       // i ain't the sharpest tool in the shed...
        self
    }

    /// Sets whether the user can dismiss this modal without consequences. Adds
    /// a "Cancel" button which simply closes the modal without taking action.
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    /// Show the modal. To add more buttons, use [`self.show_with_extras`].
    pub fn show(self, ctx: &Context) -> Self {
        self.show_with_extras(ctx, |_| {})
    }

    /// Shows this modal with additional options for the user to select.
    pub fn show_with_extras(mut self, ctx: &Context, content: impl FnOnce(&mut Ui)) -> Self {
        let modal = egui::Modal::new(Id::new(self.name.to_owned())).show(ctx, |ui| {
            ui.set_width(PANEL_WIDTH_SMALL);

            ui.vertical_centered(|ui| {
                ui.heading(self.heading.to_owned());

                // add the body if we have it
                if let Some(body) = self.body.to_owned() {
                    ui.add_space(SPACE_MEDIUM);
                    ui.label(body);
                }
            });

            ui.separator();

            // Right align these buttons, order is also inverted
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // We should always be able to exit
                if self.cancellable && ui.button("🗙 Cancel").clicked() {
                    self.should_close = true;
                }

                content(ui);
            });
        });

        if modal.should_close() {
            self.should_close = true;
        }

        self
    }

    /// Whether the modal should close.
    pub fn should_close(&self) -> bool {
        self.should_close
    }
}
