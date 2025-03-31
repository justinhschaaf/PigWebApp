use eframe::emath::Align;
use egui::{Context, Id, Layout, RichText, Ui, WidgetText};

pub struct Modal {
    name: String,
    heading: RichText,
    body: Option<WidgetText>,
    cancellable: bool,
    should_close: bool,
}

impl Modal {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            heading: RichText::from(name),
            body: None,
            cancellable: true,
            should_close: false,
        }
    }

    pub fn with_heading(mut self, heading: impl Into<RichText>) -> Self {
        self.heading = heading.into();
        self
    }

    pub fn with_body(mut self, body: impl Into<WidgetText>) -> Self {
        self.body = Some(body.into()); // ...once told me, the world is gonna roll me
                                       // i ain't the sharpest tool in the shed...
        self
    }

    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    pub fn show(mut self, ctx: &Context) -> Self {
        self.show_with_extras(ctx, |_| {})
    }

    pub fn show_with_extras(mut self, ctx: &Context, content: impl FnOnce(&mut Ui)) -> Self {
        let modal = egui::Modal::new(Id::new(self.name.to_owned())).show(ctx, |ui| {
            ui.set_width(320.0);

            ui.vertical_centered(|ui| {
                ui.heading(self.heading.to_owned());

                // add the body if we have it
                if let Some(body) = self.body.to_owned() {
                    ui.add_space(8.0);
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

    pub fn should_close(&self) -> bool {
        self.should_close
    }
}
