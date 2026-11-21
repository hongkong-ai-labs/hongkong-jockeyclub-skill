//! `hkjc-mcp` — Hong Kong Jockey Club MCP server over stdio.

use anyhow::Result;

fn main() -> Result<()> {
    let client = hkjc::api::Client::new()?;
    hkjc::mcp::serve_blocking(client)
}
