use std::{
    env, fmt,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::eyre::{self, Context};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static LOCAL_DEV: OnceLock<bool> = OnceLock::new();

fn is_local_dev() -> bool {
    *LOCAL_DEV.get_or_init(|| env::var("BAAN_LOCAL_DEV").is_ok())
}

/// Initialize runtime configuration from PATH.
///
/// If no configuration file exists, one will be created.
pub fn mk_runtime_config() -> eyre::Result<Config> {
    let config_dir = if is_local_dev() {
        let mut dir = env::current_dir()?;
        find_project_root(&mut dir, 0)?.join("target/config.toml")
    } else {
        if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME").map(PathBuf::from) {
            xdg_config_home.join("baan")
        } else {
            tracing::debug!("XDG_CONFIG_HOME not set, falling back to user's home directory");
            let Some(home_dir) = env::home_dir().map(PathBuf::from) else {
                eyre::bail!("home directory not set");
            };
            let config_home = home_dir.join(".config/baan");
            config_home
        }
    };

    fs::create_dir_all(&config_dir)?;
    let config = read_from_file_or_default(config_dir.join("config.toml"))?;
    Ok(config)
}

// TODO: buggy since we need to do a fs::create_all
fn read_from_file_or_default(file_path: PathBuf) -> eyre::Result<Config> {
    tracing::debug!(?file_path, "reading configuration from file");
    if file_path.is_file() {
        tracing::debug!(?file_path, "found existing config");
        let buf = fs::read_to_string(file_path)?;
        return toml::from_str::<Config>(&buf).wrap_err("failed to deserialize config");
    }

    tracing::debug!(?file_path, "using defaults");
    let config = Config::default();
    let ser = toml::to_string(&config).wrap_err("failed to serialize config")?;
    let mut out = File::create(&file_path)?;
    out.write_all(ser.as_bytes())?;
    Ok(config)
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub(crate) user: UserConfig,
    #[serde(default)]
    pub(crate) template: Template,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct UserConfig {
    #[serde(default)]
    pub(crate) editor: Option<String>,
    #[serde(default = "default_home_dir")]
    pub(crate) home_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Template {
    #[serde(rename = "headers")]
    pub(crate) root_headers: Vec<String>,
}

fn default_home_dir() -> PathBuf {
    if is_local_dev() {
        let mut dir = env::current_dir().unwrap();
        return find_project_root(&mut dir, 0).unwrap().join("target/notes");
    }
    PathBuf::from("~/.local/share/baan")
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            editor: None,
            home_dir: default_home_dir(),
        }
    }
}

impl Default for Template {
    fn default() -> Self {
        Self {
            root_headers: vec!["TODO".to_string(), "NOTES".to_string()],
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "( editor: {:?}, home_dir: {}, template: ( headers: {:?} ) )",
            self.user.editor,
            self.user.home_dir.display(),
            self.template.root_headers
        )
    }
}

/// Utility to automatically create a test configuration file when developing.
fn find_project_root(init_path: &mut PathBuf, depth: u8) -> eyre::Result<PathBuf> {
    static MAX_RECURSION_DEPTH: u8 = 8;
    eyre::ensure!(
        depth <= MAX_RECURSION_DEPTH,
        "failed to find matching project root"
    );

    let project_path = init_path.join("Cargo.toml");
    if project_path.exists() {
        return Ok(init_path.clone());
    }

    match init_path.pop() {
        true => find_project_root(init_path, depth + 1),
        false => eyre::bail!("path cannot be truncated to parent"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_defaults_when_no_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let non_existent = temp_file.path().with_extension("nonexistent");

        let config = read_from_file_or_default(non_existent).unwrap();

        assert_eq!(config.user.editor, None);
        assert_eq!(config.user.home_dir, PathBuf::from("~/.local/share/baan"));
        assert_eq!(config.template.root_headers, vec!["TODO", "NOTES"]);
    }

    #[test]
    fn test_full_config_deserializes() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[user]
editor = "nvim"
home_dir = "/custom/path"

[template]
headers = ["Custom Header", "Another One"]
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, Some("nvim".to_string()));
        assert_eq!(config.user.home_dir, PathBuf::from("/custom/path"));
        assert_eq!(
            config.template.root_headers,
            vec!["Custom Header", "Another One"]
        );
    }

    #[test]
    fn test_partial_config_editor_only() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[user]
editor = "vim"
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, Some("vim".to_string()));
        assert_eq!(config.user.home_dir, PathBuf::from("~/.local/share/baan")); // default
        assert_eq!(config.template.root_headers, vec!["TODO", "NOTES"]); // default
    }

    #[test]
    fn test_partial_config_home_dir_only() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[user]
home_dir = "/notes/here"
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, None); // default
        assert_eq!(config.user.home_dir, PathBuf::from("/notes/here"));
        assert_eq!(config.template.root_headers, vec!["TODO", "NOTES"]); // default
    }

    #[test]
    fn test_partial_config_template_only() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[template]
headers = ["Ideas", "Backlog"]
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, None); // default
        assert_eq!(config.user.home_dir, PathBuf::from("~/.local/share/baan")); // default
        assert_eq!(config.template.root_headers, vec!["Ideas", "Backlog"]);
    }

    #[test]
    fn test_empty_config_file() {
        let temp_file = NamedTempFile::new().unwrap();
        // File exists but is empty

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, None);
        assert_eq!(config.user.home_dir, PathBuf::from("~/.local/share/baan"));
        assert_eq!(config.template.root_headers, vec!["TODO", "NOTES"]);
    }

    #[test]
    fn test_user_section_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[template]
headers = ["Only Template"]
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        // UserConfig should use its defaults when section is missing
        assert_eq!(config.user.editor, None);
        assert_eq!(config.user.home_dir, PathBuf::from("~/.local/share/baan"));
        assert_eq!(config.template.root_headers, vec!["Only Template"]);
    }

    #[test]
    fn test_template_section_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[user]
editor = "emacs"
"#
        )
        .unwrap();

        let config = read_from_file_or_default(temp_file.path().to_path_buf()).unwrap();

        assert_eq!(config.user.editor, Some("emacs".to_string()));
        // Template should use its defaults when section is missing
        assert_eq!(config.template.root_headers, vec!["TODO", "NOTES"]);
    }

    #[test]
    fn test_invalid_toml_returns_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml [[[").unwrap();

        let result = read_from_file_or_default(temp_file.path().to_path_buf());

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("failed to deserialize config")
        );
    }
}
