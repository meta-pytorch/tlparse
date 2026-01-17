use clap::Parser;

use anyhow::{bail, Context};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use tlparse::{
    // New reusable library API for multi-rank landing generation
    generate_multi_rank_landing,
    parse_path,
    // Context used to pass rank list; other fields are recomputed inside the API
    MultiRankContext,
    ParseConfig,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    path: PathBuf,
    /// Parse most recent log
    #[arg(long)]
    latest: bool,
    /// Output directory, defaults to `tl_out`
    #[arg(short, default_value = "tl_out")]
    out: PathBuf,
    /// Delete out directory if it already exists
    #[arg(long)]
    overwrite: bool,
    /// Return non-zero exit code if unrecognized log lines are found.  Mostly useful for unit
    /// testing.
    #[arg(long)]
    strict: bool,
    /// Return non-zero exit code if some log lines do not have associated compile id.  Used for
    /// unit testing
    #[arg(long)]
    strict_compile_id: bool,
    /// Don't open browser at the end
    #[arg(long)]
    no_browser: bool,
    /// Some custom HTML to append to the top of report
    #[arg(long, default_value = "")]
    custom_header_html: String,
    /// Be more chatty
    #[arg(short, long)]
    verbose: bool,
    /// Some parsers will write output as rendered html for prettier viewing.
    /// Enabiling this option will enforce output as plain text for easier diffing
    #[arg(short, long)]
    plain_text: bool,
    /// For export specific logs
    #[arg(short, long)]
    export: bool,
    /// For inductor provenance tracking highlighter
    #[arg(short, long)]
    inductor_provenance: bool,
    /// Parse all ranks and create a unified multi-rank report
    #[arg(long)]
    all_ranks_html: bool,
    /// Start a local HTTP server to serve the output directory
    #[arg(long)]
    serve: bool,
    /// Port for the HTTP server (used with --serve). If not specified, finds an available port.
    #[arg(long)]
    port: Option<u16>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Early validation of incompatible flags
    if cli.all_ranks_html && cli.latest {
        bail!("--latest cannot be used with --all-ranks-html");
    }

    // --serve implies --no-browser (we'll serve instead of opening)
    let open_browser = !cli.no_browser && !cli.serve;

    let path = if cli.latest {
        let input_path = cli.path;
        // Path should be a directory
        if !input_path.is_dir() {
            bail!(
                "Input path {} is not a directory (required when using --latest)",
                input_path.display()
            );
        }

        let last_modified_file = std::fs::read_dir(&input_path)
            .with_context(|| format!("Couldn't access directory {}", input_path.display()))?
            .flatten()
            .filter(|f| f.metadata().unwrap().is_file())
            .max_by_key(|x| x.metadata().unwrap().modified().unwrap());

        let Some(last_modified_file) = last_modified_file else {
            bail!("No files found in directory {}", input_path.display());
        };
        last_modified_file.path()
    } else {
        cli.path
    };

    let config = ParseConfig {
        strict: cli.strict,
        strict_compile_id: cli.strict_compile_id,
        custom_parsers: Vec::new(),
        custom_header_html: cli.custom_header_html,
        verbose: cli.verbose,
        plain_text: cli.plain_text,
        export: cli.export,
        inductor_provenance: cli.inductor_provenance,
    };

    if cli.all_ranks_html {
        handle_all_ranks(&config, path, cli.out.clone(), cli.overwrite, open_browser)?;
    } else {
        handle_one_rank(
            &config,
            path,
            false, // already converted path to latest log file
            cli.out.clone(),
            open_browser,
            cli.overwrite,
        )?;
    }

    if cli.serve {
        serve_directory(&cli.out, cli.port)?;
    }

    Ok(())
}

/// Create the output directory
fn setup_output_directory(out_path: &PathBuf, overwrite: bool) -> anyhow::Result<()> {
    if out_path.exists() {
        if !overwrite {
            bail!(
                "Directory {} already exists; pass --overwrite to replace it or use -o OUTDIR",
                out_path.display()
            );
        }
        fs::remove_dir_all(&out_path)?;
    }
    fs::create_dir_all(&out_path)?;
    Ok(())
}

/// Parse a log file and write the rendered artefacts into `output_dir`.
fn parse_and_write_output(
    config: &ParseConfig,
    log_path: &PathBuf,
    output_dir: &PathBuf,
) -> anyhow::Result<PathBuf> {
    let output = parse_path(log_path, config)?;

    for (filename, content) in output {
        let out_path = output_dir.join(&filename);
        if let Some(dir) = out_path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(out_path, content)?;
    }
    Ok(output_dir.join("index.html"))
}

fn handle_one_rank(
    cfg: &ParseConfig,
    input_path: PathBuf,
    latest: bool,
    out_dir: PathBuf,
    open_browser: bool,
    overwrite: bool,
) -> anyhow::Result<()> {
    // Resolve which log file we should parse
    let log_path = if latest {
        if !input_path.is_dir() {
            bail!(
                "Input path {} is not a directory (required with --latest)",
                input_path.display()
            );
        }
        std::fs::read_dir(input_path)?
            .flatten()
            .filter(|e| e.metadata().ok().map_or(false, |m| m.is_file()))
            .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
            .map(|e| e.path())
            .context("No files found in directory for --latest")?
    } else {
        input_path.clone()
    };

    setup_output_directory(&out_dir, overwrite)?;
    let main_output_file = parse_and_write_output(cfg, &log_path, &out_dir)?;

    if open_browser {
        opener::open(&main_output_file)?;
    }
    Ok(())
}

