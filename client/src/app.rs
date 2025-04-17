use crate::data::state::ClientState;
use crate::pages::layout::LayoutRender;
use crate::pages::pigpage::PigPageRender;
use crate::pages::{RenderPage, Routes};
use crate::style;
use egui::Context;
use matchit::Router;
use std::mem;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    /// Global app info
    state: ClientState,

    /// The current route
    route: Routes,

    /// The page router
    #[serde(skip)]
    router: Router<Routes>,

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
            router: Router::new(),
            layout: LayoutRender::default(),
            page_render: Box::new(PigPageRender::default()),
        }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show the global layout first
            self.layout.ui(ui, &mut self.state, None);

            // Then show the current route
            // TODO actually get the URL path
            if let Ok(route) = &self.router.at("/pigs") {
                // If the route has changed, update the state to reflect it
                if mem::discriminant(route.value) != mem::discriminant(&self.route) {
                    self.route = route.value.clone();
                    self.page_render = self.route.get_renderer();

                    // Tell the page renderer it's being opened
                    self.page_render.open(&mut self.state, Some(&route.params));
                }

                // Render the page
                self.page_render.ui(ui, &mut self.state, Some(&route.params))
            } else {
                // TODO 404 not found
            }
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
        res.page_render = res.route.get_renderer();
        res.page_render.open(&mut res.state, None);

        // Register routes with the router
        res.register_routes();

        res
    }

    pub fn register_routes(&mut self) {
        self.router.insert("/pigs/{*id}", Routes::Pigs).expect("Can't add route!");
        self.router.insert("/pigs", Routes::Pigs).expect("Can't add route!");
    }
}
