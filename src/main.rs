mod generate;

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
    hooks: Option<HooksConfig>,
}

#[derive(Debug, Deserialize)]
struct HooksConfig {
    pre_run: Option<String>,
    post_run: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlatformConfig {
    package: String,
    runner: String,
    agent_port: u16,
    source: Option<String>,
    runner_prefix: Option<String>,
    home_tab: Option<String>,
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

fn resolve_relative(base_dir: &Path, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() { path.to_path_buf() } else { base_dir.join(p) }
}

fn cmd_doctor(config: &ProjectConfig, config_dir: &Path) {
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
        let resolved = resolve_relative(config_dir, fix_path);
        if resolved.exists() {
            println!("  PASS  fixtures ({})", resolved.display());
            passed += 1;
        } else {
            println!("  FAIL  fixtures — {} not found", resolved.display());
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

fn cmd_run(config: &ProjectConfig, config_dir: &Path, args: &[String]) {
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

    let suite_raw = suite_override
        .map(|s| s.to_string())
        .or_else(|| config.suite.clone())
        .unwrap_or_else(|| {
            eprintln!("No suite path. Use --suite <path> or set 'suite' in project.yaml");
            process::exit(1);
        });
    let suite = resolve_relative(config_dir, &suite_raw).to_string_lossy().to_string();

    let tc_files = collect_tc_files(&suite);
    if tc_files.is_empty() {
        eprintln!("No TC YAML files found in {}", suite);
        process::exit(1);
    }

    println!("tctl run — project: {}, suite: {}, TCs: {}", config.project, suite, tc_files.len());

    // pre_run hook
    if let Some(ref hooks) = config.hooks {
        if let Some(ref pre_run) = hooks.pre_run {
            let script = resolve_relative(config_dir, pre_run);
            println!("pre_run: {}", script.display());
            let status = Command::new("sh").arg("-c").arg(script.to_string_lossy().as_ref()).status();
            match status {
                Ok(s) if s.success() => println!("pre_run: OK"),
                Ok(s) => {
                    eprintln!("pre_run FAILED (exit {}). Aborting.", s.code().unwrap_or(-1));
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("pre_run exec error: {}. Aborting.", e);
                    process::exit(1);
                }
            }
        }
    }

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

        let email = config.credentials.as_ref()
            .and_then(|c| c.email.as_deref())
            .map(|e| resolve_env(e))
            .unwrap_or_default();
        let password = config.credentials.as_ref()
            .and_then(|c| c.password.as_deref())
            .map(|p| resolve_env(p))
            .unwrap_or_default();
        let email_var = if dev.platform == "ios" { "IDB_TEST_EMAIL" } else { "DDB_TEST_EMAIL" };
        let pass_var = if dev.platform == "ios" { "IDB_TEST_PASSWORD" } else { "DDB_TEST_PASSWORD" };
        let fixtures_var = if dev.platform == "ios" { "IDB_FIXTURES_PATH" } else { "DDB_FIXTURES_PATH" };
        let fixtures_abs = config.fixtures.as_ref()
            .map(|f| resolve_relative(config_dir, f).to_string_lossy().to_string())
            .unwrap_or_default();
        let agent_port_var = if dev.platform == "ios" { "IDB_AGENT_PORT" } else { "DDB_AGENT_PORT" };

        // Inline env vars in command string (nosandbox uses tmux run-shell, doesn't inherit env)
        let mut env_prefix = String::new();
        if !email.is_empty() { env_prefix.push_str(&format!("{}={} ", email_var, email)); }
        if !password.is_empty() { env_prefix.push_str(&format!("{}={} ", pass_var, password)); }
        if !fixtures_abs.is_empty() { env_prefix.push_str(&format!("{}={} ", fixtures_var, fixtures_abs)); }
        env_prefix.push_str(&format!("{}={} ", agent_port_var, plat.agent_port));
        if let Some(ref tab) = plat.home_tab {
            let tab_var = if dev.platform == "ios" { "IDB_HOME_TAB" } else { "DDB_HOME_TAB" };
            env_prefix.push_str(&format!("{}={} ", tab_var, tab));
        }

        let prefix = plat.runner_prefix.as_deref().map(|p| format!("{} ", p)).unwrap_or_default();
        let cmd = format!(
            "{}{}{} {} -d {}{}{} {}",
            prefix, env_prefix, plat.runner, test_subcmd, dev.name, catalogue_arg, baseline_arg, tc_list
        );

        eprintln!("exec: {}", cmd);
        let status = Command::new("sh").arg("-c").arg(&cmd).status();

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

    // post_run hook
    if let Some(ref hooks) = config.hooks {
        if let Some(ref post_run) = hooks.post_run {
            let script = resolve_relative(config_dir, post_run);
            println!("post_run: {}", script.display());
            let status = Command::new("sh").arg("-c").arg(script.to_string_lossy().as_ref()).status();
            match status {
                Ok(s) if s.success() => println!("post_run: OK"),
                Ok(s) => eprintln!("post_run FAILED (exit {}). Warning only.", s.code().unwrap_or(-1)),
                Err(e) => eprintln!("post_run exec error: {}. Warning only.", e),
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

// ── tctl validate ──

fn cmd_validate(args: &[String]) {
    let mut spec_path: Option<&str> = None;
    let mut fixtures_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixtures" if i + 1 < args.len() => { fixtures_path = Some(args[i + 1].clone()); i += 2; }
            "--project" if i + 1 < args.len() => { i += 2; } // skip, handled by main
            s if !s.starts_with('-') && spec_path.is_none() => { spec_path = Some(s); i += 1; }
            _ => { i += 1; }
        }
    }
    let spec_path = spec_path.unwrap_or_else(|| {
        eprintln!("Usage: tctl validate <spec.yaml> [--fixtures <path>]");
        process::exit(1);
    });

    let content = fs::read_to_string(spec_path).unwrap_or_else(|e| {
        eprintln!("FAIL  read: {}: {}", spec_path, e);
        process::exit(2);
    });
    let spec: serde_yaml::Value = serde_yaml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("FAIL  parse: {}: {}", spec_path, e);
        process::exit(2);
    });

    let mut passed = 0;
    let mut failed = 0;
    let mut warnings = 0;

    // 1. Top-level structure
    let ticket = spec.get("ticket");
    if ticket.is_some() && ticket.unwrap().get("id").is_some() {
        println!("  PASS  ticket.id present");
        passed += 1;
    } else {
        println!("  ERROR  ticket.id missing");
        failed += 1;
    }

    let frs = spec.get("functional_requirements").and_then(|v| v.as_sequence());
    let fr_list = match frs {
        Some(list) if !list.is_empty() => {
            println!("  PASS  functional_requirements: {} FRs", list.len());
            passed += 1;
            list
        }
        _ => {
            println!("  ERROR  functional_requirements missing or empty");
            failed += 1;
            process::exit(1);
        }
    };

    // Collect all state keys for requires resolution (pre-pass)
    let mut all_state_keys: Vec<String> = Vec::new();
    for i in 0..fr_list.len() {
        let fr_id = fr_list[i].get("id").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(states) = fr_list[i].get("state_table").or_else(|| fr_list[i].get("states")).and_then(|v| v.as_sequence()) {
            for state in states {
                let sid = state.get("id").and_then(|v| v.as_str()).unwrap_or("");
                all_state_keys.push(format!("{}-{}", fr_id, sid));
            }
        }
    }

    // 2. Per-FR validation
    println!("  INFO  validating {} FRs...", fr_list.len());
    for i in 0..fr_list.len() {
        let fr = &fr_list[i];
        let fr_id = fr.get("id").and_then(|v| v.as_str()).unwrap_or("??");
        let fr_name = fr.get("name").and_then(|v| v.as_str()).unwrap_or("??");

        // FR has id + name
        if fr_id == "??" || fr_name == "??" {
            println!("  ERROR  FR missing id or name");
            failed += 1;
            continue;
        }

        // FR has type
        let fr_type = fr.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if fr_type != "static" && fr_type != "interactive" {
            println!("  ERROR  {}: type must be 'static' or 'interactive', got '{}'", fr_id, fr_type);
            failed += 1;
        }

        // FR has ≥1 state
        let states = fr.get("state_table").or_else(|| fr.get("states")).and_then(|v| v.as_sequence());
        match states {
            Some(list) if !list.is_empty() => {
                passed += 1;
            }
            _ => {
                println!("  ERROR  {}: no states in state_table", fr_id);
                failed += 1;
                continue;
            }
        }
        let states = states.unwrap();

        for state in states {
            let sid = state.get("id").and_then(|v| v.as_str()).unwrap_or("??");
            let full_id = format!("{}-{}", fr_id, sid);

            // State has renders
            let renders = state.get("renders").and_then(|v| v.as_str()).unwrap_or("");
            if renders.is_empty() {
                println!("  ERROR  {}: renders is empty", full_id);
                failed += 1;
            }

            // State has assert_target — predict generator fallback chain
            let assert_target = state.get("assert_target").and_then(|v| v.as_str()).unwrap_or("");
            if !assert_target.is_empty() && assert_target.contains('_') && !assert_target.contains(' ') && assert_target == assert_target.to_lowercase() {
                let state_name = state.get("state").and_then(|v| v.as_str()).unwrap_or("??");
                println!("  WARN   {}: assert_target '{}' is a code identifier — generator will fall back to state name '{}' (likely fails). Add visible UI text.", full_id, assert_target, state_name);
                warnings += 1;
            }
            if assert_target.is_empty() {
                let has_quoted = renders.contains('\'') || renders.contains('"');
                if has_quoted {
                    println!("  WARN  {}: no assert_target — generator will extract from renders quoted strings", full_id);
                } else {
                    let state_name = state.get("state").and_then(|v| v.as_str()).unwrap_or("??");
                    println!("  WARN  {}: no assert_target and no quoted text in renders — generator will fall back to state name '{}' (likely fails). Add assert_target.", full_id, state_name);
                }
                warnings += 1;
            }

            // No MISSING/placeholder values
            let condition = state.get("condition").and_then(|v| v.as_str()).unwrap_or("");
            for field_val in [renders, assert_target, condition] {
                if field_val.to_uppercase().contains("MISSING") || field_val.contains("TODO") || field_val.contains("TBD") {
                    println!("  ERROR  {}: placeholder value detected: '{}'", full_id, &field_val[..field_val.len().min(50)]);
                    failed += 1;
                }
            }

            // Requires references resolve
            if let Some(req) = state.get("requires").and_then(|v| v.as_str()) {
                if !req.is_empty() {
                    for r in req.split(',') {
                        let r = r.trim();
                        if !r.is_empty() && !all_state_keys.contains(&r.to_string()) {
                            println!("  ERROR  {}: requires '{}' does not resolve to any state", full_id, r);
                            failed += 1;
                        }
                    }
                }
            }

            // Fixture ref check
            if let Some(fixture) = state.get("fixture").and_then(|v| v.as_str()) {
                if fixture.to_uppercase() == "MISSING" || fixture.contains("TODO") {
                    println!("  ERROR  {}: fixture is placeholder: '{}'", full_id, fixture);
                    failed += 1;
                }
            }
        }
    }

    // 3. Fixtures section
    if let Some(fixtures) = spec.get("fixtures") {
        if let Some(required) = fixtures.get("required").and_then(|v| v.as_sequence()) {
            println!("  PASS  fixtures.required: {} entries", required.len());
            passed += 1;

            // Check fixture keys against fixtures.yaml if provided
            if let Some(ref fp) = fixtures_path {
                if Path::new(fp).exists() {
                    let fix_content = fs::read_to_string(fp).unwrap_or_default();
                    for entry in required {
                        let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("");
                        if !key.is_empty() && !fix_content.contains(key) {
                            println!("  WARN  fixture '{}' not found in {}", key, fp);
                            warnings += 1;
                        }
                    }
                }
            }
        }
    }

    // 4. Coverage summary
    if let Some(coverage) = spec.get("coverage_summary") {
        let total_frs = coverage.get("total_frs").and_then(|v| v.as_u64()).unwrap_or(0);
        let total_states = coverage.get("total_states").and_then(|v| v.as_u64()).unwrap_or(0);
        if total_frs as usize == fr_list.len() {
            println!("  PASS  coverage_summary.total_frs matches ({} == {})", total_frs, fr_list.len());
            passed += 1;
        } else {
            println!("  ERROR  coverage_summary.total_frs mismatch ({} != {})", total_frs, fr_list.len());
            failed += 1;
        }
        let actual_states: usize = fr_list.iter()
            .map(|fr| fr.get("state_table").or_else(|| fr.get("states")).and_then(|v| v.as_sequence()).map(|s| s.len()).unwrap_or(0))
            .sum();
        if total_states as usize == actual_states {
            println!("  PASS  coverage_summary.total_states matches ({} == {})", total_states, actual_states);
            passed += 1;
        } else {
            println!("  ERROR  coverage_summary.total_states mismatch ({} != {})", total_states, actual_states);
            failed += 1;
        }
    }

    println!("\n{} passed, {} errors, {} warnings", passed, failed, warnings);
    if failed > 0 {
        process::exit(1);
    }
}

// ── tctl generate ──

fn cmd_generate(args: &[String]) {
    let mut spec_path: Option<&str> = None;
    let mut output_dir = "generated".to_string();
    let mut baseline_dir: Option<String> = None;
    let mut _platform: Option<String> = None;
    let mut package: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output-dir" if i + 1 < args.len() => { output_dir = args[i + 1].clone(); i += 2; }
            "--baseline-dir" if i + 1 < args.len() => { baseline_dir = Some(args[i + 1].clone()); i += 2; }
            "--platform" if i + 1 < args.len() => { _platform = Some(args[i + 1].clone()); i += 2; }
            "--package" if i + 1 < args.len() => { package = Some(args[i + 1].clone()); i += 2; }
            "--project" if i + 1 < args.len() => { i += 2; }
            s if !s.starts_with('-') && spec_path.is_none() => { spec_path = Some(s); i += 1; }
            _ => { i += 1; }
        }
    }

    let spec_path = spec_path.unwrap_or_else(|| {
        eprintln!("Usage: tctl generate <spec.yaml> [--output-dir <dir>] [--baseline-dir <dir>] [--platform <android|ios>]");
        process::exit(1);
    });

    let content = fs::read_to_string(spec_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", spec_path, e);
        process::exit(2);
    });
    let spec: generate::SpecInput = serde_yaml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Failed to parse {}: {}", spec_path, e);
        process::exit(2);
    });

    let baseline_path = baseline_dir.map(|d| PathBuf::from(d));
    let tcs = generate::generate_journeys(&spec, baseline_path.as_deref(), package.as_deref(), _platform.as_deref());

    let out = Path::new(&output_dir);
    match generate::write_tcs(&tcs, out) {
        Ok(n) => println!("{} TCs generated to {}", n, output_dir),
        Err(e) => {
            eprintln!("Failed to write TCs: {}", e);
            process::exit(2);
        }
    }
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

    if command == "validate" {
        cmd_validate(&args[remaining_args_start..]);
        return;
    }

    if command == "generate" {
        cmd_generate(&args[remaining_args_start..]);
        return;
    }

    let config_path = project_path.unwrap_or_else(|| find_project_yaml());
    let config_dir = Path::new(&config_path).parent().unwrap_or(Path::new(".")).to_path_buf();
    let config = load_config(&config_path);

    match command.as_str() {
        "run" => cmd_run(&config, &config_dir, &args[remaining_args_start..]),
        "doctor" => cmd_doctor(&config, &config_dir),
        _ => {
            eprintln!("Unknown command: {}", command);
            process::exit(1);
        }
    }
}
