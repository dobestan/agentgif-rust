//! AgentGIF CLI (Rust) — GIF for humans. Cast for agents.
//!
//! Install: cargo install agentgif
//! Usage:   agentgif login | upload | search | list | info | embed | badge
//!
//! Full documentation: https://agentgif.com/docs/cli/

mod client;
mod config;

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::io::{self, Write};
use std::process;

const VERSION: &str = "0.2.0";

#[derive(Parser)]
#[command(
    name = "agentgif",
    version = VERSION,
    about = "AgentGIF — GIF for humans. Cast for agents.",
    after_help = "Docs: https://agentgif.com/docs/cli/"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate via browser
    Login,
    /// Remove stored credentials
    Logout,
    /// Show current user info
    Whoami,
    /// Upload a GIF
    Upload {
        /// Path to GIF file
        path: String,
        /// GIF title
        #[arg(short, long)]
        title: Option<String>,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Command demonstrated
        #[arg(short, long)]
        command: Option<String>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Asciicast file path
        #[arg(long)]
        cast: Option<String>,
        /// Terminal theme
        #[arg(long)]
        theme: Option<String>,
        /// Upload as unlisted
        #[arg(long)]
        unlisted: bool,
        /// Don't auto-detect repo
        #[arg(long)]
        no_repo: bool,
    },
    /// Search public GIFs
    Search {
        /// Search query
        query: Vec<String>,
    },
    /// List your GIFs
    List {
        /// Filter by repo slug
        #[arg(long)]
        repo: Option<String>,
    },
    /// Show GIF details (JSON)
    Info {
        /// GIF ID
        gif_id: String,
    },
    /// Show embed codes
    Embed {
        /// GIF ID
        gif_id: String,
        /// Output format: md, html, iframe, script, all
        #[arg(short, long, default_value = "all")]
        format: String,
    },
    /// Update GIF metadata
    Update {
        /// GIF ID
        gif_id: String,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
        /// New command
        #[arg(short, long)]
        command: Option<String>,
        /// New tags
        #[arg(long)]
        tags: Option<String>,
    },
    /// Delete a GIF
    Delete {
        /// GIF ID
        gif_id: String,
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Generate demo GIFs from a README or package docs
    Generate {
        /// Source URL (GitHub repo, PyPI, npm)
        source: Option<String>,
        /// PyPI package name
        #[arg(long)]
        pypi: Option<String>,
        /// npm package name
        #[arg(long)]
        npm: Option<String>,
        /// Maximum GIFs to generate
        #[arg(long, default_value = "5")]
        max: u32,
        /// Don't wait for completion, return job ID immediately
        #[arg(long)]
        no_wait: bool,
    },
    /// Check status of a generate job
    GenerateStatus {
        /// Job ID
        job_id: String,
    },
    /// Record a VHS tape, generate GIF, and upload
    Record {
        /// Path to .tape file
        tape_path: String,
        /// Terminal theme for recording
        #[arg(long)]
        theme: Option<String>,
    },
    /// Terminal-themed package badges
    Badge {
        #[command(subcommand)]
        command: BadgeCommand,
    },
}

#[derive(Subcommand)]
enum BadgeCommand {
    /// Generate a badge URL and embed codes
    Url {
        /// Provider: pypi, npm, crates, github
        #[arg(short, long)]
        provider: String,
        /// Package name
        #[arg(short = 'k', long = "package")]
        package: String,
        /// Metric: version, downloads, stars
        #[arg(short, long, default_value = "version")]
        metric: String,
        /// Terminal theme (e.g. dracula)
        #[arg(long)]
        theme: Option<String>,
        /// Badge style (default, flat)
        #[arg(long)]
        style: Option<String>,
        /// Output format: url, md, html, img, all
        #[arg(short, long, default_value = "all")]
        format: String,
    },
    /// List available terminal themes
    Themes,
}

