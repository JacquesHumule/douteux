use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use pager::Pager;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    url: String,
    token: String,
}

fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not find config directory")?;
    Ok(base.join("sus").join("config.json"))
}

fn load_config() -> Result<Config> {
    let path = config_path()?;
    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "config not found at {}\nRun `sus config set` first",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).context("invalid config file")
}

fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, serde_json::to_string_pretty(config)?)
        .with_context(|| format!("failed to write config to {}", path.display()))
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "sus", about = "douteux URL shadifier CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage stored configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Shorten a URL
    Shorten {
        /// The URL to shorten
        url: String,
        /// Slug shape: "shorten", "shorten:12", "shady", "shady:movie", "shady:crack", "shady:pup", or "shady:tape"
        #[arg(long)]
        pattern: Option<String>,
    },
    /// Upload a file and get a short download link
    Upload {
        /// Path to the file to upload
        file: PathBuf,
        /// Slug shape: "shorten", "shorten:12", "shady", "shady:movie", "shady:crack", "shady:pup", or "shady:tape"
        #[arg(long)]
        pattern: Option<String>,
    },
    /// Delete a slug
    Delete {
        /// The slug to delete
        slug: String,
    },
    /// List all slugs
    List,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set the API URL and bearer token
    Set {
        /// Base URL of the douteux worker (e.g. https://douteux.example.com)
        #[arg(long)]
        url: String,
        /// Bearer token for authenticated routes
        #[arg(long)]
        token: String,
    },
    /// Print the current configuration
    Show,
}

// ---------------------------------------------------------------------------
// API helpers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SlugResponse {
    slug: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ListEntryKind {
    Url { url: String },
    File { r2_key: String },
    Unknown,
}

#[derive(Deserialize)]
struct ListEntry {
    slug: String,
    #[serde(flatten)]
    kind: ListEntryKind,
}

fn api_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn shorten(config: &Config, url: &str, pattern: Option<&str>) -> Result<String> {
    let client = Client::new();
    let res = client
        .post(api_url(&config.url, "/"))
        .bearer_auth(&config.token)
        .json(&serde_json::json!({ "url": url, "pattern": pattern }))
        .send()
        .context("request failed")?;

    if !res.status().is_success() {
        bail!(
            "server returned {}: {}",
            res.status(),
            res.text().unwrap_or_default()
        );
    }

    let body: SlugResponse = res.json().context("invalid response")?;
    Ok(body.slug)
}

fn upload(config: &Config, file: &PathBuf, pattern: Option<&str>) -> Result<String> {
    use std::io::Seek;

    let filename = file
        .file_name()
        .context("invalid file path")?
        .to_string_lossy()
        .to_string();
    let mime = mime_guess(file);

    let mut handle =
        fs::File::open(file).with_context(|| format!("could not open {}", file.display()))?;

    // Content-address the upload: hash the file, then rewind so the same handle
    // can be streamed as the request body.
    let sha256 =
        sha256_hex(&mut handle).with_context(|| format!("could not read {}", file.display()))?;
    handle.rewind().context("could not rewind file")?;

    // The worker streams the raw body straight into R2, so `pattern`, the
    // original file name and the hash travel as query parameters. reqwest
    // derives Content-Length from the file handle, which the worker requires
    // to size the R2 upload.
    let mut query: Vec<(&str, &str)> = vec![("filename", &filename), ("sha256", &sha256)];
    if let Some(pattern) = pattern {
        query.push(("pattern", pattern));
    }

    let client = Client::new();
    let res = client
        .post(api_url(&config.url, "/"))
        .bearer_auth(&config.token)
        .query(&query)
        .header(reqwest::header::CONTENT_TYPE, mime)
        .body(handle)
        .send()
        .context("request failed")?;

    if !res.status().is_success() {
        bail!(
            "server returned {}: {}",
            res.status(),
            res.text().unwrap_or_default()
        );
    }

    let body: SlugResponse = res.json().context("invalid response")?;
    Ok(body.slug)
}

fn list(config: &Config) -> Result<Vec<ListEntry>> {
    let client = Client::new();
    let res = client
        .get(api_url(&config.url, "/"))
        .bearer_auth(&config.token)
        .send()
        .context("request failed")?;

    if !res.status().is_success() {
        bail!(
            "server returned {}: {}",
            res.status(),
            res.text().unwrap_or_default()
        );
    }

    res.json().context("invalid response")
}

fn delete(config: &Config, slug: &str) -> Result<()> {
    let client = Client::new();
    let res = client
        .delete(api_url(&config.url, slug))
        .bearer_auth(&config.token)
        .send()
        .context("request failed")?;

    if !res.status().is_success() {
        bail!(
            "server returned {}: {}",
            res.status(),
            res.text().unwrap_or_default()
        );
    }

    Ok(())
}

/// Streaming lowercase-hex SHA-256 of a reader.
fn sha256_hex(reader: &mut impl std::io::Read) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        }))
}

/// Guess MIME type from file extension, falling back to octet-stream.
fn mime_guess(path: &PathBuf) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Config { action } => match action {
            ConfigAction::Set { url, token } => {
                let config = Config { url, token };
                save_config(&config)?;
                println!("Config saved to {}", config_path()?.display());
            }
            ConfigAction::Show => {
                let config = load_config()?;
                println!("url:   {}", config.url);
                println!("token: {}", config.token);
            }
        },

        Command::Shorten { url, pattern } => {
            let config = load_config()?;
            let slug = shorten(&config, &url, pattern.as_deref())?;
            let full = api_url(&config.url, &slug);
            println!("{full}");
        }

        Command::Upload { file, pattern } => {
            let config = load_config()?;
            let slug = upload(&config, &file, pattern.as_deref())?;
            let full = api_url(&config.url, &slug);
            println!("{full}");
        }

        Command::Delete { slug } => {
            let config = load_config()?;
            delete(&config, &slug)?;
            println!("Deleted {slug}");
        }

        Command::List => {
            let config = load_config()?;
            let entries = list(&config)?;

            Pager::with_default_pager("less -R").setup();

            if entries.is_empty() {
                println!("No slugs found.");
            } else {
                let base = config.url.trim_end_matches('/');
                let slug_w = entries
                    .iter()
                    .map(|e| e.slug.len())
                    .max()
                    .unwrap_or(4)
                    .max(4);
                println!("{:<slug_w$}  {:<8}  {}", "SLUG", "TYPE", "TARGET");
                println!("{}", "-".repeat(slug_w + 2 + 8 + 2 + 40));
                for entry in &entries {
                    let (kind, target) = match &entry.kind {
                        ListEntryKind::Url { url } => ("url", url.as_str().to_string()),
                        ListEntryKind::File { r2_key } => ("file", format!("r2:{r2_key}")),
                        ListEntryKind::Unknown => ("unknown", String::new()),
                    };
                    println!(
                        "{:<slug_w$}  {:<8}  {}  →  {base}/{}",
                        entry.slug, kind, target, entry.slug
                    );
                }
            }
        }
    }

    Ok(())
}
