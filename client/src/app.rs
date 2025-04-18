use crate::data::state::ClientState;
use crate::pages::layout::LayoutRender;
use crate::pages::pigpage::PigPageRender;
use crate::pages::{RenderPage, Routes};
use crate::style;
use eframe::WebInfo;
use egui::Context;
use urlable::{parse_url, ParsedURL};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    /// Global app info
    state: ClientState,

    /// The current route
    route: Routes,

    #[serde(skip)]
    /// The layout renderer
    layout: LayoutRender,

    /// The currently open page renderer
    #[serde(skip)]
    page_render: Box<dyn RenderPage>,
}

impl Default for PigWebClient {
    fn default() -> Self {
        Self {
            state: ClientState::default(),
            route: Routes::Pigs,
            layout: LayoutRender::default(),
            page_render: Box::new(PigPageRender::default()),
        }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // get the current url
            let mut url = Self::url_from_webinfo(&frame.info().web_info);

            // show the global layout first
            self.layout.ui(ui, &mut self.state, &url);

            // get the route from the url
            let route = match url.pathname.as_str() {
                "/pigs" | "/" => Routes::Pigs,
                _ => Routes::NotFound,
            };

            // If the route has changed, update the state to reflect it
            if route != self.route {
                self.route = route;
                self.page_render = self.route.get_renderer();

                // Tell the page renderer it's being opened
                self.page_render.open(&mut self.state, &url);
            }

            // Render the page
            self.page_render.ui(ui, &mut self.state, &url)
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::APP_KEY, &self);
    }
}

impl PigWebClient {
    /// The key used to access cached data for the Pig Web App
    pub const APP_KEY: &'static str = "pigweb";

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any) or default state data
        // Note that you must enable the `persistence` feature for this to work.
        let mut res: PigWebClient =
            cc.storage.and_then(|storage| eframe::get_value(storage, Self::APP_KEY)).unwrap_or_default();

        // Setup styles
        res.state.colorix = style::set_styles(cc);

        // Get the updated renderer, in case a different page was loaded
        // then send the open command
        let url = Self::url_from_webinfo(&cc.integration_info.web_info);
        res.page_render = res.route.get_renderer();
        res.page_render.open(&mut res.state, &url);

        res
    }

    fn url_from_webinfo(info: &WebInfo) -> ParsedURL {
        let mut url = parse_url(info.location.url.as_str());
        url.hash = info.location.hash.to_owned();
        url
    }
}
