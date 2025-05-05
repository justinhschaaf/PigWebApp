use egui::text::LayoutJob;
use egui::{Align, FontSelection, Galley, Layout, Sense, Ui, WidgetText};
use egui_extras::{Column, TableBody, TableBuilder, TableRow};
use std::sync::Arc;

pub mod modal;
pub mod style;

/// Starts a two-column table meant to show the data in a struct. Start with
/// this function, then call [`TableBuilder::body`] and use
/// [`add_properties_row`] for each property you want to show.
///
/// Example:
/// ```rust
/// use crate::pigweb_client::ui::{properties_list, add_properties_row};
///
/// pub fn ui(ui: &mut egui::Ui) {
///     properties_list(ui).body(|mut body| {
///         add_properties_row(&mut body, 40.0, "id", |ui| {
///             ui.label("1111111");
///         });
///     });
/// }
/// ```
pub fn properties_list(ui: &mut Ui) -> TableBuilder {
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .column(Column::initial(180.0))
        .column(Column::remainder())
        .cell_layout(Layout::left_to_right(Align::Center))
}

/// Adds a two-column row with a text label and value to a properties table.
/// Meant to be used in conjunction with [`properties_list`].
///
/// Example:
/// ```rust
/// use crate::pigweb_client::ui::{properties_list, add_properties_row};
///
/// pub fn ui(ui: &mut egui::Ui) {
///     properties_list(ui).body(|mut body| {
///         add_properties_row(&mut body, 40.0, "id", |ui| {
///             ui.label("1111111");
///         });
///     });
/// }
/// ```
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

/// Creates a list where each row can be selected or deselected and adds it to
/// the ui. The contents of each row are added by the add_row callback, which
/// should return whether the row was previously selected.
///
/// Returns [Some] if your selection should be updated, with the selection being
/// [Some] if a different item was selected or [None] if the current item should
/// be deselected.
///
/// You can check the implementations for [`crate::pages::pigpage::PigPageRender`]
/// or [`crate::pages::bulkpage::BulkPageRender`] for a usage example.
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
                        if selected {
                            // if this row is selected, deselect it
                            clicked = Some(None);
                        } else {
                            // change the selection
                            // ...and we clone the clone because of fucking course we do D:<
                            clicked = Some(Some(e.clone()));
                        }
                    }
                });
            });
        });

    clicked
}

/// A custom layouter which visually wraps text while still treating it as a
/// single line. Must be saved as a variable before applying it to a
/// [`egui::TextEdit::singleline`].
///
/// Example:
/// ```rust
/// use crate::pigweb_client::ui::wrapped_singleline_layouter;
///
/// let mut layouter = wrapped_singleline_layouter();
/// let te = egui::TextEdit::singleline(&mut "Value").desired_rows(4).layouter(&mut layouter);
/// ```
// Adapted from https://github.com/emilk/egui/blob/0db56dc9f1a8459b5b9376159fab7d7048b19b65/crates/egui/src/widgets/text_edit/builder.rs#L521-L529
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
