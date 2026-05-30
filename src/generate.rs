use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

// ── Spec structs (deserialized from preflight-output.yaml) ──

#[derive(Debug, Deserialize)]
pub struct SpecInput {
    pub ticket: Option<TicketMeta>,
    pub functional_requirements: Vec<FunctionalRequirement>,
    pub journeys: Option<Vec<JourneyDef>>,
}

#[derive(Debug, Deserialize)]
pub struct JourneyDef {
    pub id: String,
    pub name: String,
    pub auth: Option<bool>,
    pub steps: Vec<JourneyStepDef>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum JourneyStepDef {
    Assert { assert: AssertRef },
    SearchSite { search_site: String },
    Tap { tap: String, element_type: Option<String> },
    Type { r#type: String },
    ScrollTo { scroll_to: String },
    ScrollToAssert { scroll_to_assert: AssertRef },
    LongPress { long_press: String },
    Wait { wait: u64 },
    WaitFor { wait_for: Vec<String>, timeout: Option<u64> },
    PressBack { press_back: bool },
    PlatformFork { platform_fork: HashMap<String, Vec<JourneyStepDef>> },
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssertRef {
    pub fr: String,
    pub state: String,
    #[serde(rename = "type")]
    pub assert_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TicketMeta {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FunctionalRequirement {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub fr_type: Option<String>,
    pub navigation: Option<Navigation>,
    #[serde(alias = "states")]
    pub state_table: Vec<StateEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Navigation {
    pub target: Option<String>,
    pub via: Option<Vec<String>>,
    pub baseline_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StateEntry {
    pub id: String,
    pub state: String,
    pub condition: Option<String>,
    pub renders: Option<String>,
    pub assert_target: Option<String>,
    pub assert_type: Option<String>,
    pub fixture: Option<String>,
    pub requires: Option<String>,
    pub wait_for: Option<Vec<String>>,
    pub platform_fork: Option<HashMap<String, String>>,
}

// ── Baseline structs ──

#[derive(Debug, Deserialize)]
struct BaselineFile {
    elements: Option<Vec<BaselineElement>>,
}

#[derive(Debug, Deserialize)]
struct BaselineElement {
    content: Option<String>,
    #[serde(rename = "type")]
    _type: Option<String>,
}

// ── Generated TC ──

#[derive(Debug)]
pub struct GeneratedStep {
    pub action: String,
    pub site_id: Option<String>,
    pub user_id: Option<String>,
    pub target_fuzzy: Option<String>,
    pub element_type: Option<String>,
    pub text: Option<String>,
    pub seconds: Option<u64>,
    pub wait_for: Option<Vec<String>>,
    pub wait_timeout: Option<u64>,
    pub assert_type: Option<String>,
    pub source_comment: Option<String>,
}

#[derive(Debug)]
pub struct GeneratedTC {
    pub id: String,
    pub name: String,
    pub precondition_activity: String,
    pub precondition_package: Option<String>,
    pub precondition_logged_in: bool,
    pub steps: Vec<GeneratedStep>,
}

// ── Navigation target → baseline file mapping ──

const BASELINE_FILE_MAP: &[(&str, &[&str])] = &[
    ("site_detail", &["site-detail-scrolled.yaml", "site-detail.yaml"]),
    ("questions_list", &["questions-list.yaml"]),
    ("post_question", &["post-question-form.yaml"]),
    ("post_answer", &["questions-list.yaml"]),
    ("profile_qa_tab", &["profile-qa-scrolled.yaml", "profile-qa.yaml", "profile-dashboard.yaml"]),
];

// ── Mapping functions ──

pub fn is_code_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.contains('_') && !s.contains(' ') && s == s.to_lowercase()
}

pub fn find_baseline_match(baseline_contents: &[String], assert_target: &str, renders: &str) -> Option<String> {
    if baseline_contents.is_empty() {
        return None;
    }
    let target_lower = assert_target.to_lowercase();

    // Exact match
    for c in baseline_contents {
        if c.to_lowercase() == target_lower {
            return Some(c.clone());
        }
    }
    // Substring match on assert_target
    if !target_lower.is_empty() {
        for c in baseline_contents {
            if c.to_lowercase().contains(&target_lower) {
                return Some(c.clone());
            }
        }
    }
    // Match from quoted strings in renders
    for q in extract_quoted(renders) {
        let q_lower = q.to_lowercase();
        for c in baseline_contents {
            if c.to_lowercase().contains(&q_lower) {
                return Some(c.clone());
            }
        }
    }
    None
}

pub fn resolve_assert(
    state: &StateEntry,
    baseline_contents: &[String],
) -> (String, String, String) {
    let atype = state.assert_type.as_deref().unwrap_or("element_exists").to_string();
    let atarget = state.assert_target.as_deref().unwrap_or("");
    let renders = state.renders.as_deref().unwrap_or("");

    // 1. Baseline match
    if let Some(m) = find_baseline_match(baseline_contents, atarget, renders) {
        return (atype, m, "baseline".into());
    }
    // 2. Spec assert_target (if real UI text)
    if !atarget.is_empty() && !is_code_identifier(atarget) {
        return (atype, atarget.to_string(), "spec".into());
    }
    // 3. Quoted strings in renders
    let quoted = extract_quoted(renders);
    if let Some(first) = quoted.into_iter().next() {
        return (atype, first, "renders".into());
    }
    // 4. State name fallback for code identifiers
    if is_code_identifier(atarget) && !state.state.is_empty() {
        return (atype, state.state.clone(), "state_name".into());
    }
    // 5. Last resort
    let fallback = if !atarget.is_empty() { atarget.to_string() } else { state.state.clone() };
    (atype, fallback, "fallback".into())
}

fn extract_quoted(s: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' || c == '\'' {
            let quote = c;
            chars.next();
            let mut buf = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == quote {
                    chars.next();
                    break;
                }
                buf.push(nc);
                chars.next();
            }
            if !buf.is_empty() {
                results.push(buf);
            }
        } else {
            chars.next();
        }
    }
    results
}

// ── Baseline loading ──

pub fn extract_baseline_contents(path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let data: BaselineFile = match serde_yaml::from_str(&content) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let elements = data.elements.unwrap_or_default();
    elements.iter().filter_map(|e| {
        let c = e.content.as_deref().unwrap_or("");
        if c.len() > 2 && !c.starts_with("ic_") && !c.starts_with("btn_") {
            Some(c.to_string())
        } else {
            None
        }
    }).collect()
}

pub fn load_baseline(baseline_dir: &Path, fr: &FunctionalRequirement) -> Vec<String> {
    // 1. FR-level baseline_file override from spec
    if let Some(ref nav) = fr.navigation {
        if let Some(ref bf) = nav.baseline_file {
            let p = baseline_dir.join(bf);
            if p.exists() {
                return extract_baseline_contents(&p);
            }
        }
    }
    // 2. Navigation target → baseline file map
    if let Some(ref nav) = fr.navigation {
        if let Some(ref target) = nav.target {
            for &(key, files) in BASELINE_FILE_MAP {
                if key == target.as_str() {
                    for f in files {
                        let p = baseline_dir.join(f);
                        if p.exists() {
                            return extract_baseline_contents(&p);
                        }
                    }
                }
            }
        }
    }
    // 3. All baselines in directory
    let mut contents = Vec::new();
    if let Ok(entries) = fs::read_dir(baseline_dir) {
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.extension().map(|e| e == "yaml").unwrap_or(false) {
                contents.extend(extract_baseline_contents(&p));
            }
        }
    }
    contents
}

// ── Journey generation ──

fn resolve_checkpoint(
    fr_id: &str,
    state_id: &str,
    state_lookup: &HashMap<String, (&FunctionalRequirement, &StateEntry)>,
    baseline_dir: Option<&Path>,
) -> GeneratedStep {
    let key = format!("{}-{}", fr_id, state_id);
    if let Some((fr, state)) = state_lookup.get(&key) {
        let baseline_contents = baseline_dir
            .map(|d| load_baseline(d, fr))
            .unwrap_or_default();
        let (assert_type, assert_text, assert_source) = resolve_assert(state, &baseline_contents);
        GeneratedStep {
            action: "assert".into(),
            assert_type: Some(assert_type),
            target_fuzzy: Some(assert_text),
            source_comment: Some(format!("{} ({})", key, assert_source)),
            ..step_defaults()
        }
    } else {
        GeneratedStep {
            action: "assert".into(),
            assert_type: Some("element_exists".into()),
            target_fuzzy: Some(format!("UNRESOLVED:{}", key)),
            source_comment: Some(format!("{} (unresolved)", key)),
            ..step_defaults()
        }
    }
}

fn expand_journey_step(
    step_def: &JourneyStepDef,
    state_lookup: &HashMap<String, (&FunctionalRequirement, &StateEntry)>,
    baseline_dir: Option<&Path>,
    platform: Option<&str>,
    steps: &mut Vec<GeneratedStep>,
) {
    match step_def {
        JourneyStepDef::Assert { assert: aref } => {
            steps.push(resolve_checkpoint(&aref.fr, &aref.state, state_lookup, baseline_dir));
        }
        JourneyStepDef::SearchSite { search_site } => {
            let name_ref = format!("{{{{fixtures.{}.name}}}}", search_site);
            steps.push(GeneratedStep {
                action: "tap".into(),
                target_fuzzy: Some("Search".into()),
                element_type: Some("Button".into()),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(3), ..step_defaults() });
            steps.push(GeneratedStep {
                action: "type".into(),
                text: Some(name_ref.clone()),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(3), ..step_defaults() });
            steps.push(GeneratedStep {
                action: "tap".into(),
                target_fuzzy: Some(name_ref),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(5), ..step_defaults() });
        }
        JourneyStepDef::Tap { tap, element_type } => {
            steps.push(GeneratedStep {
                action: "tap".into(),
                target_fuzzy: Some(tap.clone()),
                element_type: element_type.clone(),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(3), ..step_defaults() });
        }
        JourneyStepDef::Type { r#type } => {
            steps.push(GeneratedStep {
                action: "type".into(),
                text: Some(r#type.clone()),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(1), ..step_defaults() });
        }
        JourneyStepDef::ScrollTo { scroll_to } => {
            steps.push(GeneratedStep {
                action: "scroll_to".into(),
                target_fuzzy: Some(scroll_to.clone()),
                ..step_defaults()
            });
        }
        JourneyStepDef::ScrollToAssert { scroll_to_assert } => {
            let checkpoint = resolve_checkpoint(&scroll_to_assert.fr, &scroll_to_assert.state, state_lookup, baseline_dir);
            if let Some(ref target) = checkpoint.target_fuzzy {
                steps.push(GeneratedStep {
                    action: "scroll_to".into(),
                    target_fuzzy: Some(target.clone()),
                    ..step_defaults()
                });
            }
            steps.push(checkpoint);
        }
        JourneyStepDef::LongPress { long_press } => {
            steps.push(GeneratedStep {
                action: "long_press".into(),
                target_fuzzy: Some(long_press.clone()),
                ..step_defaults()
            });
            steps.push(GeneratedStep { action: "wait_idle".into(), seconds: Some(3), ..step_defaults() });
        }
        JourneyStepDef::Wait { wait } => {
            steps.push(GeneratedStep {
                action: "wait_idle".into(),
                seconds: Some(*wait),
                ..step_defaults()
            });
        }
        JourneyStepDef::WaitFor { wait_for, timeout } => {
            steps.push(GeneratedStep {
                action: "wait_idle".into(),
                seconds: Some(timeout.unwrap_or(10)),
                wait_for: Some(wait_for.clone()),
                wait_timeout: Some(timeout.unwrap_or(10)),
                ..step_defaults()
            });
        }
        JourneyStepDef::PressBack { .. } => {
            steps.push(GeneratedStep {
                action: "press_back".into(),
                ..step_defaults()
            });
        }
        JourneyStepDef::PlatformFork { platform_fork } => {
            let plat = platform.unwrap_or("android");
            if let Some(plat_steps) = platform_fork.get(plat) {
                for ps in plat_steps {
                    expand_journey_step(ps, state_lookup, baseline_dir, platform, steps);
                }
            }
        }
    }
}

pub fn generate_journeys(
    spec: &SpecInput,
    baseline_dir: Option<&Path>,
    package: Option<&str>,
    platform: Option<&str>,
) -> Vec<GeneratedTC> {
    let journeys = match &spec.journeys {
        Some(j) if !j.is_empty() => j,
        _ => return Vec::new(),
    };

    let mut state_lookup: HashMap<String, (&FunctionalRequirement, &StateEntry)> = HashMap::new();
    for fr in &spec.functional_requirements {
        for state in &fr.state_table {
            let key = format!("{}-{}", fr.id, state.id);
            state_lookup.insert(key, (fr, state));
        }
    }

    let mut tcs = Vec::new();
    for journey in journeys {
        let mut steps = Vec::new();
        for step_def in &journey.steps {
            expand_journey_step(step_def, &state_lookup, baseline_dir, platform, &mut steps);
        }
        tcs.push(GeneratedTC {
            id: journey.id.clone(),
            name: journey.name.clone(),
            precondition_activity: "MainActivity".into(),
            precondition_package: package.map(|s| s.to_string()),
            precondition_logged_in: journey.auth.unwrap_or(false),
            steps,
        });
    }
    tcs
}

// ── YAML output ──

pub fn tc_to_yaml(tc: &GeneratedTC) -> String {
    let mut lines = Vec::new();
    lines.push(format!("id: {}", tc.id));
    lines.push(format!("name: \"{}\"", tc.name));
    lines.push("precondition:".into());
    if let Some(ref pkg) = tc.precondition_package {
        lines.push(format!("  package: {}", pkg));
    }
    lines.push(format!("  activity: {}", tc.precondition_activity));
    if tc.precondition_logged_in {
        lines.push("  logged_in: true".into());
    }
    lines.push("steps:".into());
    for step in &tc.steps {
        if step.action == "assert" {
            if let Some(ref src) = step.source_comment {
                lines.push(format!("  # source: {}", src));
            }
            lines.push(format!("  - assert: {}", step.assert_type.as_deref().unwrap_or("element_exists")));
            if let Some(ref t) = step.target_fuzzy {
                lines.push(format!("    target: {{content_fuzzy: \"{}\"}}", t));
            }
        } else {
            lines.push(format!("  - action: {}", step.action));
            if let Some(ref sid) = step.site_id {
                lines.push(format!("    site_id: {}", sid));
            }
            if let Some(ref uid) = step.user_id {
                lines.push(format!("    user_id: {}", uid));
            }
            if let Some(ref t) = step.target_fuzzy {
                if let Some(ref et) = step.element_type {
                    lines.push(format!("    target: {{content_fuzzy: \"{}\", type: \"{}\"}}", t, et));
                } else {
                    lines.push(format!("    target: {{content_fuzzy: \"{}\"}}", t));
                }
            }
            if let Some(ref t) = step.text {
                lines.push(format!("    text: \"{}\"", t));
            }
            if let Some(s) = step.seconds {
                lines.push(format!("    seconds: {}", s));
            }
            if let Some(ref wf) = step.wait_for {
                let wf_str = wf.join(", ");
                lines.push(format!("    wait_for: [{}]", wf_str));
            }
            if let Some(wt) = step.wait_timeout {
                lines.push(format!("    wait_timeout: {}", wt));
            }
        }
    }
    lines.join("\n") + "\n"
}

pub fn write_tcs(tcs: &[GeneratedTC], output_dir: &Path) -> io::Result<usize> {
    if output_dir.is_dir() {
        for entry in fs::read_dir(output_dir)?.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                fs::remove_file(&p)?;
            }
        }
    }
    fs::create_dir_all(output_dir)?;
    for tc in tcs {
        let filename = format!("{}.yaml", tc.id.to_lowercase().replace('-', "_"));
        let path = output_dir.join(&filename);
        fs::write(&path, tc_to_yaml(tc))?;
    }
    Ok(tcs.len())
}

fn step_defaults() -> GeneratedStep {
    GeneratedStep {
        action: String::new(),
        site_id: None,
        user_id: None,
        target_fuzzy: None,
        element_type: None,
        text: None,
        seconds: None,
        wait_for: None,
        wait_timeout: None,
        assert_type: None,
        source_comment: None,
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_code_identifier() {
        assert!(is_code_identifier("portrait_image_url"));
        assert!(is_code_identifier("ic_user_default"));
        assert!(!is_code_identifier("Questions & Answers"));
        assert!(!is_code_identifier("question card"));
        assert!(!is_code_identifier(""));
        assert!(!is_code_identifier("Profile"));
    }

    #[test]
    fn test_resolve_assert_baseline_match() {
        let baseline = vec![
            "Questions & Answers".to_string(),
            "Post question".to_string(),
            "sinisa".to_string(),
        ];
        let state = StateEntry {
            id: "S1".into(), state: "has-questions".into(),
            condition: None, renders: None,
            assert_target: Some("Questions & Answers".into()),
            assert_type: Some("element_exists".into()),
            fixture: None, requires: None, wait_for: None, platform_fork: None,
        };
        let (_, text, source) = resolve_assert(&state, &baseline);
        assert_eq!(text, "Questions & Answers");
        assert_eq!(source, "baseline");
    }

    #[test]
    fn test_resolve_assert_code_identifier_fallback() {
        let baseline: Vec<String> = vec!["sinisa".into()];
        let state = StateEntry {
            id: "S1".into(), state: "User with avatar".into(),
            condition: None,
            renders: Some("Avatar image (26x26 circle) + name + date".into()),
            assert_target: Some("portrait_image_url".into()),
            assert_type: Some("element_exists".into()),
            fixture: None, requires: None, wait_for: None, platform_fork: None,
        };
        let (_, text, source) = resolve_assert(&state, &baseline);
        // baseline has "sinisa" which doesn't match "portrait_image_url"
        // code identifier → state name fallback
        assert_eq!(text, "User with avatar");
        assert_eq!(source, "state_name");
    }

    #[test]
    fn test_resolve_assert_spec_text() {
        let state = StateEntry {
            id: "S1".into(), state: "org-answer".into(),
            condition: None, renders: None,
            assert_target: Some("organization logo".into()),
            assert_type: Some("element_exists".into()),
            fixture: None, requires: None, wait_for: None, platform_fork: None,
        };
        let (_, text, source) = resolve_assert(&state, &[]);
        assert_eq!(text, "organization logo");
        assert_eq!(source, "spec");
    }

    #[test]
    fn test_resolve_assert_renders_quoted() {
        let state = StateEntry {
            id: "S1".into(), state: "test".into(),
            condition: None,
            renders: Some("Shows 'Write your question' text field".into()),
            assert_target: None,
            assert_type: Some("element_exists".into()),
            fixture: None, requires: None, wait_for: None, platform_fork: None,
        };
        let (_, text, source) = resolve_assert(&state, &[]);
        assert_eq!(text, "Write your question");
        assert_eq!(source, "renders");
    }

    #[test]
    fn test_extract_quoted() {
        let q = extract_quoted("'Write your question' text field and \"submit\"");
        assert_eq!(q, vec!["Write your question", "submit"]);
    }

    #[test]
    fn test_tc_to_yaml_roundtrip() {
        let tc = GeneratedTC {
            id: "J1".into(),
            name: "Browse Q&A".into(),
            precondition_activity: "MainActivity".into(),
            precondition_package: Some("se.naturkartan".into()),
            precondition_logged_in: false,
            steps: vec![
                GeneratedStep {
                    action: "tap".into(),
                    target_fuzzy: Some("Search".into()),
                    element_type: Some("Button".into()),
                    ..step_defaults()
                },
                GeneratedStep {
                    action: "wait_idle".into(),
                    seconds: Some(5),
                    ..step_defaults()
                },
                GeneratedStep {
                    action: "assert".into(),
                    assert_type: Some("element_exists".into()),
                    target_fuzzy: Some("Questions & Answers".into()),
                    source_comment: Some("FR6-S1 (baseline)".into()),
                    ..step_defaults()
                },
            ],
        };
        let yaml = tc_to_yaml(&tc);
        assert!(yaml.contains("id: J1"));
        assert!(yaml.contains("package: se.naturkartan"));
        assert!(!yaml.contains("logged_in"));
        assert!(yaml.contains("type: \"Button\""));
        assert!(yaml.contains("# source: FR6-S1 (baseline)"));
        assert!(yaml.contains("assert: element_exists"));
    }

    #[test]
    fn test_tc_to_yaml_logged_in() {
        let tc = GeneratedTC {
            id: "J2".into(),
            name: "Post Question".into(),
            precondition_activity: "MainActivity".into(),
            precondition_package: None,
            precondition_logged_in: true,
            steps: vec![],
        };
        let yaml = tc_to_yaml(&tc);
        assert!(yaml.contains("logged_in: true"));
    }

    #[test]
    fn test_generate_journeys_from_spec() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        assert_eq!(tcs.len(), 2);
        assert_eq!(tcs[0].id, "J1");
        assert_eq!(tcs[1].id, "J2");
    }

    #[test]
    fn test_journey_auth_flag() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        assert!(!tcs[0].precondition_logged_in);
        assert!(tcs[1].precondition_logged_in);
    }

    #[test]
    fn test_journey_search_site_expands() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        let j1 = &tcs[0];
        assert_eq!(j1.steps[0].action, "tap");
        assert_eq!(j1.steps[0].target_fuzzy.as_deref(), Some("Search"));
        assert_eq!(j1.steps[0].element_type.as_deref(), Some("Button"));
        assert_eq!(j1.steps[2].action, "type");
        assert_eq!(j1.steps[2].text.as_deref(), Some("{{fixtures.test_site.name}}"));
    }

    #[test]
    fn test_journey_assert_resolves() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        let asserts: Vec<_> = tcs[0].steps.iter().filter(|s| s.action == "assert").collect();
        assert!(!asserts.is_empty());
        assert_eq!(asserts[0].target_fuzzy.as_deref(), Some("Questions & Answers"));
        assert!(asserts[0].source_comment.as_deref().unwrap().contains("FR6-S1"));
    }

    #[test]
    fn test_journey_scroll_to_assert() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        let j1 = &tcs[0];
        let has_scroll = j1.steps.iter().any(|s| s.action == "scroll_to" && s.target_fuzzy.as_deref() == Some("Questions & Answers"));
        assert!(has_scroll, "scroll_to_assert should emit scroll_to before assert");
    }

