mod server;
mod tools;

use rmcp::ServiceExt;
use weave_core::git::find_repo_root;
use weave_crdt::EntityStateDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log to stderr so it doesn't interfere with MCP stdio transport
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("weave_mcp=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let repo_root = find_repo_root()?;
    let state_path = repo_root.join(".weave").join("state.automerge");
    let state = EntityStateDoc::open(&state_path)?;

    let server = server::WeaveServer::new(state, repo_root);

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
