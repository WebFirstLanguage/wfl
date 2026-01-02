use tower_lsp::{LspService, Server};
use wfl_lsp::WflLanguageServer;

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if --mcp flag is present
    match args.get(1).map(|s| s.as_str()) {
        Some("--mcp") => {
            // Run MCP server
            run_mcp_server().await;
        }
        _ => {
            // Default: Run LSP server (backward compatible)
            run_lsp_server().await;
        }
    }
}

/// Run the Language Server Protocol server
async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| WflLanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the Model Context Protocol server
async fn run_mcp_server() {
    if let Err(e) = wfl_lsp::mcp_server::run_server().await {
        eprintln!("[MCP] Error running MCP server: {}", e);
        std::process::exit(1);
    }
}