fn main() {
    check_for_updates();

    let cli = Cli::parse();
    match cli.command {
        Commands::Login => cmd_login(),
        Commands::Logout => cmd_logout(),
        Commands::Whoami => cmd_whoami(),
        Commands::Upload {
            path,
            title,
            description,
            command,
            tags,
            cast,
            theme,
            unlisted,
            no_repo,
        } => cmd_upload(
            &path,
            title.as_deref(),
            description.as_deref(),
            command.as_deref(),
            tags.as_deref(),
            cast.as_deref(),
            theme.as_deref(),
            unlisted,
            no_repo,
        ),
        Commands::Search { query } => cmd_search(&query.join(" ")),
        Commands::List { repo } => cmd_list(repo.as_deref()),
        Commands::Info { gif_id } => cmd_info(&gif_id),
        Commands::Embed { gif_id, format } => cmd_embed(&gif_id, &format),
        Commands::Update {
            gif_id,
            title,
            description,
            command,
            tags,
        } => cmd_update(&gif_id, title, description, command, tags),
        Commands::Delete { gif_id, yes } => cmd_delete(&gif_id, yes),
        Commands::Generate {
            source,
            pypi,
            npm,
            max,
            no_wait,
        } => cmd_generate(source.as_deref(), pypi.as_deref(), npm.as_deref(), max, no_wait),
        Commands::GenerateStatus { job_id } => cmd_generate_status(&job_id),
        Commands::Record { tape_path, theme } => cmd_record(&tape_path, theme.as_deref()),
        Commands::Badge { command } => match command {
            BadgeCommand::Url {
                provider,
                package,
                metric,
                theme,
                style,
                format,
            } => cmd_badge_url(&provider, &package, &metric, theme, style, &format),
            BadgeCommand::Themes => cmd_badge_themes(),
        },
    }
}

fn check_for_updates() {
    let c = client::Client::new();
    if let Ok(data) = c.cli_version() {
        if let Some(latest) = data.get("latest") {
            if !latest.is_empty() && latest != VERSION {
                eprintln!(
                    "Update available: {} → {}. Run: cargo install agentgif",
                    VERSION, latest
                );
            }
        }
    }
}

fn require_auth() {
    if config::get_api_key().is_empty() {
        eprintln!("Not logged in. Run: agentgif login");
        process::exit(1);
    }
}

// ── login ───────────────────────────────────────

fn cmd_login() {
    let c = client::Client::new();
    let data = match c.device_auth() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let verify_url = data["verification_url"].as_str().unwrap_or("");
    let user_code = data["user_code"].as_str().unwrap_or("");
    let device_code = data["device_code"].as_str().unwrap_or("");
    let interval = data["interval"].as_f64().unwrap_or(5.0).max(1.0) as u64;

    println!("\nOpen this URL to authenticate:\n  {verify_url}\n");
    println!("Your code: {user_code}\n");

    // Try to open browser
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(verify_url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(verify_url)
        .spawn();

    println!("Waiting for approval...");

    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_secs(interval));

        let (token, status) = match c.device_token(device_code) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if status == 200 {
            let api_key = token["api_key"].as_str().unwrap_or("");
            let username = token["username"].as_str().unwrap_or("");
            if let Err(e) = config::save_credentials(api_key, username) {
                eprintln!("Error saving credentials: {e}");
                process::exit(1);
            }
            println!("✓ Logged in as @{username}");
            return;
        }
        if status == 403 {
            eprintln!("Authorization denied by user");
            process::exit(1);
        }
        if status == 410 {
            eprintln!("Device code expired — try again");
            process::exit(1);
        }
        // 400 = still pending, continue
    }
    eprintln!("Authentication timed out after 5 minutes");
    process::exit(1);
}

// ── logout ──────────────────────────────────────

fn cmd_logout() {
    if let Err(e) = config::clear_credentials() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
    println!("✓ Logged out");
}

// ── whoami ──────────────────────────────────────

