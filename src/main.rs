use baan::Cmd;
use clap::Parser;
use color_eyre::eyre::{self, Context};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set log level
    #[arg(long, env = "BAAN_LOG_LEVEL", default_value = "baan=error")]
    log_level: String,
    #[command(subcommand)]
    command: Option<Cmd>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let Args { command, log_level } = Args::parse();
    let filter = EnvFilter::builder().parse(log_level)?;
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let cfg = baan::mk_runtime_config()?;

    let command = command.unwrap_or(Cmd::Open);
    let result = baan::Engine::run(command.clone(), cfg);
    std::process::exit(result.with_context(|| format!("failed to run command {command}"))?)
}
