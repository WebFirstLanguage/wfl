use tower_lsp::{LspService, Server};
use wfl_lsp::WflLanguageServer;

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| WflLanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
