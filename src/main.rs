use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::Instant;
use std::{env, fs};

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    project: String,
    platforms: HashMap<String, PlatformConfig>,
    devices: Vec<DeviceConfig>,
    catalogue: Option<String>,
    fixtures: Option<String>,
    suite: Option<String>,
    credentials: Option<CredentialConfig>,
}

#[derive(Debug, Deserialize)]
struct PlatformConfig {
    package: String,
    runner: String,
    agent_port: u16,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceConfig {
    name: String,
    platform: String,
}

#[derive(Debug, Deserialize)]
struct CredentialConfig {
    email: Option<String>,
    password: Option<String>,
}

fn load_config(path: &str) -> ProjectConfig {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", path, e);
        process::exit(2);
    });
    serde_yaml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Failed to parse {}: {}", path, e);
        process::exit(2);
    })
}

fn resolve_env(s: &str) -> String {
    if s.starts_with("${") && s.ends_with('}') {
        let var = &s[2..s.len() - 1];
        env::var(var).unwrap_or_default()
    } else {
        s.to_string()
    }
}

fn find_project_yaml() -> String {
    let candidates = [
        "project.yaml",
        "tctl.yaml",
        "examples/naturkartan.yaml",
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    eprintln!("No project.yaml found. Specify with --project <path>");
    process::exit(2);
}

fn shell_check(cmd: &str) -> (i32, String) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output();
    match output {
        Ok(o) => {
            let code = o.status.code().unwrap_or(-1);
            let out = String::from_utf8_lossy(&o.stdout).to_string()
                + &String::from_utf8_lossy(&o.stderr);
            (code, out)
        }
        Err(e) => (-1, format!("exec error: {}", e)),
    }
}

// ── tctl doctor ──