fn cmd_whoami() {
    require_auth();
    let c = client::Client::new();
    let user = match c.whoami() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };
    let username = user["username"].as_str().unwrap_or("-");
    let display_name = user["display_name"].as_str().unwrap_or("-");
    let upload_count = user["upload_count"].as_f64().unwrap_or(0.0);
    println!("@{username} — {display_name}");
    println!("GIFs: {:.0}", upload_count);
}

// ── upload ──────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_upload(
    path: &str,
    title: Option<&str>,
    description: Option<&str>,
    command: Option<&str>,
    tags: Option<&str>,
    cast: Option<&str>,
    theme: Option<&str>,
    unlisted: bool,
    no_repo: bool,
) {
    require_auth();

    let mut opts = HashMap::new();
    if let Some(v) = title {
        opts.insert("title".to_string(), v.to_string());
    }
    if let Some(v) = description {
        opts.insert("description".to_string(), v.to_string());
    }
    if let Some(v) = command {
        opts.insert("command".to_string(), v.to_string());
    }
    if let Some(v) = tags {
        opts.insert("tags".to_string(), v.to_string());
    }
    if let Some(v) = cast {
        opts.insert("cast_path".to_string(), v.to_string());
    }
    if let Some(v) = theme {
        opts.insert("theme".to_string(), v.to_string());
    }

    if unlisted {
        opts.insert("visibility".to_string(), "unlisted".to_string());
    } else {
        opts.insert("visibility".to_string(), "public".to_string());
    }

    if !no_repo {
        if let Some(repo) = detect_repo() {
            opts.insert("repo_slug".to_string(), repo);
        }
    }

    println!("Uploading {path}...");
    let c = client::Client::new();
    match c.upload(path, &opts) {
        Ok(result) => {
            println!("✓ Uploaded: {}", result["url"].as_str().unwrap_or("-"));
            if let Some(md) = result["embed"]["markdown"].as_str() {
                println!("  Embed: {md}");
            }
        }
        Err(e) => {
            eprintln!("Upload failed: {e}");
            process::exit(1);
        }
    }
}

fn detect_repo() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let url = url.trim_end_matches('/');
    let name = url.rsplit('/').next()?;
    Some(name.trim_end_matches(".git").to_string())
}

// ── search ──────────────────────────────────────

fn cmd_search(query: &str) {
    if query.is_empty() {
        eprintln!("Usage: agentgif search <query>");
        process::exit(1);
    }

    let c = client::Client::new();
    let data = match c.search(query) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let results = data["results"].as_array();
    if results.map_or(true, |r| r.is_empty()) {
        println!("No results found.");
        return;
    }

    println!("Search: {} ({} results)\n", query, data["count"]);
    println!("{:<10}  {:<30}  {}", "ID", "Title", "Command");
    println!(
        "{:<10}  {:<30}  {}",
        "──────────",
        "──────────────────────────────",
        "────────────────────"
    );
    for gif in results.unwrap() {
        let id = gif["id"].as_str().unwrap_or("-");
        let mut title = gif["title"].as_str().unwrap_or("-").to_string();
        let cmd = gif["command"].as_str().unwrap_or("-");
        if title.len() > 30 {
            title.truncate(30);
        }
        println!("{:<10}  {:<30}  {}", id, title, cmd);
    }
}

// ── list ────────────────────────────────────────

fn cmd_list(repo: Option<&str>) {
    require_auth();

    let c = client::Client::new();
    let data = match c.list_gifs(repo.unwrap_or("")) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let results = data["results"].as_array();
    if results.map_or(true, |r| r.is_empty()) {
        println!("No GIFs found.");
        return;
    }

    println!("My GIFs ({})\n", data["count"]);
    println!("{:<10}  {:<30}  {:>6}", "ID", "Title", "Views");
    println!(
        "{:<10}  {:<30}  {:>6}",
        "──────────",
        "──────────────────────────────",
        "──────"
    );
    for gif in results.unwrap() {
        let id = gif["id"].as_str().unwrap_or("-");
        let mut title = gif["title"].as_str().unwrap_or("-").to_string();
        let views = gif["view_count"].as_f64().unwrap_or(0.0);
        if title.len() > 30 {
            title.truncate(30);
        }
        println!("{:<10}  {:<30}  {:>6.0}", id, title, views);
    }
}

