// ---------------------------------------------------------------------------
// main.rs — Binary entrypoint
//
// This is intentionally thin. All server logic lives in the library crate.
// Responsibilities:
//   - Parse CLI arguments
//   - Initialize logging/tracing
//   - Call websearch::serve() with the validated config
//
// If you need to add a new tool or change server behaviour, look in
// src/lib.rs (or add a module under src/).
// ---------------------------------------------------------------------------

use clap::Parser;
use websearch::config::Args;
use websearch::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .without_time()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::WARN.into())
                .from_env_lossy()
                .add_directive("websearch=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();
    let config = Config::from_args(args)?;

    websearch::serve(config).await?;

    Ok(())
}