fn cmd_doctor(config: &ProjectConfig) {
    let mut passed = 0;
    let mut failed = 0;

    println!("tctl doctor — project: {}", config.project);
    println!();

    for dev in &config.devices {
        println!("── {} ({}) ──", dev.name, dev.platform);
        let platform = config.platforms.get(&dev.platform);
        let Some(plat) = platform else {
            println!("  SKIP  platform '{}' not configured", dev.platform);
            continue;
        };

        // 1. device_connected
        let cmd = format!("{} devices 2>&1", plat.runner);
        let (code, out) = shell_check(&cmd);
        if code == 0 && out.contains(&dev.name) {
            println!("  PASS  device_connected");
            passed += 1;
        } else {
            println!("  FAIL  device_connected — '{}' not found", dev.name);
            failed += 1;
        }

        // 2. wda_ready (iOS only)
        if dev.platform == "ios" {
            let cmd = format!("{} wda status {} 2>&1", plat.runner, dev.name);
            let (_, out) = shell_check(&cmd);
            if out.contains("READY") {
                println!("  PASS  wda_ready");
                passed += 1;
            } else {
                println!("  FAIL  wda_ready — WDA not responding");
                failed += 1;
            }
        }

        // 3. agent_health
        let host = if dev.platform == "ios" { "192.168.1.114" } else { "localhost" };
        let cmd = format!(
            "curl -s --connect-timeout 2 --max-time 5 http://{}:{}/health",
            host, plat.agent_port
        );
        let (code, out) = shell_check(&cmd);
        if code == 0 && out.contains("ok") {
            println!("  PASS  agent_health");
            passed += 1;
        } else {
            println!("  FAIL  agent_health — agent not responding on port {}", plat.agent_port);
            failed += 1;
        }

        // 4. idle_resources
        let idle_endpoint = if dev.platform == "ios" { "idle" } else { "idle-resources" };
        let cmd = format!(
            "curl -s --connect-timeout 2 --max-time 5 http://{}:{}/{}",
            host, plat.agent_port, idle_endpoint
        );
        let (code, out) = shell_check(&cmd);
        if code == 0 && (out.contains("navigation") || out.contains("ui_thread") || out.contains("idle")) {
            println!("  PASS  idle_resources");
            passed += 1;
        } else {
            println!("  FAIL  idle_resources — agent missing idle resource registry");
            failed += 1;
        }

        // 5. battery_saver (Android only)
        if dev.platform == "android" {
            let cmd = format!(
                "{} adb -d {} shell settings get global low_power 2>&1",
                plat.runner, dev.name
            );
            let (_, out) = shell_check(&cmd);
            if out.trim() == "0" {
                println!("  PASS  battery_saver_off");
                passed += 1;
            } else {
                println!("  FAIL  battery_saver_off — battery saver is ON");
                failed += 1;
            }
        }

        println!();
    }

    // Global checks
    println!("── global ──");

    // 6. credentials
    let email_var = config.credentials.as_ref()
        .and_then(|c| c.email.as_deref())
        .unwrap_or("${IDB_TEST_EMAIL}");
    let email = resolve_env(email_var);
    if !email.is_empty() {
        println!("  PASS  credentials (email set)");
        passed += 1;
    } else {
        println!("  FAIL  credentials — email env var not set");
        failed += 1;
    }

    // 7. fixtures
    if let Some(fix_path) = &config.fixtures {
        if Path::new(fix_path).exists() {
            println!("  PASS  fixtures ({})", fix_path);
            passed += 1;
        } else {
            println!("  FAIL  fixtures — {} not found", fix_path);
            failed += 1;
        }
    }

    // 8. runner binary freshness + PATH resolution
    let mut checked_runners: Vec<String> = Vec::new();
    for (plat_name, plat) in &config.platforms {
        if checked_runners.contains(&plat.runner) { continue; }
        checked_runners.push(plat.runner.clone());

        let (_, which_out) = shell_check(&format!("which -a {} 2>/dev/null", plat.runner));
        let paths: Vec<&str> = which_out.trim().lines().collect();
        if paths.is_empty() {
            println!("  FAIL  runner_path ({}) — not found in PATH", plat.runner);
            failed += 1;
            continue;
        }
        let primary = paths[0];
        println!("  PASS  runner_path ({}) → {}", plat.runner, primary);
        passed += 1;
        if paths.len() > 1 {
            println!("  WARN  runner_path ({}) — {} copies in PATH: {}", plat.runner, paths.len(), paths.join(", "));
        }

        if let Some(src) = &plat.source {
            let bin_meta = fs::metadata(primary);
            let src_path = Path::new(src);
            let src_newest = find_newest_source(src_path);
            if let (Ok(bin_m), Some(src_t)) = (bin_meta, src_newest) {
                if let Ok(bin_t) = bin_m.modified() {
                    let bin_dur = bin_t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    if src_t > bin_dur {
                        println!("  FAIL  runner_fresh ({}) — binary older than source", plat.runner);
                        failed += 1;
                    } else {
                        println!("  PASS  runner_fresh ({}) — binary up to date", plat.runner);
                        passed += 1;
                    }
                }
            }
        }
    }

    println!();
    println!("{} passed, {} failed", passed, failed);
    if failed > 0 {
        process::exit(1);
    }
}

fn find_newest_source(dir: &Path) -> Option<std::time::Duration> {
    let mut newest: Option<std::time::Duration> = None;
    let Ok(entries) = fs::read_dir(dir) else { return None };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.starts_with('.') || name == "target" || name == "build" || name == ".build" || name == "DerivedData" { continue; }
        if path.is_dir() {
            if let Some(t) = find_newest_source(&path) {
                if newest.map_or(true, |n| t > n) { newest = Some(t); }
            }
        } else if let Ok(meta) = fs::metadata(&path) {
            if let Ok(mod_time) = meta.modified() {
                let dur = mod_time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                if newest.map_or(true, |n| dur > n) { newest = Some(dur); }
            }
        }
    }
    newest
}

// ── tctl run ──

