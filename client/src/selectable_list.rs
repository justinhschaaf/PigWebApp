use eframe::emath::Align;
use egui::{Layout, Sense, Ui};
use egui_extras::{Column, TableRow};

pub struct SelectableList {}

impl SelectableList {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show<T: Clone>(
        self,
        ui: &mut Ui,
        items: &Vec<T>,
        mut add_row: impl FnMut(&mut TableRow, &T) -> bool,
    ) -> Option<Option<T>> {
        let mut clicked = None;

        egui_extras::TableBuilder::new(ui)
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
}
