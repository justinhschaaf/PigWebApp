use eframe::emath::Align;
use egui::{Layout, Sense, Ui, WidgetText};
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

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
