// Simple example MCP client for WFL
// Demonstrates how to interact with wfl-lsp in MCP mode

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting WFL MCP client example...\n");

    // Spawn wfl-lsp in MCP mode
    let mut child = Command::new("wfl-lsp")
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let reader = BufReader::new(stdout);

    // Helper function to send request and read response
    let mut send_request = |request: Value| -> Result<Value, Box<dyn std::error::Error>> {
        let request_json = serde_json::to_string(&request)?;
        writeln!(stdin, "{}", request_json)?;
        stdin.flush()?;

        // Read response line
        for line in reader.by_ref().lines() {
            let line = line?;
            if !line.trim().is_empty() {
                let response: Value = serde_json::from_str(&line)?;
                return Ok(response);
            }
        }
        Err("No response received".into())
    };

    // Example 1: Initialize
    println!("1. Initializing MCP server...");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let init_response = send_request(init_request)?;
    println!("Server version: {}\n", init_response["result"]["serverInfo"]["version"]);

    // Example 2: List tools
    println!("2. Listing available tools...");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let tools_response = send_request(tools_request)?;
    let tools = tools_response["result"]["tools"].as_array().unwrap();
    println!("Available tools: {}", tools.len());
    for tool in tools {
        println!("  - {}: {}", tool["name"], tool["description"]);
    }
    println!();

    // Example 3: Parse WFL code
    println!("3. Parsing WFL code...");
    let parse_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "parse_wfl",
            "arguments": {
                "source": "store x as 5\ndisplay x",
                "include_positions": false
            }
        }
    });

    let parse_response = send_request(parse_request)?;
    let parse_result: Value = serde_json::from_str(
        parse_response["result"]["content"][0]["text"].as_str().unwrap()
    )?;
    println!("Parse result: {}", parse_result["message"]);
    println!("Statement count: {}\n", parse_result["statement_count"]);

    // Example 4: Analyze code with error
    println!("4. Analyzing code with error...");
    let analyze_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "analyze_wfl",
            "arguments": {
                "source": "store x as 5\ndisplay y"
            }
        }
    });

    let analyze_response = send_request(analyze_request)?;
    let analyze_result: Value = serde_json::from_str(
        analyze_response["result"]["content"][0]["text"].as_str().unwrap()
    )?;
    println!("Analysis result: {}", analyze_result["message"]);
    if let Some(diagnostics) = analyze_result["diagnostics"].as_array() {
        for diag in diagnostics {
            println!("  Error: {}", diag["message"]);
        }
    }
    println!();

    // Example 5: Get completions
    println!("5. Getting code completions...");
    let completion_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "get_completions",
            "arguments": {
                "source": "store ",
                "line": 0,
                "column": 6
            }
        }
    });

    let completion_response = send_request(completion_request)?;
    let completion_result: Value = serde_json::from_str(
        completion_response["result"]["content"][0]["text"].as_str().unwrap()
    )?;
    println!("Completion count: {}", completion_result["completion_count"]);
    println!("First few completions:");
    if let Some(completions) = completion_result["completions"].as_array() {
        for (i, comp) in completions.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, comp["label"]);
        }
    }
    println!();

    // Example 6: List resources
    println!("6. Listing resources...");
    let resources_request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "resources/list"
    });

    let resources_response = send_request(resources_request)?;
    let resources = resources_response["result"]["resources"].as_array().unwrap();
    println!("Available resources: {}", resources.len());
    for resource in resources {
        println!("  - {}: {}", resource["uri"], resource["description"]);
    }
    println!();

    println!("========================================");
    println!("All examples completed successfully!");
    println!("========================================");

    Ok(())
}
