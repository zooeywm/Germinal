use germinal::host::GerminalRuntimeHost;
use germinal_infra::window::GerminalWindowApp;

#[compio::main]
async fn main() {
    GerminalWindowApp::new(GerminalRuntimeHost::new()).run();
}