    #[test]
    fn test_journey_platform_fork() {
        let spec = make_journey_spec();
        let tcs_android = generate_journeys(&spec, None, None, Some("android"));
        let j2_android = &tcs_android[1];
        let has_long_press = j2_android.steps.iter().any(|s| s.action == "long_press");
        assert!(has_long_press, "Android should have long_press");

        let tcs_ios = generate_journeys(&spec, None, None, Some("ios"));
        let j2_ios = &tcs_ios[1];
        let has_tap_remove = j2_ios.steps.iter().any(|s| s.action == "tap" && s.target_fuzzy.as_deref() == Some("Remove this question"));
        assert!(has_tap_remove, "iOS should have tap 'Remove this question'");
    }

    #[test]
    fn test_journey_no_navigate_to_actions() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        for tc in &tcs {
            for step in &tc.steps {
                assert!(!step.action.starts_with("navigate_to"), "Journey {} has navigate_to action", tc.id);
            }
        }
    }

    #[test]
    fn test_journey_no_journeys_returns_empty() {
        let spec = SpecInput {
            ticket: None, journeys: None,
            functional_requirements: vec![],
        };
        let tcs = generate_journeys(&spec, None, None, None);
        assert!(tcs.is_empty());
    }

    #[test]
    fn test_journey_type_filter_in_yaml() {
        let spec = make_journey_spec();
        let tcs = generate_journeys(&spec, None, None, None);
        let yaml = tc_to_yaml(&tcs[0]);
        assert!(yaml.contains("type: \"Button\""));
    }

    fn make_journey_spec() -> SpecInput {
        SpecInput {
            ticket: None,
            journeys: Some(vec![
                JourneyDef {
                    id: "J1".into(),
                    name: "Browse Q&A".into(),
                    auth: Some(false),
                    steps: vec![
                        JourneyStepDef::SearchSite { search_site: "test_site".into() },
                        JourneyStepDef::ScrollToAssert { scroll_to_assert: AssertRef { fr: "FR6".into(), state: "S1".into(), assert_type: None } },
                        JourneyStepDef::Tap { tap: "see all questions".into(), element_type: None },
                        JourneyStepDef::Assert { assert: AssertRef { fr: "FR7".into(), state: "S1".into(), assert_type: None } },
                    ],
                },
                JourneyDef {
                    id: "J2".into(),
                    name: "Delete Question".into(),
                    auth: Some(true),
                    steps: vec![
                        JourneyStepDef::SearchSite { search_site: "test_site".into() },
                        JourneyStepDef::PlatformFork { platform_fork: {
                            let mut m = HashMap::new();
                            m.insert("android".into(), vec![
                                JourneyStepDef::LongPress { long_press: "question".into() },
                            ]);
                            m.insert("ios".into(), vec![
                                JourneyStepDef::Tap { tap: "Remove this question".into(), element_type: None },
                            ]);
                            m
                        }},
                        JourneyStepDef::Assert { assert: AssertRef { fr: "FR10".into(), state: "S1".into(), assert_type: None } },
                    ],
                },
            ]),
            functional_requirements: vec![
                make_fr("FR6", "Q&A Section", vec![("S1", "has-questions", "Questions & Answers")]),
                make_fr("FR7", "Full Questions List", vec![("S1", "has-questions", "Questions & Answers")]),
                make_fr("FR10", "Delete Question", vec![("S1", "dialog-shown", "Delete question")]),
            ],
        }
    }

    fn make_fr(id: &str, name: &str, states: Vec<(&str, &str, &str)>) -> FunctionalRequirement {
        FunctionalRequirement {
            id: id.into(), name: name.into(), fr_type: Some("static".into()),
            navigation: None,
            state_table: states.into_iter().map(|(sid, state, target)| StateEntry {
                id: sid.into(), state: state.into(),
                condition: None, renders: None,
                assert_target: Some(target.into()),
                assert_type: Some("element_exists".into()),
                fixture: None, requires: None, wait_for: None, platform_fork: None,
            }).collect(),
        }
    }
}
