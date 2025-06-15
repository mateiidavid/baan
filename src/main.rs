use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process,
    sync::LazyLock,
};

use chrono::{DateTime, Local};
use clap::{command, Parser, Subcommand};
use color_eyre::eyre::{self, Context, OptionExt};
use tracing::info;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Home directory where all notes are stored.
    // TODO: not sure I want this exposed but we'll see
    #[arg(short = 'p', long = "home-path", env = "BAAN_HOME_DIR")]
    home_dir: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}

// Note: static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
static PROJECT_MARKERS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    vec![
        ".git",
        "flake.nix",
        "Makefile",
        "makefile",
        "CMakeLists.txt",
        "Cargo.toml",
        "go.mod",
        "package.json",
        "pyproject.toml",
    ]
});

#[derive(Clone, Debug, Subcommand)]
enum Cmd {
    /// Used to initialise `baan` in the current working directory.
    ///
    /// `baan` will create a new project in the designated notes home directory
    /// and open a features page based on the current git status.
    // TODO: default if no git? treat it as main branch or something
    // TODO: find a way to make the default, I want `baan .` to call Init ( . ), see what
    // needs to happen to make it
    Init {
        #[arg(default_value = ".")]
        project_path: PathBuf,
    },
    /// Used to open today's TODO note.
    ///
    /// Creates a new note for the day if one doesn't exist.
    Today,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let Args { home_dir, command } = Args::parse();
    let result = match &command {
        Cmd::Init { project_path } => handle_init(home_dir, project_path.to_path_buf()),
        Cmd::Today => handle_today(home_dir),
    };

    std::process::exit(result.with_context(|| format!("failed to run command {command}"))?)
}

impl std::fmt::Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Init { project_path } => write!(f, "Init ({project_path:?})"),
            Cmd::Today => f.write_str("Today"),
        }
    }
}

fn handle_init(home_dir: PathBuf, init_path: PathBuf) -> eyre::Result<i32> {
    eyre::ensure!(init_path.is_dir(), "project path must be a directory");
    // 3. Figure out branch if it is a directory
    // 4. Open file
    let file_path = open_init(&home_dir, init_path)?;
    let status = process::Command::new("hx")
        .current_dir(home_dir)
        .arg(file_path)
        .status()?;

    Ok(status.code().unwrap_or(0))
}

fn open_init(home_path: &PathBuf, mut init_path: PathBuf) -> eyre::Result<PathBuf> {
    let project = find_project_root(&mut init_path, 0)?;
    // TODO: maybe improve error handling here
    assert!(fs::exists(&home_path).expect("home directory should exist"));
    let mut file_path = project.resolve_notes_path(&home_path)?;
    tracing::info!(?file_path, "resolved path");
    if file_path.exists() {
        return Ok(file_path);
    }
    // open today's file.
    let mut file = fs::File::create_new(&file_path)?;
    // TODO: if you feel like it, handle interrupted
    match file.write_all("# TODO\n\n- [ ]".as_bytes()) {
        Ok(_) => {
            let src = file_path.clone();
            // pop first ext.
            file_path.set_extension("");
            file_path.set_extension("md");
            fs::rename(src, &file_path)?;
            tracing::info!(?file_path, "initialised new project file");
        }
        Err(error) => {
            tracing::error!(%error, "failed to write today's daily; cleaning-up");
            fs::remove_file(file_path)
                .wrap_err_with(|| "failed to clean-up tmp path {file_path:?}")?;
            std::process::exit(100);
        }
    }
    Ok(file_path)
}

#[derive(Debug)]
struct ProjectRoot {
    name: String,
    path: PathBuf,
    marker: MarkerType,
}

impl std::fmt::Display for ProjectRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProjectRoot ({}): '{}':{:?}",
            self.name,
            self.path.display(),
            self.marker
        )
    }
}

#[derive(Debug)]
enum MarkerType {
    Git,
    BuildSystem,
    HomeDir,
}

impl ProjectRoot {
    fn try_new(path: PathBuf, marker: MarkerType) -> eyre::Result<Self> {
        use crate::MarkerType::*;

        let name = match &marker {
            Git | BuildSystem => path
                .parent()
                .ok_or_eyre(format!("path '{path:?} has no parent even if it should"))?
                .canonicalize()?
                .file_name()
                .ok_or_eyre(format!("unsupported project root path '{path:?}'"))?
                .to_string_lossy()
                .to_string(),
            HomeDir => String::from("home"),
        };
        let path = path.canonicalize()?;
        Ok(Self { name, path, marker })
    }

    // So,
    // usually we want /notes/...
    // when there is no build project, everything goes into notes/braindump.md; ez
    // when there is a build project, everything goes into notes/<name>/braindump.md
    // when there is a git, everything goes into notes/<name>/<branch>.md
    fn resolve_notes_path(&self, home_dir: &PathBuf) -> eyre::Result<PathBuf> {
        match self.marker {
            MarkerType::Git => self.resolve_feature_path(home_dir),
            MarkerType::BuildSystem => self.resolve_build_path(home_dir),
            MarkerType::HomeDir => Ok(home_dir.join("braindump").with_extension("md")),
        }
    }