fn cmd_run(config: &ProjectConfig, args: &[String]) {
    let mut device_filter: Option<&str> = None;
    let mut platform_filter: Option<&str> = None;
    let mut suite_override: Option<&str> = None;
    let mut capture_baseline = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--device" => {
                i += 1;
                device_filter = args.get(i).map(|s| s.as_str());
            }
            "-p" | "--platform" => {
                i += 1;
                platform_filter = args.get(i).map(|s| s.as_str());
            }
            "--suite" => {
                i += 1;
                suite_override = args.get(i).map(|s| s.as_str());
            }
            "--capture-baseline" => capture_baseline = true,
            _ => {}
        }
        i += 1;
    }

    let suite = suite_override
        .map(|s| s.to_string())
        .or_else(|| config.suite.clone())
        .unwrap_or_else(|| {
            eprintln!("No suite path. Use --suite <path> or set 'suite' in project.yaml");
            process::exit(1);
        });

    let tc_files = collect_tc_files(&suite);
    if tc_files.is_empty() {
        eprintln!("No TC YAML files found in {}", suite);
        process::exit(1);
    }

    println!("tctl run — project: {}, suite: {}, TCs: {}", config.project, suite, tc_files.len());

    let devices: Vec<&DeviceConfig> = config.devices.iter().filter(|d| {
        if let Some(df) = device_filter {
            return d.name == df;
        }
        if let Some(pf) = platform_filter {
            return d.platform == pf;
        }
        true
    }).collect();

    if devices.is_empty() {
        eprintln!("No devices match filter");
        process::exit(1);
    }

    for dev in &devices {
        let Some(plat) = config.platforms.get(&dev.platform) else {
            eprintln!("Platform '{}' not configured for device '{}'", dev.platform, dev.name);
            continue;
        };

        println!("\n── {} ({}) ──", dev.name, dev.platform);
        let start = Instant::now();

        let catalogue_arg = if dev.platform == "ios" {
            config.catalogue.as_ref()
                .map(|c| format!(" --catalogue {}", c))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let baseline_arg = if capture_baseline { " --capture-baseline" } else { "" };

        let tc_list = tc_files.iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        let test_subcmd = if dev.platform == "ios" { "test run" } else { "test" };

        let cmd = format!(
            "{} {} -d {}{}{} {}",
            plat.runner, test_subcmd, dev.name, catalogue_arg, baseline_arg, tc_list
        );

        eprintln!("exec: {}", cmd);
        let status = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .status();

        let elapsed = start.elapsed();
        match status {
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                println!("  exit: {} ({:.1}s)", code, elapsed.as_secs_f64());
            }
            Err(e) => {
                println!("  exec error: {} ({:.1}s)", e, elapsed.as_secs_f64());
            }
        }
    }
}

fn collect_tc_files(suite_path: &str) -> Vec<PathBuf> {
    let path = Path::new(suite_path);
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    if !path.is_dir() {
        return vec![];
    }
    let mut files: Vec<PathBuf> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false)
                && !p.file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

// ── main ──

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: tctl <command> [options]");
        eprintln!("Commands: run, doctor");
        eprintln!("Options: --project <path>  (default: project.yaml)");
        process::exit(1);
    }

    let mut project_path: Option<String> = None;
    let mut command_name = String::new();
    let mut remaining_args_start = args.len();
    let mut skip_next = false;
    for i in 1..args.len() {
        if skip_next { skip_next = false; continue; }
        if args[i] == "--project" && i + 1 < args.len() {
            project_path = Some(args[i + 1].clone());
            skip_next = true;
        } else if command_name.is_empty() && !args[i].starts_with('-') {
            command_name = args[i].clone();
            remaining_args_start = i + 1;
        }
    }

    if command_name.is_empty() {
        eprintln!("Usage: tctl <command> [options]");
        process::exit(1);
    }

    let command = &command_name;
    let config_path = project_path.unwrap_or_else(|| find_project_yaml());
    let config = load_config(&config_path);

    match command.as_str() {
        "run" => cmd_run(&config, &args[remaining_args_start..]),
        "doctor" => cmd_doctor(&config),
        _ => {
            eprintln!("Unknown command: {}", command);
            process::exit(1);
        }
    }
}
