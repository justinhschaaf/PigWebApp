use egui::text::LayoutJob;
use egui::{Align, FontSelection, Galley, Layout, Sense, Ui, WidgetText};
use egui_extras::{Column, TableBody, TableBuilder, TableRow};
use std::sync::Arc;

pub mod modal;
pub mod style;

pub fn properties_list(ui: &mut Ui) -> TableBuilder {
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .column(Column::initial(180.0))
        .column(Column::remainder())
        .cell_layout(Layout::left_to_right(Align::Center))
}

pub fn add_properties_row(
    body: &mut TableBody<'_>,
    height: f32,
    label: impl Into<WidgetText>,
    add_value: impl FnOnce(&mut Ui),
) {
    body.row(height, |mut row| {
        row.col(|ui| {
            ui.label(label);
        });

        row.col(add_value);
    });
}

pub fn selectable_list<T: Clone>(
    ui: &mut Ui,
    items: &Vec<T>,
    mut add_row: impl FnMut(&mut TableRow, &T) -> bool,
) -> Option<Option<T>> {
    let mut clicked = None;

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .column(Column::remainder())
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .body(|mut body| {
            // This means we don't have to clone the list every frame
            items.iter().for_each(|e| {
                body.row(18.0, |mut row| {
                    let selected = add_row(&mut row, e);

                    if row.response().clicked() {
                        // warn about unsaved changes, else JUST DO IT
                        if selected {
                            clicked = Some(None);
                        } else {
                            // ...and we clone the clone because of fucking course we do D:<
                            clicked = Some(Some(e.clone()));
                        }
                    }
                });
            });
        });

    clicked
}

// Adapted from https://github.com/emilk/egui/blob/0db56dc9f1a8459b5b9376159fab7d7048b19b65/crates/egui/src/widgets/text_edit/builder.rs#L521-L529
// We need to write a custom layouter for this so we can visually
// wrap the text while still treating it as a single line
pub fn wrapped_singleline_layouter() -> impl FnMut(&Ui, &str, f32) -> Arc<Galley> {
    |ui: &Ui, text: &str, wrap_width: f32| {
        let job = LayoutJob::simple(
            text.to_owned(),
            FontSelection::default().resolve(ui.style()),
            ui.visuals().override_text_color.unwrap_or_else(|| ui.visuals().widgets.inactive.text_color()),
            wrap_width,
        );
        ui.fonts(|f| f.layout_job(job))
    }
}
