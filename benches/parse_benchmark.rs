//! Benchmark for tlparse: measures wall time and peak memory (RSS).
//!
//! Usage:
//!   TLPARSE_BENCH_INPUT=/path/to/file cargo bench --bench parse_benchmark
//!   cargo bench --bench parse_benchmark -- /path/to/file   # custom input via CLI arg

use std::io::BufRead;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::tempdir;

const WARMUP_ITERS: u32 = 2;
const BENCH_ITERS: u32 = 5;

fn get_peak_rss_bytes() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use std::mem::MaybeUninit;
        unsafe {
            let mut usage = MaybeUninit::<libc::rusage>::zeroed();
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
                // macOS reports ru_maxrss in bytes
                Some(usage.assume_init().ru_maxrss as u64)
            } else {
                None
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::mem::MaybeUninit;
        unsafe {
            let mut usage = MaybeUninit::<libc::rusage>::zeroed();
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
                // Linux reports ru_maxrss in kilobytes
                Some(usage.assume_init().ru_maxrss as u64 * 1024)
            } else {
                None
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn run_parse(input: &PathBuf) -> std::time::Duration {
    let config = tlparse::ParseConfig::default();
    let out_dir = tempdir().expect("failed to create temp dir");
    let start = Instant::now();
    let output = tlparse::parse_path(input, &config).expect("parse_path failed");
    let elapsed = start.elapsed();

    // Write output to exercise the full pipeline
    for (path, content) in &output {
        let full_path = out_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create output subdirectory");
        }
        std::fs::write(&full_path, content)
            .expect("failed to write output file");
    }
    elapsed
}

fn main() {
    // Determine input path: CLI arg > env var (no default — must be explicit)
    let args: Vec<String> = std::env::args().collect();
    let input_path = if args.len() > 1 && !args[1].starts_with('-') {
        PathBuf::from(&args[1])
    } else if let Ok(env_path) = std::env::var("TLPARSE_BENCH_INPUT") {
        PathBuf::from(env_path)
    } else {
        eprintln!("Error: no input file specified.");
        eprintln!();
        eprintln!("Provide a TORCH_LOG file via one of:");
        eprintln!("  TLPARSE_BENCH_INPUT=/path/to/file cargo bench --bench parse_benchmark");
        eprintln!("  cargo bench --bench parse_benchmark -- /path/to/file");
        std::process::exit(1);
    };

    if !input_path.exists() {
        eprintln!("Error: input file not found: {}", input_path.display());
        std::process::exit(1);
    }

    let file_size = std::fs::metadata(&input_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let line_count = std::io::BufReader::new(
        std::fs::File::open(&input_path).expect("failed to open input file for line counting"),
    )
    .lines()
    .count();

    println!("=== tlparse benchmark ===");
    println!(
        "Input: {} ({}, {} lines)",
        input_path.display(),
        format_bytes(file_size),
        line_count
    );
    println!();

    // Cold-run RSS: measure peak RSS after a single parse before any warmup.
    // This captures the first-run memory footprint before caches are populated.
    let rss_cold_before = get_peak_rss_bytes();
    run_parse(&input_path);
    let rss_cold_after = get_peak_rss_bytes();

    // Warmup
    print!("Warming up ({WARMUP_ITERS} iterations)...");
    for _ in 0..WARMUP_ITERS {
        run_parse(&input_path);
    }
    println!(" done");

    // NOTE: ru_maxrss reports the *lifetime* peak RSS of the process, so the
    // value after warmup already includes the high-water mark from earlier
    // iterations.  The "RSS delta (during bench)" below therefore only captures
    // *new* peaks that exceed the warmup maximum — it will be zero if the
    // warmup already reached the true peak.  The cold-run measurement above
    // provides a more meaningful single-iteration memory figure.
    let rss_before = get_peak_rss_bytes();

    // Benchmark
    println!("Running {BENCH_ITERS} iterations...");
    let mut durations = Vec::with_capacity(BENCH_ITERS as usize);
    for i in 0..BENCH_ITERS {
        let elapsed = run_parse(&input_path);
        println!("  iter {}: {:.3}ms", i + 1, elapsed.as_secs_f64() * 1000.0);
        durations.push(elapsed);
    }

    let rss_after = get_peak_rss_bytes();

    // Stats
    durations.sort();
    let total: std::time::Duration = durations.iter().sum();
    let mean = total / BENCH_ITERS;
    let median = durations[durations.len() / 2];
    let min = durations[0];
    let max = durations[durations.len() - 1];

    println!();
    println!("--- Results ---");
    println!("  mean:   {:.3}ms", mean.as_secs_f64() * 1000.0);
    println!("  median: {:.3}ms", median.as_secs_f64() * 1000.0);
    println!("  min:    {:.3}ms", min.as_secs_f64() * 1000.0);
    println!("  max:    {:.3}ms", max.as_secs_f64() * 1000.0);

    // Cold-run RSS (single iteration, no prior warmup)
    if let (Some(before), Some(after)) = (rss_cold_before, rss_cold_after) {
        println!("  cold-run peak RSS: {}", format_bytes(after));
        if after > before {
            println!(
                "  cold-run RSS delta: {}",
                format_bytes(after - before)
            );
        }
    }

    if let Some(rss) = rss_after {
        println!("  lifetime peak RSS: {}", format_bytes(rss));
        if let Some(before) = rss_before {
            if rss > before {
                println!(
                    "  RSS delta (during bench): {}",
                    format_bytes(rss - before)
                );
            }
        }
    } else {
        println!("  peak RSS: unavailable on this platform");
    }
}
