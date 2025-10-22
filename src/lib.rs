mod config;

use std::{env, fmt, fs, io::Write, path::PathBuf, process};

use clap::Subcommand;
use color_eyre::eyre::{self, Context};
pub use config::mk_runtime_config;

use crate::config::{Config, Template, UserConfig};

pub struct Engine;

#[derive(Clone, Default, Debug, Subcommand)]
pub enum Cmd {
    /// Open `baan` on the latest `main.md` created.
    #[default]
    Open,
    /// Create a new `main.md` and archive the previous one
    New,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cmd::Open => write!(f, "baan-open"),
            Cmd::New => write!(f, "baan-new"),
        }
    }
}

impl Engine {
    pub fn run(cmd: Cmd, config: Config) -> eyre::Result<i32> {
        tracing::info!(%config, ?cmd, "running baan");
        let UserConfig { editor, home_dir } = config.user;
        let editor = if let Some(editor) = editor {
            editor
        } else {
            tracing::debug!("no editor found; falling back to $EDITOR");
            env::var("EDITOR").wrap_err("no editor configured (or found in PATH), ensure your environment is set-up correctly")?
        };
        eyre::ensure!(
            &home_dir.exists(),
            "NOTES home directory does not exist or is improperly configured"
        );
        let path_main = home_dir.join("main.md");
        tracing::debug!(file_path = ?path_main, "found main file path");
        let status = match cmd {
            Cmd::Open => {
                if !path_main.exists() {
                    create_main_file(&path_main, render_template(config.template))?
                }
                tracing::debug!("exec: {editor} {path_main:?}");
                process::Command::new(&editor)
                    .current_dir(&home_dir)
                    .args([&path_main])
                    .status()?
            }
            Cmd::New => {
                if !path_main.exists() {
                    create_main_file(&path_main, render_template(config.template))?;
                    tracing::debug!("exec: {editor} {path_main:?}");
                    process::Command::new(&editor)
                        .current_dir(&home_dir)
                        .args([&path_main])
                        .status()?
                } else {
                    let dt = chrono::Local::now();
                    const DT_FMT_PATH: &str = "%Y-%m-%d";

                    // Get today's date and format according to tmpl.
                    let dt_title = format!("{}", dt.format(DT_FMT_PATH));
                    fs::rename(
                        &path_main,
                        home_dir.join(format!("main-{dt_title}.archive.md")),
                    )?;

                    create_main_file(&path_main, render_template(config.template))?;
                    tracing::debug!("exec: {editor} {path_main:?}");
                    process::Command::new(&editor)
                        .current_dir(&home_dir)
                        .args([&path_main])
                        .status()?
                }
            }
        };

        Ok(status.code().unwrap_or(0))
    }
}

fn create_main_file(file_path: &PathBuf, buf: String) -> eyre::Result<()> {
    tracing::debug!(?file_path, "creating main notes file");
    let mut file = fs::File::create_new(&file_path)?;
    file.write_all(buf.as_bytes())?;
    tracing::debug!(?file_path, "created file with template");
    Ok(())
}

fn render_template(tmpl: Template) -> String {
    let mut buf = String::new();
    for header in tmpl.root_headers {
        buf.push_str("# ");
        buf.push_str(&header);
        buf.push_str("\n---\n");
        buf.push_str("\n\n\n\n");
    }
    buf
}