    fn resolve_build_path(&self, home_dir: &PathBuf) -> eyre::Result<PathBuf> {
        tracing::info!(project = %self, "resolving build path");
        let project_root = self.resolve_project_root(home_dir)?;
        Ok(project_root.join("braindump").with_extension("md"))
    }
    // when we have git
    fn resolve_feature_path(&self, home_dir: &PathBuf) -> eyre::Result<PathBuf> {
        tracing::info!(project = %self, "resolving feature path");
        let head = self.path.join("HEAD");
        let mut head_file = fs::OpenOptions::new().read(true).open(head)?;
        let mut head_content = String::new();
        let n = head_file.read_to_string(&mut head_content)?;
        tracing::info!(project = %self, bytes_read = %n, "read HEAD from git directory");
        let feat_name = head_content
            .strip_prefix("ref: refs/heads/")
            .map(|s| s.trim().to_string())
            .or_else(|| {
                // Detached HEAD
                let hash = head_content.trim();
                Some(format!("HEAD:{}", &hash[..8.min(hash.len())]))
            })
            .map(PathBuf::from)
            .ok_or_eyre("failed to read branch name from HEAD")?;

        let project_root = self.resolve_project_root(home_dir)?;
        Ok(project_root.join(feat_name).with_extension("md"))
    }

    fn resolve_project_root(&self, home_dir: &PathBuf) -> eyre::Result<PathBuf> {
        tracing::info!(project = %self, "resolving project root");
        let root_path = home_dir.join(&self.name);
        if root_path.exists() {
            Ok(root_path)
        } else {
            fs::create_dir(&root_path)?;
            Ok(root_path)
        }
    }
}

const MAX_RECURSION_DEPTH: u8 = 8;

fn find_project_root(init_path: &mut PathBuf, depth: u8) -> eyre::Result<ProjectRoot> {
    if depth > MAX_RECURSION_DEPTH {
        let home_dir = find_home_dir().ok_or_eyre("failed to find any matching directories")?;
        return ProjectRoot::try_new(home_dir, MarkerType::HomeDir);
    }

    for marker in &*PROJECT_MARKERS {
        let marker_path = init_path.join(marker);
        if marker_path.exists() {
            let marker_type = if *marker == ".git" {
                MarkerType::Git
            } else {
                MarkerType::BuildSystem
            };
            return ProjectRoot::try_new(marker_path, marker_type);
        }
    }

    match init_path.pop() {
        true => find_project_root(init_path, depth + 1),
        false => eyre::bail!("path cannot be truncated to parent"),
    }
}

#[cfg(not(windows))]
fn find_home_dir() -> Option<PathBuf> {
    std::env::home_dir()
}

#[cfg(windows)]
fn find_home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFIE").ok().map(PathBuf::from)
}

// Why we don't want zombies? A zombie process is going to exist until reaped,
// e.g. parent gets it status so it can see if it failed or something similar.
//
// Parent -> Child, if Parent dies as soon as Child spawns, if Child dies, Child is zombie. Nobody to reap;
// we do a linux classic double fork to avoid this.
//
// Parent -> Child -> Grandchild
// Child -> dies instantly. (1)
// Parent -> reaps child (2)
// Grandchild -> is now reparented to init (2)
// Grandchild -> exits, reaped by init (3)
//
// We will do none of those, we just wait
fn handle_today(home_dir: PathBuf) -> eyre::Result<i32> {
    tracing::info!(home_path = ?home_dir, "handling today's daily note");
    let file_path = open_today(&home_dir)?;
    let status = process::Command::new("hx")
        .current_dir(home_dir)
        .arg(file_path)
        .status()?;

    Ok(status.code().unwrap_or(0))
}

fn open_today(home_path: &PathBuf) -> eyre::Result<PathBuf> {
    // TODO: maybe improve error handling here
    assert!(fs::exists(&home_path).expect("home directory should exist"));
    let today_dir = home_path.join("daily");
    let should_create = !fs::exists(&today_dir).wrap_err_with(|| "failed to read '{today_dir}'")?;
    if should_create {
        info!(dir_name = ?today_dir, home_path = ?home_path, "daily notes path does not exist");
        fs::create_dir(&today_dir).wrap_err_with(|| "failed to create '{today_dir}'")?;
        info!(dir_name = ?today_dir, home_path = ?home_path, "created directory for daily notes");
    }

    let dt = chrono::Local::now();
    let (mut file_path, exists) = resolve_path(&today_dir, &dt)?;
    if exists {
        tracing::info!(?file_path, "found existing daily file");
        return Ok(file_path);
    }

    // open today's file.
    let mut file = fs::File::create_new(&file_path)?;
    // TODO: if you feel like it, handle interrupted
    match file.write_all("# TODO\n\n- [ ]".as_bytes()) {
        Ok(_) => {
            let src = file_path.clone();
            // pop first ext.
            file_path.set_extension("");
            file_path.set_extension("md");
            fs::rename(src, &file_path)?;
            tracing::info!(?file_path, "initialised new daily file");
        }
        Err(error) => {
            tracing::error!(%error, "failed to write today's daily; cleaning-up");
            fs::remove_file(file_path)
                .wrap_err_with(|| "failed to clean-up tmp path {file_path:?}")?;
            std::process::exit(100);
        }
    }

    Ok(file_path)
}

/// Resolves path for today's daily
fn resolve_path(dir: &PathBuf, dt: &DateTime<Local>) -> eyre::Result<(PathBuf, bool)> {
    const DT_FMT_PATH: &str = "%d-%m-%y";

    // Get today's date and format according to tmpl.
    let dt_title = format!("{}", dt.format(DT_FMT_PATH));

    // First, try to see if file actually exists, if it does, return it
    let mut file_path = dir.join(dt_title);
    file_path.set_extension("md");
    // TODO: I don't like this much but oh well
    // when the file exists, return path & marker
    if fs::exists(&file_path)? {
        return Ok((file_path, true));
    }

    file_path.set_extension("");
    file_path.set_extension("tmp.md");
    Ok((file_path, false))
}
