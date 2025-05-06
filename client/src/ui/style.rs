use egui::epaint::text::{FontInsert, InsertFontFamily};
use egui::{Color32, FontData};
use egui_colors::tokens::ThemeColor;
use egui_colors::Colorix;

/// Theme color for accepted pig names in bulk import
pub const COLOR_ACCEPTED: Color32 = Color32::from_rgb(85, 187, 85);

/// Theme color for rejected pig names in bulk import
pub const COLOR_REJECTED: Color32 = Color32::from_rgb(221, 51, 68);

/// Primary font used by the application.
const FONT_MAIN: &[u8] = include_bytes!("../../data/ReadexPro-Regular.ttf");

/// Monospace font used for code blocks.
const FONT_MONO: &[u8] = include_bytes!("../../data/IBMPlexMono-Medium.ttf");

/// Smallest UI panel size, also determines modal width.
pub const PANEL_WIDTH_SMALL: f32 = 320.0;

/// Standard UI panel size
pub const PANEL_WIDTH_MEDIUM: f32 = 540.0;

/// Largest UI panel size, should really have nothing next to it at this scale
pub const PANEL_WIDTH_LARGE: f32 = 960.0;

/// Smallest spacer size
pub const SPACE_SMALL: f32 = 4.0;

/// Standard spacer size
pub const SPACE_MEDIUM: f32 = 8.0;

/// Large spacer size, works well with a spaced header right under it
pub const SPACE_LARGE: f32 = 88.0;

/// Smallest width for a column in a table
/// Large should be [`egui_extras::Column::remainder`]
pub const TABLE_COLUMN_WIDTH_SMALL: f32 = 200.0;

/// Medium width for a column in a table.
/// Large should be [`egui_extras::Column::remainder`]
pub const TABLE_COLUMN_WIDTH_MEDIUM: f32 = 280.0;

/// Smallest height for a row in a table
pub const TABLE_ROW_HEIGHT_SMALL: f32 = 18.0;

/// Largest height for a row in a table
pub const TABLE_ROW_HEIGHT_LARGE: f32 = 36.0;

/// The primary theme color for the application, used for text and backgrounds.
/// Greyscale.
const THEME_PRIMARY: ThemeColor = ThemeColor::Gray;

/// The secondary theme color, accenting the primary. Pink.
const THEME_ACCENT: ThemeColor = ThemeColor::Custom([255, 137, 172]);

/// The theme scale used by egui_colors.
const THEME: [ThemeColor; 12] = [
    THEME_PRIMARY,
    THEME_PRIMARY,
    THEME_ACCENT,
    THEME_ACCENT,
    THEME_ACCENT,
    THEME_PRIMARY,
    THEME_PRIMARY,
    THEME_ACCENT,
    THEME_ACCENT,
    THEME_ACCENT,
    THEME_PRIMARY,
    THEME_PRIMARY,
];

/// The standard format timestamps should be displayed in. See
/// [`chrono::format::strftime`] for options.
pub const TIME_FMT: &'static str = "%a, %b %e %Y %T";

/// Sets global styles on the given CreationContext and initializes Colorix to
/// manage it. Returns the Colorix instance
pub fn set_styles(cc: &eframe::CreationContext<'_>) -> Colorix {
    // This is also where you can customize the look and feel of egui using
    // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

    // Set zoom to 110% so everything is slightly easier to see
    cc.egui_ctx.set_zoom_factor(1.1);

    // Initialize Colorix with the global ctx and our theme. We could use
    // Colorix::local_from_style without the context, but we would also have
    // to know in advance if dark mode is enabled. It's easier to just let
    // the widget and egui itself worry about that.
    let colorix = Colorix::global(&cc.egui_ctx, THEME);

    // Add fonts https://github.com/emilk/egui/blob/0db56dc9f1a8459b5b9376159fab7d7048b19b65/examples/custom_font/src/main.rs
    cc.egui_ctx.add_font(FontInsert::new(
        "readex-pro",
        FontData::from_static(FONT_MAIN),
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));

    cc.egui_ctx.add_font(FontInsert::new(
        "ibm-plex-mono",
        FontData::from_static(FONT_MONO),
        vec![InsertFontFamily {
            family: egui::FontFamily::Monospace,
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));

    colorix
}
