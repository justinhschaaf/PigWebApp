use crate::data::api::ApiError;
use crate::data::state::ClientState;
use crate::pages::layout::Layout;
use crate::pages::{pigpage, Page, PageImpl};
use egui::Context;
use egui_router::EguiRouter;
use tokio::sync::mpsc;

pub struct PigWebClient {
    state: ClientState,
    router: EguiRouter<ClientState>,
    route_receiver: mpsc::Receiver<Page>,
}

impl PigWebClient {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, route_receiver) = mpsc::channel(8);
        let mut state = ClientState::new(cc, tx);
        let router = get_router(&mut state);

        Self { state, router, route_receiver }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Determine if the route changed
        if let Some(page) = self.route_receiver.try_recv().ok() {
            let route = page.get_route();
            self.state.page = page;
            if let Err(err) = self.router.navigate(&mut self.state, route) {
                self.state.display_error = Some(ApiError::new(err.to_string()));
            }
        }

        // Defer to the router to render everything
        // TODO if this doesn't work, we'll have to manually create the Ui panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show the global layout first
            Layout::ui(ui, &mut self.state);

            // Then show the current route
            self.router.ui(ui, &mut self.state)
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.state.save(storage)
    }
}

pub fn get_router(state: &mut ClientState) -> EguiRouter<ClientState> {
    // https://github.com/lucasmerlin/hello_egui/blob/b1093eff0361e639b1567bf34d4b8c136cebf141/fancy-example/src/routes.rs#L38-L55
    EguiRouter::builder()
        .history({
            // if you try to return the history directly instead of setting a variable, it *will* complain loudly
            #[cfg(target_arch = "wasm32")]
            let history = egui_router::history::BrowserHistory::new(Some("/#".to_string()));
            #[cfg(not(target_arch = "wasm32"))]
            let history = egui_router::history::DefaultHistory::default();
            history
        })
        .default_path("/")
        .route("/pigs/{*slug}", pigpage::request)
        .route("/pigs", pigpage::request)
        .route_redirect("/", "/pigs")
        .build(state)
}