// ── info ────────────────────────────────────────

fn cmd_info(gif_id: &str) {
    let c = client::Client::new();
    let data = match c.get_gif(gif_id) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&data).unwrap_or_default()
    );
}

// ── embed ───────────────────────────────────────

fn cmd_embed(gif_id: &str, format: &str) {
    let c = client::Client::new();
    let codes = match c.embed_codes(gif_id) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let key_map: HashMap<&str, &str> = [
        ("md", "markdown"),
        ("html", "html"),
        ("iframe", "iframe"),
        ("script", "script"),
    ]
    .into();

    if format == "all" {
        for (name, code) in &codes {
            println!("{name}:\n{code}\n");
        }
    } else {
        let key = key_map.get(format).copied().unwrap_or(format);
        if let Some(code) = codes.get(key) {
            println!("{code}");
        } else {
            eprintln!("Unknown format: {format}");
            process::exit(1);
        }
    }
}

// ── update ──────────────────────────────────────

fn cmd_update(
    gif_id: &str,
    title: Option<String>,
    description: Option<String>,
    command: Option<String>,
    tags: Option<String>,
) {
    require_auth();

    let mut fields = HashMap::new();
    if let Some(v) = title {
        fields.insert("title".to_string(), v);
    }
    if let Some(v) = description {
        fields.insert("description".to_string(), v);
    }
    if let Some(v) = command {
        fields.insert("command".to_string(), v);
    }
    if let Some(v) = tags {
        fields.insert("tags".to_string(), v);
    }

    if fields.is_empty() {
        eprintln!("No fields to update. Use --title, --description, --command, or --tags.");
        process::exit(1);
    }

    let c = client::Client::new();
    match c.update_gif(gif_id, &fields) {
        Ok(result) => {
            println!(
                "✓ Updated {}: {}",
                gif_id,
                result["title"].as_str().unwrap_or("-")
            );
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ── delete ──────────────────────────────────────

fn cmd_delete(gif_id: &str, skip_confirm: bool) {
    require_auth();

    if !skip_confirm {
        print!("Delete {gif_id}? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Cancelled.");
            return;
        }
    }

    let c = client::Client::new();
    match c.delete_gif(gif_id) {
        Ok(()) => println!("✓ Deleted {gif_id}"),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ── generate ────────────────────────────────────

fn detect_source_type(url: &str) -> &str {
    if url.contains("github.com") {
        "github"
    } else if url.contains("pypi.org") {
        "pypi"
    } else if url.contains("npmjs.com") {
        "npm"
    } else {
        ""
    }
}

fn cmd_generate(
    source: Option<&str>,
    pypi: Option<&str>,
    npm: Option<&str>,
    max: u32,
    no_wait: bool,
) {
    require_auth();

    // Determine source_url and source_type from flags
    let (source_url, source_type) = if let Some(pkg) = pypi {
        (format!("https://pypi.org/project/{pkg}/"), "pypi".to_string())
    } else if let Some(pkg) = npm {
        (
            format!("https://www.npmjs.com/package/{pkg}"),
            "npm".to_string(),
        )
    } else if let Some(url) = source {
        let st = detect_source_type(url);
        (url.to_string(), st.to_string())
    } else {
        eprintln!("Provide a source URL, --pypi <package>, or --npm <package>");
        process::exit(1);
    };

    let mut payload = serde_json::json!({
        "source_url": source_url,
        "max_gifs": max,
    });
    if !source_type.is_empty() {
        payload["source_type"] = serde_json::Value::String(source_type);
    }

    let c = client::Client::new();
    let job = match c.generate_tape(&payload) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let job_id = job["job_id"].as_str().unwrap_or("");
    println!("Job created: {job_id}");
    if let Some(status_url) = job["status_url"].as_str() {
        println!("  Status URL: {status_url}");
    }

    if no_wait {
        println!("Check status: agentgif generate-status {job_id}");
        return;
    }

    // Poll until completed/failed (max 5 min)
    println!("Generating GIFs...");
    let start = std::time::Instant::now();
    let mut prev_status = String::new();
    loop {
        if start.elapsed().as_secs() > 300 {
            eprintln!("Timed out after 5 minutes. Check status:");
            eprintln!("  agentgif generate-status {job_id}");
            process::exit(1);
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        let data = match c.generate_status(job_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let current = data["status"].as_str().unwrap_or("").to_string();
        if current != prev_status {
            println!("  Status: {current}");
            prev_status = current.clone();
        }

        if current == "completed" {
            let gifs_created = data["gifs_created"].as_f64().unwrap_or(0.0);
            println!("Done! {:.0} GIFs generated.", gifs_created);

            if let Some(gifs) = data["gifs"].as_array() {
                println!();
                println!("{:<10}  {:<30}  {}", "ID", "Title", "URL");
                println!(
                    "{:<10}  {:<30}  {}",
                    "──────────",
                    "──────────────────────────────",
                    "────────────────────────────────────────"
                );
                for gif in gifs {
                    let id = gif["id"].as_str().unwrap_or("-");
                    let title = gif["title"].as_str().unwrap_or("-");
                    let gif_url = gif["url"].as_str().unwrap_or("-");
                    println!("{:<10}  {:<30}  {}", id, title, gif_url);
                }
            }
            return;
        }

        if current == "failed" {
            let err_msg = data["error_message"]
                .as_str()
                .unwrap_or("Unknown error");
            eprintln!("Generation failed: {err_msg}");
            process::exit(1);
        }
    }
}

// ── generate-status ─────────────────────────────

fn cmd_generate_status(job_id: &str) {
    require_auth();

    let c = client::Client::new();
    let data = match c.generate_status(job_id) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let status = data["status"].as_str().unwrap_or("-");
    let commands_found = data["commands_found"].as_f64().unwrap_or(0.0);
    let gifs_created = data["gifs_created"].as_f64().unwrap_or(0.0);
    println!("Job:      {job_id}");
    println!("Status:   {status}");
    println!("Commands: {:.0}", commands_found);
    println!("GIFs:     {:.0}", gifs_created);

    if let Some(err) = data["error_message"].as_str() {
        if !err.is_empty() {
            println!("Error:    {err}");
        }
    }

    if let Some(gifs) = data["gifs"].as_array() {
        if !gifs.is_empty() {
            println!();
            println!("{:<10}  {:<30}  {}", "ID", "Title", "URL");
            println!(
                "{:<10}  {:<30}  {}",
                "──────────",
                "──────────────────────────────",
                "────────────────────────────────────────"
            );
            for gif in gifs {
                let id = gif["id"].as_str().unwrap_or("-");
                let title = gif["title"].as_str().unwrap_or("-");
                let gif_url = gif["url"].as_str().unwrap_or("-");
                println!("{:<10}  {:<30}  {}", id, title, gif_url);
            }
        }
    }
}

// ── record ──────────────────────────────────────

fn cmd_record(tape_file: &str, theme: Option<&str>) {
    require_auth();

    let tape_path = std::path::Path::new(tape_file);
    if !tape_path.exists() {
        eprintln!("Tape file not found: {tape_file}");
        process::exit(1);
    }

    // Parse the tape file for "Output <path>" to find the GIF path
    let tape_content = match std::fs::read_to_string(tape_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading tape file: {e}");
            process::exit(1);
        }
    };

    let mut output_path = tape_path.with_extension("gif");
    for line in tape_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Output ") {
            let path_str = trimmed.strip_prefix("Output ").unwrap().trim();
            // Remove surrounding quotes if present
            let path_str = path_str.trim_matches('"').trim_matches('\'');
            output_path = std::path::PathBuf::from(path_str);
            break;
        }
    }

    // Run VHS
    println!("Running VHS: {tape_file}");
    let result = std::process::Command::new("vhs").arg(tape_file).output();

    match result {
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("VHS not found. Install it: https://github.com/charmbracelet/vhs");
            } else {
                eprintln!("Error running VHS: {e}");
            }
            process::exit(1);
        }
        Ok(output) => {
            if !output.status.success() {
                eprintln!("VHS failed:\n{}", String::from_utf8_lossy(&output.stderr));
                process::exit(1);
            }
        }
    }

    if !output_path.exists() {
        eprintln!("Expected output not found: {}", output_path.display());
        process::exit(1);
    }

    // Upload the generated GIF
    println!("Uploading {}...", output_path.display());
    let mut opts = HashMap::new();

    // Derive title from tape file stem
    let auto_title = tape_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(['-', '_'], " ");
    opts.insert("title".to_string(), auto_title);
    opts.insert("visibility".to_string(), "public".to_string());

    if let Some(t) = theme {
        opts.insert("theme".to_string(), t.to_string());
    }

    if let Some(repo) = detect_repo() {
        opts.insert("repo_slug".to_string(), repo);
    }

    let c = client::Client::new();
    match c.upload(&output_path.to_string_lossy(), &opts) {
        Ok(result) => {
            println!("✓ Uploaded: {}", result["url"].as_str().unwrap_or("-"));
            if let Some(md) = result["embed"]["markdown"].as_str() {
                println!("  Embed: {md}");
            }
        }
        Err(e) => {
            eprintln!("Upload failed: {e}");
            process::exit(1);
        }
    }
}

// ── badge ───────────────────────────────────────

fn cmd_badge_url(
    provider: &str,
    package: &str,
    metric: &str,
    theme: Option<String>,
    style: Option<String>,
    format: &str,
) {
    let mut opts = HashMap::new();
    opts.insert("metric".to_string(), metric.to_string());
    if let Some(t) = theme {
        opts.insert("theme".to_string(), t);
    }
    if let Some(s) = style {
        opts.insert("style".to_string(), s);
    }

    let c = client::Client::new();
    let data = match c.badge_url(provider, package, &opts) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    match format {
        "all" => {
            println!("URL:  {}", data.get("url").unwrap_or(&String::new()));
            println!(
                "Markdown:\n{}",
                data.get("markdown").unwrap_or(&String::new())
            );
            println!("HTML:\n{}", data.get("html").unwrap_or(&String::new()));
            println!("Image:\n{}", data.get("img").unwrap_or(&String::new()));
        }
        "url" => println!("{}", data.get("url").unwrap_or(&String::new())),
        "md" => println!("{}", data.get("markdown").unwrap_or(&String::new())),
        "html" => println!("{}", data.get("html").unwrap_or(&String::new())),
        "img" => println!("{}", data.get("img").unwrap_or(&String::new())),
        _ => {
            eprintln!("Unknown format: {format}");
            process::exit(1);
        }
    }
}

fn cmd_badge_themes() {
    let c = client::Client::new();
    let data = match c.badge_themes() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let count = data["count"].as_f64().unwrap_or(0.0);
    println!("Badge Themes ({:.0})\n", count);
    println!(
        "{:<20}  {:<25}  {:<12}  {}",
        "Slug", "Name", "Category", "Preview URL"
    );
    println!(
        "{:<20}  {:<25}  {:<12}  {}",
        "────────────────────",
        "─────────────────────────",
        "────────────",
        "────────────────────────────────────────"
    );

    if let Some(themes) = data["themes"].as_array() {
        for t in themes {
            let slug = t["slug"].as_str().unwrap_or("-");
            let mut name = t["name"].as_str().unwrap_or("-").to_string();
            let cat = t["category"].as_str().unwrap_or("-");
            let preview_url = t["preview_url"].as_str().unwrap_or("-");
            if name.len() > 25 {
                name.truncate(25);
            }
            println!("{:<20}  {:<25}  {:<12}  {}", slug, name, cat, preview_url);
        }
    }
}
