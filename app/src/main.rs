use germinal::app_deps::AppDeps;
use germinal::host::GerminalRuntimeHost;
use germinal_infra::window::GerminalWindowApp;

#[compio::main]
async fn main() {
    let deps = AppDeps::new();

    GerminalWindowApp::new(GerminalRuntimeHost::new(deps))
        .run()
        .await;
}
