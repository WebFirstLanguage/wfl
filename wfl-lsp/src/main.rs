use tower_lsp::{LspService, Server};
use wfl_lsp::WflLanguageServer;

fn print_help() {
    println!("wfl-lsp - WFL Language Server Protocol Implementation");
    println!();
    println!("USAGE:");
    println!("    wfl-lsp [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --version                     Show version information");
    println!("    --help                        Show this help message");
    println!("    --mcp                         Run as Model Context Protocol server");
    println!("    --stdio                       Use stdio communication (default)");
    println!("    --log-level <LEVEL>           Set logging level [default: info]");
    println!("                                 [possible values: error, warn, info, debug, trace]");
    println!("    --max-completion-items <NUM>  Maximum completion items [default: 100]");
    println!("    --hover-timeout <MS>          Hover timeout in milliseconds [default: 1000]");
    println!("    --tcp <PORT>                  Use TCP on specified port");
    println!();
    println!("EXAMPLES:");
    println!("    wfl-lsp                       Start LSP server with stdio");
    println!("    wfl-lsp --version             Show version information");
    println!("    wfl-lsp --mcp                 Start MCP server");
    println!("    wfl-lsp --log-level debug     Start with debug logging");
}

fn print_version() {
    println!("wfl-lsp {}", env!("CARGO_PKG_VERSION"));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Handle version and help flags first (no tokio runtime needed)
    for arg in &args[1..] {
        match arg.as_str() {
            "--version" | "-v" => {
                print_version();
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
    }

    // Only create tokio runtime when actually running a server
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async_main(args));
}

async fn async_main(args: Vec<String>) {
    // Parse remaining command-line arguments
    let mut mcp_mode = false;
    let mut _stdio_mode = true; // Default mode
    let mut _tcp_port: Option<u16> = None;
    let mut log_level = "info";
    let mut _max_completion_items = 100;
    let mut _hover_timeout = 1000;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp" => {
                mcp_mode = true;
                i += 1;
            }
            "--stdio" => {
                _stdio_mode = true;
                i += 1;
            }
            "--tcp" => {
                if i + 1 < args.len() {
                    if let Ok(port) = args[i + 1].parse::<u16>() {
                        _tcp_port = Some(port);
                        _stdio_mode = false;
                        i += 2;
                    } else {
                        eprintln!("Error: Invalid port number for --tcp");
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Error: --tcp requires a port number");
                    std::process::exit(1);
                }
            }
            "--log-level" => {
                if i + 1 < args.len() {
                    log_level = &args[i + 1];
                    i += 2;
                } else {
                    eprintln!("Error: --log-level requires a level");
                    std::process::exit(1);
                }
            }
            "--max-completion-items" => {
                if i + 1 < args.len() {
                    if let Ok(num) = args[i + 1].parse::<usize>() {
                        _max_completion_items = num;
                        i += 2;
                    } else {
                        eprintln!("Error: Invalid number for --max-completion-items");
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Error: --max-completion-items requires a number");
                    std::process::exit(1);
                }
            }
            "--hover-timeout" => {
                if i + 1 < args.len() {
                    if let Ok(ms) = args[i + 1].parse::<u64>() {
                        _hover_timeout = ms;
                        i += 2;
                    } else {
                        eprintln!("Error: Invalid timeout for --hover-timeout");
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Error: --hover-timeout requires a timeout in milliseconds");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
    }

    // Set up logging with the parsed log level
    let mut builder = env_logger::Builder::new();

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    } else {
        builder.parse_filters(log_level);
    }

    builder.init();

    // Run the appropriate server
    if mcp_mode {
        run_mcp_server().await;
    } else {
        run_lsp_server().await;
    }
}

/// Run the Language Server Protocol server
async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(WflLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the Model Context Protocol server
async fn run_mcp_server() {
    if let Err(e) = wfl_lsp::mcp_server::run_server().await {
        eprintln!("[MCP] Error running MCP server: {}", e);
        std::process::exit(1);
    }
}