fn handle_all_ranks(
    cfg: &ParseConfig,
    path: PathBuf,
    out_path: PathBuf,
    overwrite: bool,
    open_browser: bool,
) -> anyhow::Result<()> {
    let input_dir = path;
    if !input_dir.is_dir() {
        bail!(
            "Input path {} must be a directory when using --all-ranks-html",
            input_dir.display()
        );
    }

    setup_output_directory(&out_path, overwrite)?;

    // Discover rank log files
    let rank_logs: Vec<_> = std::fs::read_dir(&input_dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let filename = path.file_name()?.to_str()?;
            filename
                .strip_prefix("dedicated_log_torch_trace_rank_")?
                .strip_suffix(".log")?
                .split('_')
                .next()?
                .parse::<u32>()
                .ok()
                .map(|rank_num| (path.clone(), rank_num))
        })
        .collect();

    if rank_logs.is_empty() {
        bail!(
            "No rank log files found in directory {}",
            input_dir.display()
        );
    }

    // Extract rank numbers, sort numerically, then convert to strings for HTML generation
    let mut rank_nums: Vec<u32> = rank_logs.iter().map(|(_, rank)| *rank).collect();
    rank_nums.sort_unstable();
    let sorted_ranks: Vec<String> = rank_nums.iter().map(|r| r.to_string()).collect();

    for (log_path, rank_num) in rank_logs {
        let subdir = out_path.join(format!("rank_{rank_num}"));
        println!("Processing rank {rank_num} → {}", subdir.display());
        handle_one_rank(cfg, log_path, false, subdir, false, overwrite)?;
    }
    // Build a minimal context; values other than ranks are recomputed inside the library API
    let ctx = MultiRankContext {
        css: "",
        custom_header_html: &cfg.custom_header_html,
        num_ranks: sorted_ranks.len(),
        ranks: sorted_ranks,
        qps: "",
        has_chromium_events: false,
        show_desync_warning: false,
        compile_id_divergence: false,
        diagnostics: Default::default(),
    };

    let landing_page_path = generate_multi_rank_landing(cfg, &ctx, &out_path)?;

    if open_browser {
        opener::open(&landing_page_path)?;
    }

    Ok(())
}

/// Find an available port in the given range
fn find_available_port(start: u16, end: u16) -> anyhow::Result<u16> {
    use std::net::TcpListener;
    for port in start..end {
        if TcpListener::bind(("0.0.0.0", port)).is_ok() {
            return Ok(port);
        }
    }
    bail!("No available ports in range {}-{}", start, end - 1)
}

/// Serve a directory over HTTP
fn serve_directory(dir: &PathBuf, port: Option<u16>) -> anyhow::Result<()> {
    let port = match port {
        Some(p) => p,
        None => find_available_port(8000, 8100)?,
    };

    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to start server on {}: {}", addr, e))?;

    let url = format!("http://localhost:{}/", port);
    println!("Serving {} at {}", dir.display(), url);
    println!("Press Ctrl+C to stop");

    let dir = dir.canonicalize()?;

    for request in server.incoming_requests() {
        let url_path = request.url().trim_start_matches('/');
        // URL decode the path
        let url_path = urlencoding_decode(url_path);
        let file_path = if url_path.is_empty() {
            dir.join("index.html")
        } else {
            dir.join(&url_path)
        };

        // Security: ensure the path is within the served directory
        let file_path = match file_path.canonicalize() {
            Ok(p) if p.starts_with(&dir) => p,
            _ => {
                let response =
                    tiny_http::Response::from_string("404 Not Found").with_status_code(404);
                let _ = request.respond(response);
                continue;
            }
        };

        if file_path.is_file() {
            match fs::File::open(&file_path) {
                Ok(mut file) => {
                    let mut content = Vec::new();
                    if file.read_to_end(&mut content).is_ok() {
                        let content_type = guess_content_type(&file_path);
                        let response = tiny_http::Response::from_data(content).with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                content_type.as_bytes(),
                            )
                            .unwrap(),
                        );
                        let _ = request.respond(response);
                    } else {
                        let response =
                            tiny_http::Response::from_string("500 Internal Server Error")
                                .with_status_code(500);
                        let _ = request.respond(response);
                    }
                }
                Err(_) => {
                    let response =
                        tiny_http::Response::from_string("404 Not Found").with_status_code(404);
                    let _ = request.respond(response);
                }
            }
        } else {
            let response = tiny_http::Response::from_string("404 Not Found").with_status_code(404);
            let _ = request.respond(response);
        }
    }

    Ok(())
}

/// Simple URL decoding (handles %XX sequences)
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Guess content type based on file extension
fn guess_content_type(path: &PathBuf) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        "py" => "text/x-python; charset=utf-8",
        _ => "application/octet-stream",
    }
    .to_string()
}
