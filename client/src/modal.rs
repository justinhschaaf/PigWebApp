use eframe::emath::Align;
use egui::{Context, Id, Layout, RichText, Ui, WidgetText};

pub struct Modal {
    should_close: bool,
}

impl Modal {
    pub fn new(ctx: &Context, name: &str, heading: impl Into<RichText>, body: impl Into<WidgetText>) -> Self {
        Self::new_with_extras(ctx, name, heading, body, |_| {})
    }

    pub fn new_with_extras(
        ctx: &Context,
        name: &str,
        heading: impl Into<RichText>,
        body: impl Into<WidgetText>,
        content: impl FnOnce(&mut Ui),
    ) -> Self {
        let mut should_close = false;

        let modal = egui::Modal::new(Id::new(name)).show(ctx, |ui| {
            ui.set_width(320.0);

            ui.vertical_centered(|ui| {
                ui.heading(heading);
                ui.add_space(8.0);
                ui.label(body);
            });

            ui.separator();

            // Right align these buttons, order is also inverted
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // We should always be able to exit
                if ui.button("🗙 Cancel").clicked() {
                    should_close = true;
                }

                content(ui);
            });
        });

        if modal.should_close() {
            should_close = true;
        }

        Self { should_close }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }
}
