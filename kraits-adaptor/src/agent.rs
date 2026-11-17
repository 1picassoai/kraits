//! Built-in repo bug-fix agent loop, live task store, and demo helpers for Kraits Repo Adaptor UI.

use crate::ast::AstFile;
use crate::json::Json;
use crate::serve::ServerState;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct ToolTrace {
    pub name: String,
    pub detail: String,
    pub ok: bool,
}

#[derive(Clone)]
pub struct TurnTrace {
    pub turn: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_n: u64,
    pub tok_per_sec: f64,
    pub wall_s: f64,
    pub reasoning_preview: String,
    pub tools: Vec<ToolTrace>,
}

#[derive(Clone)]
pub struct TaskRun {
    pub id: String,
    pub prompt: String,
    pub status: String, // "running" | "completed" | "failed"
    pub elapsed_s: f64,
    pub live_phase: String,
    pub live_tps: f64,
    pub live_tokens: u64,
    pub total_tokens: u64,
    pub last_cache_n: u64,
    pub last_prompt_tokens: u64,
    pub live_stream: String,
    pub turns: Vec<TurnTrace>,
    pub final_result: String,
    pub error: String,
    pub diff: String,
}

pub type TaskMap = Mutex<HashMap<String, TaskRun>>;

impl TaskRun {
    pub fn to_json(&self) -> Json {
        let turns_json: Vec<Json> = self
            .turns
            .iter()
            .map(|t| {
                let tools_json: Vec<Json> = t
                    .tools
                    .iter()
                    .map(|tl| {
                        Json::obj(vec![
                            ("name", Json::Str(tl.name.clone())),
                            ("detail", Json::Str(tl.detail.clone())),
                            ("ok", Json::Bool(tl.ok)),
                        ])
                    })
                    .collect();
                Json::obj(vec![
                    ("turn", Json::Num(t.turn as f64)),
                    ("prompt_tokens", Json::Num(t.prompt_tokens as f64)),
                    ("completion_tokens", Json::Num(t.completion_tokens as f64)),
                    ("cache_n", Json::Num(t.cache_n as f64)),
                    ("tok_per_sec", Json::Num((t.tok_per_sec * 100.0).round() / 100.0)),
                    ("wall_s", Json::Num((t.wall_s * 10.0).round() / 10.0)),
                    ("reasoning_preview", Json::Str(t.reasoning_preview.clone())),
                    ("tools", Json::Arr(tools_json)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("id", Json::Str(self.id.clone())),
            ("prompt", Json::Str(self.prompt.clone())),
            ("status", Json::Str(self.status.clone())),
            ("elapsed_s", Json::Num((self.elapsed_s * 10.0).round() / 10.0)),
            ("live_phase", Json::Str(self.live_phase.clone())),
            ("live_tps", Json::Num((self.live_tps * 100.0).round() / 100.0)),
            ("live_tokens", Json::Num(self.live_tokens as f64)),
            ("total_tokens", Json::Num(self.total_tokens as f64)),
            ("last_cache_n", Json::Num(self.last_cache_n as f64)),
            ("last_prompt_tokens", Json::Num(self.last_prompt_tokens as f64)),
            ("live_stream", Json::Str(self.live_stream.clone())),
            ("turns", Json::Arr(turns_json)),
            ("final_result", Json::Str(self.final_result.clone())),
            ("error", Json::Str(self.error.clone())),
            ("diff", Json::Str(self.diff.clone())),
        ])
    }
}

fn safe_rel_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(format!("Absolute path rejected: {}", rel));
    }
    for comp in p.components() {
        if matches!(comp, Component::ParentDir | Component::Prefix(_) | Component::RootDir) {
            return Err(format!("Path escapes repo: {}", rel));
        }
    }
    Ok(root.join(p))
}

fn normalize_win_cmd(cmd: &str) -> String {
    let mut s = cmd.trim().to_string();
    if let Some(idx) = s.find("&&") {
        let head = s[..idx].trim();
        if head.starts_with("cd /") || head == "cd ~" {
            s = s[idx + 2..].trim().to_string();
        }
    }
    s = s.replace(" && ", "; ");
    if let Some(pidx) = s.rfind("| head ") {
        s = s[..pidx].trim().to_string();
    }
    s
}

fn run_shell_in_repo(root: &Path, cmd: &str) -> (i32, String) {
    let mut command = if cfg!(windows) {
        let win_cmd = normalize_win_cmd(cmd);
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-Command", &win_cmd]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    command.current_dir(root);
    let src_dir = root.join("src");
    if src_dir.is_dir() && std::env::var_os("PYTHONPATH").is_none() {
        command.env("PYTHONPATH", src_dir);
    }
    match command.output() {
        Ok(out) => {
            let code = out.status.code().unwrap_or(1);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let s_tail: String = stdout.chars().rev().take(1800).collect::<String>().chars().rev().collect();
            let e_tail: String = stderr.chars().rev().take(1800).collect::<String>().chars().rev().collect();
            (
                code,
                format!("EXIT_CODE={}\nSTDOUT:\n{}\nSTDERR:\n{}", code, s_tail.trim(), e_tail.trim()),
            )
        }
        Err(e) => (1, format!("EXIT_CODE=1\nSTDERR:\n{}", e)),
    }
}

pub fn git_diff_repo(root: &Path) -> String {
    match Command::new("git").args(["diff", "--", "src", "tests"]).current_dir(root).output() {
        Ok(out) => {
            let txt = String::from_utf8_lossy(&out.stdout).into_owned();
            if txt.trim().is_empty() {
                String::from_utf8_lossy(&out.stderr).into_owned()
            } else {
                txt
            }
        }
        Err(_) => String::new(),
    }
}

fn git_diff_paths(root: &Path, files: &std::collections::BTreeSet<String>) -> String {
    if files.is_empty() {
        return String::new();
    }
    let mut args: Vec<String> = vec!["diff".into(), "--".into()];
    args.extend(files.iter().cloned());
    match Command::new("git").args(&args).current_dir(root).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

fn check_python_syntax(root: &Path, path: &Path) -> Result<(), String> {
    if path.extension().and_then(|e| e.to_str()) != Some("py") {
        return Ok(());
    }
    let script = "import ast, sys; ast.parse(open(sys.argv[1], encoding='utf-8').read(), filename=sys.argv[1])";
    match Command::new("python")
        .arg("-c")
        .arg(script)
        .arg(path)
        .current_dir(root)
        .output()
    {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        Err(e) => Err(format!("syntax check failed: {}", e)),
    }
}

fn build_unified_hunk(rel: &str, body_before: &str, old_n: &str, new_n: &str) -> String {
    let prefix = body_before.find(old_n).map(|i| &body_before[..i]).unwrap_or("");
    let start_line = prefix.lines().count().max(1);
    let old_lines: Vec<String> = old_n.lines().map(|l| format!("-{}", l)).collect();
    let new_lines: Vec<String> = new_n.lines().map(|l| format!("+{}", l)).collect();
    format!(
        "diff --git a/{rel} b/{rel}\n--- a/{rel}\n+++ b/{rel}\n@@ -{sl},{oc} +{sl},{nc} @@\n{old}\n{new}",
        rel = rel,
        sl = start_line,
        oc = old_lines.len().max(1),
        nc = new_lines.len().max(1),
        old = old_lines.join("\n"),
        new = new_lines.join("\n")
    )
}

fn levenshtein_ascii(a: &[u8], b: &[u8]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(!ca.eq_ignore_ascii_case(&cb));
            cur[j + 1] = (cur[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn suggest_repo_paths(store: &crate::repo::RepoStore, bad_rel: &str) -> Vec<String> {
    let cleaned = clean_tool_path(bad_rel).replace("..", "/").replace('.', "/");
    let fname = Path::new(&cleaned).file_name().and_then(|s| s.to_str()).unwrap_or(&cleaned).to_ascii_lowercase();
    let bad_low = bad_rel.replace('\\', "/").to_ascii_lowercase();
    let mut scored: Vec<(usize, String)> = store
        .files
        .keys()
        .map(|p| {
            let p_low = p.to_ascii_lowercase();
            let p_fname = Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p).to_ascii_lowercase();
            let dist_full = levenshtein_ascii(bad_low.as_bytes(), p_low.as_bytes());
            let dist_name = levenshtein_ascii(fname.as_bytes(), p_fname.as_bytes());
            let mut penalty = dist_full + dist_name * 2;
            let p_noext = p_low.trim_end_matches(".py");
            if cleaned.to_ascii_lowercase().contains(p_noext) {
                penalty = penalty.saturating_sub(8);
            }
            if p_low.starts_with("src/") {
                penalty = penalty.saturating_sub(1);
            }
            (penalty, p.clone())
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(3).map(|(_, p)| p).collect()
}

fn run_search_repo_query(store: &crate::repo::RepoStore, root: &Path, query: &str, path_filter: &str) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("ERROR: `query` is required".into());
    }
    let q_low = q.to_ascii_lowercase();
    let mut hits = Vec::new();
    let mut total_chars = 0usize;
    'outer: for rel in store.files.keys() {
        if !path_filter.is_empty() && !rel.contains(path_filter) {
            continue;
        }
        let Ok(raw) = fs::read_to_string(root.join(rel)) else { continue };
        for (idx, line) in raw.lines().enumerate() {
            if line.to_ascii_lowercase().contains(&q_low) {
                let snip: String = line.trim().chars().take(180).collect();
                let entry = format!("{}:{}: {}", rel, idx + 1, snip);
                total_chars += entry.len() + 1;
                hits.push(entry);
                if hits.len() >= 25 || total_chars >= 3200 {
                    break 'outer;
                }
            }
        }
    }
    if hits.is_empty() {
        Err(format!("No matches found for `{}`", q))
    } else {
        Ok(hits.join("\n"))
    }
}

fn infer_search_token(name: &str, args: &Json) -> String {
    if name == "ast_symbol" {
        if let Some(sym) = args.get("symbol").and_then(|s| s.as_str()).map(str::trim).filter(|s| s.len() >= 3) {
            return sym.trim_start_matches('_').split('_').take(2).collect::<Vec<_>>().join("_");
        }
    }
    if let Some(p) = args.get("path").and_then(|s| s.as_str()) {
        let stem = Path::new(p).file_stem().and_then(|s| s.to_str()).unwrap_or("").trim();
        if stem.len() >= 3 {
            return stem.to_string();
        }
    }
    "stream_with_context".into()
}

fn has_repeated_4gram(t: &str) -> bool {
    let words: Vec<String> = t
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
                .to_ascii_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() < 8 {
        return false;
    }
    let mut seen: HashSet<[&str; 4]> = HashSet::new();
    for win in words.windows(4) {
        let key = [win[0].as_str(), win[1].as_str(), win[2].as_str(), win[3].as_str()];
        if !seen.insert(key) {
            return true;
        }
    }
    false
}

fn summary_has_unknown_path(t: &str, indexed_files: &BTreeMap<String, AstFile>) -> bool {
    if indexed_files.is_empty() {
        return false;
    }
    let top_dirs: HashSet<&str> = indexed_files
        .keys()
        .filter_map(|p| p.split('/').next().filter(|_| p.contains('/')))
        .collect();
    const EXTS: &[&str] = &[
        ".py", ".rs", ".ts", ".tsx", ".js", ".jsx", ".json", ".toml", ".yaml", ".yml",
        ".md", ".txt", ".rst", ".html", ".css", ".sh", ".cfg", ".ini",
    ];
    for raw_tok in t.split_whitespace() {
        if raw_tok.contains("http://") || raw_tok.contains("https://") {
            continue;
        }
        let trimmed = raw_tok
            .trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '(' | ')' | '[' | ']' | ',' | ';' | ':'))
            .trim_end_matches('.')
            .replace('\\', "/");
        if trimmed.is_empty() || !trimmed.contains('/') {
            continue;
        }
        let mut cand = trimmed.strip_prefix("./").unwrap_or(&trimmed).to_string();
        if let Some((base, _)) = cand.split_once("::") {
            cand = base.to_string();
        }
        if let Some((base, line_part)) = cand.rsplit_once(':') {
            if !line_part.is_empty() && line_part.chars().all(|c| c.is_ascii_digit()) {
                cand = base.to_string();
            }
        }
        let cand_low = cand.to_ascii_lowercase();
        let has_ext = EXTS.iter().any(|ext| cand_low.ends_with(ext));
        let has_top_dir = cand
            .split('/')
            .next()
            .is_some_and(|head| top_dirs.contains(head));
        if (has_ext || has_top_dir) && !indexed_files.contains_key(&cand) {
            return true;
        }
    }
    false
}

fn deterministic_fix_summary(edited_files: &BTreeSet<String>, last_ok_summary: &str) -> String {
    let files_list: Vec<String> = edited_files.iter().cloned().collect();
    format!(
        "Updated {} and verified with {}.",
        files_list.join(", "),
        if last_ok_summary.is_empty() { "EXIT_CODE=0" } else { last_ok_summary }
    )
}

fn finalize_max_turns_outcome(
    edited_files: &BTreeSet<String>,
    verified_pass: bool,
    last_ok_summary: &str,
    existing_final: &str,
) -> (&'static str, &'static str, String, String) {
    if !edited_files.is_empty() && verified_pass {
        return (
            "completed",
            "Verified & Fixed",
            String::new(),
            deterministic_fix_summary(edited_files, last_ok_summary),
        );
    }
    let fin = if existing_final.trim().is_empty() {
        deterministic_failure_summary(edited_files, "exceeded max_turns (9)")
    } else {
        existing_final.to_string()
    };
    ("failed", "Aborted", "Exceeded max_turns (9)".into(), fin)
}

fn deterministic_failure_summary(edited_files: &BTreeSet<String>, reason: &str) -> String {
    if edited_files.is_empty() {
        format!("Aborted ({}) with no files modified.", reason)
    } else {
        let files_list: Vec<String> = edited_files.iter().cloned().collect();
        format!("Aborted ({}) after editing {}.", reason, files_list.join(", "))
    }
}

fn should_abort_loop_guard(
    readonly_q: bool,
    edited_empty: bool,
    verified_pass: bool,
    turns_without_edit: usize,
    total_edit_attempts: usize,
    total_tool_errors: usize,
) -> Option<&'static str> {
    if !readonly_q && edited_empty && turns_without_edit >= 5 {
        return Some("exploration loop: 5 turns with 0 edits");
    }
    if total_edit_attempts >= 4 && !verified_pass {
        return Some("non-converging edit/verify loop (4 edits without pass)");
    }
    if total_tool_errors >= 5 && !verified_pass {
        return Some("repeated tool failures (5 errors without pass)");
    }
    None
}

fn extract_verify_command(
    prompt: &str,
    last_test_cmd: &str,
    edited_rel: &str,
    files: &BTreeMap<String, AstFile>,
) -> String {
    if !last_test_cmd.trim().is_empty() {
        return last_test_cmd.trim().to_string();
    }
    if let Some(pos) = prompt.find("python -m pytest ") {
        let tail = &prompt[pos..];
        let end = tail
            .find(|c: char| matches!(c, '`' | '\n' | '\r' | ','))
            .unwrap_or(tail.len());
        let candidate = tail[..end].trim().trim_end_matches('.');
        if candidate.len() > "python -m pytest ".len() {
            return candidate.to_string();
        }
    }
    let stem = Path::new(edited_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if !stem.is_empty() {
        let guess = format!("tests/test_{}.py", stem);
        if files.contains_key(&guess) {
            return format!("python -m pytest {} -q", guess);
        }
    }
    "python -m pytest -q".into()
}

fn apply_guarded_write_to_file(
    root: &Path,
    store: &mut crate::repo::RepoStore,
    rel: &str,
    content: &str,
) -> Result<bool, (String, String)> {
    let p = safe_rel_path(root, rel).map_err(|e| (rel.to_string(), format!("ERROR: {}", e)))?;
    let norm_rel = rel.replace('\\', "/").trim_start_matches("./").to_string();
    let prev = fs::read_to_string(&p).ok();

    if let Some(ref old_raw) = prev {
        let old_bytes = old_raw.len();
        let new_bytes = content.len();
        let size_under_half = old_bytes > 0 && new_bytes.saturating_mul(2) < old_bytes;
        let old_syms = store
            .files
            .get(&norm_rel)
            .map(|f| f.nodes.len())
            .unwrap_or_else(|| {
                crate::ast::parse_ast(&norm_rel, old_raw)
                    .map(|f| f.nodes.len())
                    .unwrap_or(0)
            });
        let new_syms = crate::ast::parse_ast(&norm_rel, content)
            .map(|f| f.nodes.len())
            .unwrap_or(old_syms);
        let syms_dropped_zero = old_syms > 0 && new_syms == 0;
        if size_under_half || syms_dropped_zero {
            return Err((
                format!("{} (write refused: size/symbol guard)", rel),
                format!(
                    "ERROR: Refused write_to_file on existing indexed file `{}` (new size {}B < 50% of {}B or symbols {} -> {}). Original file preserved; use `replace_in_file` for targeted edits.",
                    norm_rel, new_bytes, old_bytes, old_syms, new_syms
                ),
            ));
        }
    }

    if let Some(par) = p.parent() {
        let _ = fs::create_dir_all(par);
    }
    fs::write(&p, content).map_err(|e| (rel.to_string(), format!("ERROR: {}", e)))?;
    if let Err(syn_err) = check_python_syntax(root, &p) {
        if let Some(old_raw) = prev {
            let _ = fs::write(&p, old_raw);
        } else {
            let _ = fs::remove_file(&p);
        }
        return Err((
            format!("{} (syntax reverted)", rel),
            format!("ERROR: Python syntax error (write reverted cleanly): {}", syn_err),
        ));
    }
    let _ = store.sync();
    Ok(prev.is_none())
}

fn run_verify_in_repo(root: &Path, store: &crate::repo::RepoStore, cmd: &str) -> (bool, String, String, String) {
    let (code, mut out) = run_shell_in_repo(root, cmd);
    let low_cmd = cmd.to_ascii_lowercase();
    let low_out = out.to_ascii_lowercase();
    let is_pytest_cmd = low_cmd.contains("pytest");
    let real_pytest_pass = !is_pytest_cmd || low_out.contains(" passed") || low_out.contains("passed in");
    let passed = code == 0 && real_pytest_pass;
    let mut pass_summary = String::new();
    if passed {
        let pass_line = out.lines().find(|l| l.contains("passed")).unwrap_or("EXIT_CODE=0").trim().to_string();
        pass_summary = format!("`{}` -> {}", cmd, pass_line);
    } else if code == 0 && !real_pytest_pass {
        out.push_str("\n\nERROR: Command exited 0 without pytest pass output (`passed`). Run `python -m pytest <test_file> -q` properly.");
    } else {
        let focused = store.build_repo_context(&out, 900);
        if let Some(idx) = focused.find("# Focused AST Node At Diagnostic") {
            out.push_str("\n\n");
            out.push_str(&focused[idx..]);
        }
    }
    let tail_preview = out.lines().take(6).collect::<Vec<_>>().join(" | ");
    let detail = format!("`{}` -> exit={} ({})", cmd, code, tail_preview);
    (passed, detail, out, pass_summary)
}

fn is_clean_english_summary_with_index(s: &str, indexed_files: Option<&BTreeMap<String, AstFile>>) -> bool {
    let t = s.trim();
    if t.len() < 20 || t.len() > 2200 {
        return false;
    }
    if !t.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if t.contains("...") || t.contains('\u{2026}') {
        return false;
    }
    if t.contains('<') || t.contains('>') {
        return false;
    }
    if !t.chars().last().is_some_and(|c| matches!(c, '.' | '!' | '?')) {
        return false;
    }
    let open_p = t.chars().filter(|&c| c == '(').count();
    let close_p = t.chars().filter(|&c| c == ')').count();
    if open_p != close_p {
        return false;
    }
    if let Some(last_tok) = t.split_whitespace().last() {
        let low = last_tok.to_ascii_lowercase();
        if low.contains('/') && low.ends_with('.') && !low.contains(".py.") && !low.contains(".md.") && !low.contains(".txt.") {
            return false;
        }
    }
    let bad_markers = [
        "</think>", "<think>", "<tool_call>", "<function=", "</result>", "</invoke>", "/**", "// Explanation",
        "{\"", "down't", "'s not available", "so that it be.", "FLAKASH", "FLFKSH",
    ];
    if bad_markers.iter().any(|m| t.contains(m)) {
        return false;
    }
    if has_repeated_4gram(t) {
        return false;
    }
    if indexed_files.is_some_and(|files| summary_has_unknown_path(t, files)) {
        return false;
    }
    let mut prev_alpha = String::new();
    for tok in t.split_whitespace() {
        let low_tok = tok.to_ascii_lowercase();
        if low_tok.contains('-') && low_tok.trim_end_matches('.').rsplit('-').next().is_some_and(|s| s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic())) {
            return false;
        }
        if tok.contains('`') || tok.contains('/') || tok.contains('\\') || tok.contains('_') || tok.contains('.') || tok.contains(':') {
            prev_alpha.clear();
            continue;
        }
        let alpha: String = tok.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if alpha.len() >= 2 && !prev_alpha.is_empty() && prev_alpha.eq_ignore_ascii_case(&alpha) {
            return false;
        }
        prev_alpha = alpha.clone();
        if alpha.len() >= 6 && alpha.chars().all(|c| c.is_ascii_uppercase()) {
            const ALLOWED_CAPS: &[&str] = &["OPTIONS", "STDOUT", "STDERR", "PYTHON", "PYTEST", "RETURN", "REQUEST", "CONTEXT", "CONTEXTVAR"];
            if !ALLOWED_CAPS.contains(&alpha.as_str()) {
                return false;
            }
        }
        let mut consec_cons = 0usize;
        for c in alpha.chars().map(|c| c.to_ascii_lowercase()) {
            if matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y') {
                consec_cons = 0;
            } else {
                consec_cons += 1;
                if consec_cons >= 5 {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
fn is_clean_english_summary(s: &str) -> bool {
    is_clean_english_summary_with_index(s, None)
}

fn prevalidate_tool_call(
    store: &crate::repo::RepoStore,
    root: &Path,
    name: &str,
    args: &Json,
) -> Result<(), (String, String)> {
    match name {
        "ast_symbol" | "read_file" | "replace_in_file" => {
            let raw_p = args.get("path").and_then(|s| s.as_str()).unwrap_or("").trim();
            let norm = raw_p.replace('\\', "/").trim_start_matches("./").to_string();
            let bad_chars = raw_p.is_empty()
                || raw_p.contains(',')
                || raw_p.contains('\'')
                || raw_p.contains('"')
                || raw_p.contains("..")
                || raw_p.contains(':')
                || norm.starts_with('/');
            if bad_chars || !store.files.contains_key(&norm) {
                let near = suggest_repo_paths(store, raw_p);
                let hint = format!(
                    "VALIDATION ERROR: Path `{}` is not in the {}-file repo index. Nearest real paths: {}",
                    raw_p,
                    store.files.len(),
                    near.join(", ")
                );
                return Err((format!("invalid path: {}", raw_p), hint));
            }
            if name == "ast_symbol" {
                let sym = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("").trim();
                if sym.is_empty() || store.extract_ast_symbol(&norm, sym).is_none() {
                    let skel = store.files.get(&norm).map(|f| f.render_skeleton()).unwrap_or_default();
                    let hint = format!(
                        "VALIDATION ERROR: Symbol `{}` not found in AST index for `{}`.\n{}",
                        sym, norm, skel
                    );
                    return Err((format!("invalid symbol: {} :: {}", norm, sym), hint));
                }
            }
            if name == "read_file" {
                let s_raw = args.get("start_line").and_then(|n| n.as_i64()).unwrap_or(1);
                let e_opt = args.get("end_line").and_then(|n| n.as_i64());
                if s_raw < 1 || e_opt.is_some_and(|e| e < s_raw) {
                    return Err((
                        format!("invalid line range: L{}-L{:?}", s_raw, e_opt),
                        format!("VALIDATION ERROR: Invalid line range start_line={} end_line={:?} for `{}`. Must satisfy 1 <= start_line <= end_line.", s_raw, e_opt, norm),
                    ));
                }
            }
            Ok(())
        }
        "write_to_file" => {
            let raw_p = args.get("path").and_then(|s| s.as_str()).unwrap_or("").trim();
            let norm = raw_p.replace('\\', "/").trim_start_matches("./").to_string();
            let bad_chars = raw_p.is_empty()
                || raw_p.contains(',')
                || raw_p.contains('\'')
                || raw_p.contains('"')
                || raw_p.contains("..")
                || raw_p.contains(':')
                || norm.starts_with('/');
            let par_ok = Path::new(&norm)
                .parent()
                .map(|p| p.as_os_str().is_empty() || root.join(p).is_dir())
                .unwrap_or(false);
            if bad_chars || !par_ok {
                let near = suggest_repo_paths(store, raw_p);
                return Err((
                    format!("invalid write path: {}", raw_p),
                    format!("VALIDATION ERROR: `{}` is not a valid target path in existing repo dirs. Nearest paths: {}", raw_p, near.join(", ")),
                ));
            }
            Ok(())
        }
        "execute_command" => {
            let cmd = args.get("command").and_then(|s| s.as_str()).unwrap_or("").trim();
            let head = cmd.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
            let allowed = matches!(head.as_str(), "python" | "python.exe" | "pytest" | "git");
            let bad_flag = cmd.contains("python - ") || cmd.contains("python.exe - ") || cmd.contains("Select_");
            if !allowed || bad_flag {
                return Err((
                    format!("invalid command: {}", cmd.chars().take(60).collect::<String>()),
                    format!("VALIDATION ERROR: Command `{}` is not allowed on Windows PowerShell. Only run `python -m pytest ...` or `git ...`; use `search_repo` to search files.", cmd),
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_readonly_question(prompt: &str) -> bool {
    let low = prompt.to_ascii_lowercase();
    let edit_verbs = [
        "fix", "patch", "implement", "add ", "modify", "edit", "refactor", "pytest",
        "write ", "create ", "regression test", "replace_in_file", "write_to_file",
        "execute_command", "failing", "broken", "bug", "issue #",
    ];
    !edit_verbs.iter().any(|v| low.contains(v))
}

fn clean_tool_path(raw: &str) -> String {
    let head = raw.split(',').next().unwrap_or(raw).trim();
    let no_line = head.split(": ").next().unwrap_or(head).trim();
    no_line.replace('\\', "/").trim_start_matches("./").to_string()
}

fn extract_readable_completion(args: &Json) -> String {
    for key in ["result", "summary", "answer", "explanation", "message", "response", "text", "output"] {
        if let Some(s) = args.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with('{') {
                if let Ok(inner) = Json::parse(t) {
                    for ik in ["summary", "answer", "explanation", "result", "analysis"] {
                        if let Some(is) = inner.get(ik).and_then(|v| v.as_str()).map(str::trim).filter(|x| !x.is_empty()) {
                            return is.to_string();
                        }
                    }
                }
            }
            return t.to_string();
        }
    }
    if let Some(obj) = args.as_obj() {
        for v in obj.values() {
            if let Some(s) = v.as_str().map(str::trim).filter(|x| !x.is_empty()) {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn overlaps_seen_range(seen: &[(usize, usize)], s: usize, e: usize) -> Option<(usize, usize)> {
    let req_len = e.saturating_sub(s) + 1;
    for &(ps, pe) in seen {
        let ov_s = s.max(ps);
        let ov_e = e.min(pe);
        if ov_e >= ov_s {
            let ov_len = ov_e - ov_s + 1;
            if ov_len * 100 >= req_len * 80 {
                return Some((ps, pe));
            }
        }
    }
    None
}

fn append_debug_log(root: &Path, line: &str) {
    let p = root.join(".kraits/completion_debug.log");
    if let Some(par) = p.parent() {
        let _ = fs::create_dir_all(par);
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{}", line);
    }
}

fn clean_prose_text(raw: &str) -> String {
    let s = raw
        .split("<tool_call>")
        .next()
        .unwrap_or(raw)
        .replace("</think>", "")
        .replace("<think>", "");
    s.trim().to_string()
}

fn extract_partial_completion_text(arg_str: &str) -> Option<String> {
    for key in ["\"result\"", "\"summary\"", "\"answer\"", "\"explanation\""] {
        let Some(k_pos) = arg_str.find(key) else { continue };
        let after_key = &arg_str[k_pos + key.len()..];
        let Some(colon) = after_key.find(':') else { continue };
        let after_colon = after_key[colon + 1..].trim_start();
        let Some(rest) = after_colon.strip_prefix('"') else { continue };
        let mut out = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            if c == '"' {
                break;
            }
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some(other) => out.push(other),
                    None => break,
                }
            } else {
                out.push(c);
            }
        }
        let mut cleaned = clean_prose_text(&out);
        cleaned = cleaned
            .trim_start_matches(|c: char| matches!(c, '"' | ',' | ':' | '\'' | '\\') || c.is_whitespace())
            .to_string();
        if let Some(rest) = cleaned.strip_prefix("Explanation:") {
            cleaned = rest.trim().to_string();
        }
        if cleaned.len() < 120 {
            continue;
        }
        if cleaned.ends_with('.') || cleaned.ends_with('!') || cleaned.ends_with('?') || cleaned.ends_with('`') {
            return Some(cleaned);
        }
        if let Some(last_dot) = cleaned.rfind(". ") {
            if last_dot >= 80 {
                return Some(cleaned[..=last_dot].trim().to_string());
            }
        }
        if let Some(last_nl) = cleaned.rfind('\n') {
            if last_nl >= 80 {
                return Some(cleaned[..last_nl].trim().to_string());
            }
        }
        return Some(cleaned);
    }
    None
}

fn synthesize_readonly_prose(
    state: &Arc<ServerState>,
    task_id: &str,
    turn: usize,
    t0: Instant,
    messages: &[Json],
    tools_schema: &Json,
    prompt: &str,
) -> Option<(String, u64, u64, u64, f64)> {
    let mut final_msgs = messages.to_vec();
    final_msgs.push(Json::obj(vec![
        ("role", Json::Str("user".into())),
        ("content", Json::Str(format!("Stop calling tools. Write a clear 3-5 sentence plain English explanation answering: {}", prompt))),
    ]));
    let payload = Json::obj(vec![
        ("model", Json::Str("Qwen3.8-Flash-Next-IQ4_XS".into())),
        ("messages", Json::Arr(final_msgs)),
        ("temperature", Json::Num(0.2)),
        ("repeat_penalty", Json::Num(1.0)),
        ("max_tokens", Json::Num(320.0)),
        ("reasoning_budget", Json::Num(0.0)),
        ("chat_template_kwargs", Json::obj(vec![("enable_thinking", Json::Bool(false))])),
        ("stream", Json::Bool(true)),
        ("stream_options", Json::obj(vec![("include_usage", Json::Bool(true))])),
    ]);
    let _ = tools_schema;
    let sr = post_upstream_chat_streaming(state, task_id, turn, t0, &payload, false).ok()?;
    let mut txt = clean_prose_text(&sr.content);
    if txt.is_empty() || txt.ends_with(':') || txt.len() < 100 {
        if let Some(ext) = extract_partial_completion_text(&sr.content) {
            txt = ext;
        }
    }
    if txt.is_empty() || txt.ends_with(':') || txt.len() < 100 {
        for tc in &sr.tool_calls {
            if let Some(arg_s) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|s| s.as_str()) {
                if let Ok(a) = Json::parse(arg_s) {
                    let ext = clean_prose_text(&extract_readable_completion(&a));
                    if !ext.is_empty() {
                        txt = ext;
                        break;
                    }
                } else if let Some(ext) = extract_partial_completion_text(arg_s) {
                    txt = ext;
                    break;
                }
            }
        }
    }
    if txt.is_empty() || txt.ends_with(':') {
        None
    } else {
        Some((txt, sr.prompt_tokens, sr.completion_tokens, sr.cache_n, sr.tok_per_sec))
    }
}

fn compact_messages_for_ctx(messages: &mut [Json], max_chars: usize) {
    let mut cur_total: usize = messages
        .iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()).map(str::len))
        .sum();
    if cur_total <= max_chars || messages.len() <= 6 {
        return;
    }
    let stub = "[earlier tool output trimmed to fit context window; re-read targeted lines if needed]";
    let keep_after = messages.len().saturating_sub(4);
    for msg in messages.iter_mut().take(keep_after).skip(2) {
        let is_tool = msg.get("role").and_then(|r| r.as_str()) == Some("tool");
        if !is_tool {
            continue;
        }
        let len = msg.get("content").and_then(|c| c.as_str()).map(str::len).unwrap_or(0);
        if len > 240 {
            if let Some(obj) = msg.as_obj_mut() {
                obj.insert("content".into(), Json::Str(stub.into()));
                cur_total = cur_total.saturating_sub(len.saturating_sub(stub.len()));
            }
            if cur_total <= max_chars {
                break;
            }
        }
    }
}

pub fn handle_demo_action(state: &Arc<ServerState>, action: &str, target: &str) -> Result<Json, String> {
    let root = &state.repo_root;
    let msg;
    let mut suggested_prompt = String::new();
    let test_cmd;

    match action {
        "bug1" => {
            let helpers = root.join("src/flask/helpers.py");
            let raw = fs::read_to_string(&helpers).map_err(|e| format!("Read helpers.py: {}", e))?;
            let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
            let body = raw.replace("\r\n", "\n");
            let orig = "    val = os.environ.get(\"FLASK_DEBUG\")\n    return bool(val and val.lower() not in {\"0\", \"false\", \"no\"})";
            let broken = "    val = os.environ.get(\"FLASK_DEBUG\")\n    return bool(val)";
            if body.contains(orig) {
                let updated = body.replace(orig, broken).replace('\n', eol);
                fs::write(&helpers, updated).map_err(|e| e.to_string())?;
                msg = "Injected demo bug in src/flask/helpers.py::get_debug_flag() (`bool(val)` ignores \"0\"/\"false\"/\"no\").".into();
            } else if body.contains(broken) {
                msg = "Demo bug #1 is already active in src/flask/helpers.py::get_debug_flag().".into();
            } else {
                return Err("Could not match get_debug_flag() in src/flask/helpers.py".into());
            }
            test_cmd = "python -m pytest tests/test_helpers.py -q".into();
            suggested_prompt = "tests/test_helpers.py is failing on TestHelpers::test_get_debug_flag because `FLASK_DEBUG=\"0\"`, `\"False\"`, and `\"No\"` return True instead of False. Inspect `src/flask/helpers.py` (`get_debug_flag`), fix the bug with `replace_in_file`, and verify with `python -m pytest tests/test_helpers.py -q`.".into();
        }
        "bug2" => {
            let cfg = root.join("src/flask/config.py");
            let raw = fs::read_to_string(&cfg).map_err(|e| format!("Read config.py: {}", e))?;
            let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
            let body = raw.replace("\r\n", "\n");
            let orig = "            if trim_namespace:\n                key = k[len(namespace) :]\n            else:\n                key = k";
            let broken = "            if not trim_namespace:\n                key = k[len(namespace) :]\n            else:\n                key = k";
            if body.contains(orig) {
                let updated = body.replace(orig, broken).replace('\n', eol);
                fs::write(&cfg, updated).map_err(|e| e.to_string())?;
                msg = "Injected demo bug in src/flask/config.py::Config.get_namespace() (inverted `trim_namespace` condition).".into();
            } else if body.contains(broken) {
                msg = "Demo bug #2 is already active in src/flask/config.py::Config.get_namespace().".into();
            } else {
                return Err("Could not match Config.get_namespace() in src/flask/config.py".into());
            }
            test_cmd = "python -m pytest tests/test_config.py -q".into();
            suggested_prompt = "tests/test_config.py is failing around `Config.get_namespace` (namespace keys are not trimmed when `trim_namespace=True`). Inspect `src/flask/config.py` (`get_namespace`) via `ast_symbol`, fix the condition cleanly with `replace_in_file`, and verify with `python -m pytest tests/test_config.py -q`.".into();
        }
        "feat1" => {
            let route_test = root.join("tests/test_route_summary.py");
            if route_test.exists() {
                msg = "tests/test_route_summary.py already exists; resetting feat1 clean.".into();
                let _ = fs::remove_file(&route_test);
                let _ = Command::new("git").args(["checkout", "--", "src/flask/app.py"]).current_dir(root).output();
            } else {
                msg = "Feat1 precheck ready: Flask.route_summary() not yet implemented in src/flask/app.py.".into();
            }
            test_cmd = "python -m pytest tests/test_route_summary.py tests/test_basic.py -q".into();
            suggested_prompt = "In src/flask/app.py add Flask.route_summary(self) iterating self.url_map.iter_rules() returning sorted dicts {endpoint, rule, methods} excluding HEAD and OPTIONS (guarding rule.methods against None, using a valid lambda key), add tests in tests/test_route_summary.py, and verify with python -m pytest tests/test_route_summary.py tests/test_basic.py -q.".into();
        }
        "reset" => {
            for tr in state.tasks.lock().unwrap().values_mut() {
                if tr.status == "running" {
                    tr.status = "cancelled".into();
                    tr.error = "Cancelled by repository reset".into();
                }
            }
            let _ = Command::new("git").args(["checkout", "--", "src", "tests"]).current_dir(root).output();
            let rt = root.join("tests/test_route_summary.py");
            if rt.exists() {
                let _ = fs::remove_file(rt);
            }
            msg = "Restored clean upstream `src/` and `tests/` files via git checkout.".into();
            test_cmd = "python -m pytest tests/test_helpers.py tests/test_config.py -q".into();
        }
        "pytest" => {
            let t = target.trim();
            let valid = !t.is_empty()
                && !t.contains('&')
                && !t.contains(';')
                && !t.contains('|')
                && !t.contains('`')
                && t.starts_with("tests/");
            let suite = if valid { t } else { "tests/test_helpers.py tests/test_config.py" };
            msg = format!("Executed `{}`.", suite);
            test_cmd = format!("python -m pytest {} -q", suite);
        }
        other => return Err(format!("Unknown demo action: {}", other)),
    }

    {
        let mut st = state.store.lock().unwrap();
        let _ = st.sync();
    }
    let (exit_code, output) = if test_cmd.is_empty() {
        (0, String::new())
    } else {
        run_shell_in_repo(root, &test_cmd)
    };
    Ok(Json::obj(vec![
        ("ok", Json::Bool(exit_code == 0)),
        ("exit_code", Json::Num(exit_code as f64)),
        ("message", Json::Str(msg)),
        ("command", Json::Str(test_cmd)),
        ("output", Json::Str(output)),
        ("suggested_prompt", Json::Str(suggested_prompt)),
        ("diff", Json::Str(git_diff_repo(root))),
    ]))
}

fn build_tools_schema() -> Json {
    let tool = |name: &str, desc: &str, props: Vec<(&str, &str)>, req: Vec<&str>| {
        let prop_map: Vec<(&str, Json)> = props
            .into_iter()
            .map(|(k, ty)| (k, Json::obj(vec![("type", Json::Str(ty.into()))])))
            .collect();
        Json::obj(vec![
            ("type", Json::Str("function".into())),
            (
                "function",
                Json::obj(vec![
                    ("name", Json::Str(name.into())),
                    ("description", Json::Str(desc.into())),
                    (
                        "parameters",
                        Json::obj(vec![
                            ("type", Json::Str("object".into())),
                            ("properties", Json::obj(prop_map)),
                            ("required", Json::Arr(req.into_iter().map(|r| Json::Str(r.into())).collect())),
                        ]),
                    ),
                ]),
            ),
        ])
    };

    Json::Arr(vec![
        tool(
            "ast_symbol",
            "Read a single AST symbol (class/def/method) cleanly with line numbers without loading the full file.",
            vec![("path", "string"), ("symbol", "string")],
            vec!["path", "symbol"],
        ),
        tool(
            "search_repo",
            "Search indexed source/test files for text or symbols (returns path:line matches). Use instead of grep.",
            vec![("query", "string"), ("path", "string")],
            vec!["query"],
        ),
        tool(
            "read_file",
            "Read a file line range (max 140 lines per call) relative to the workspace. Prefer ast_symbol or search_repo first.",
            vec![("path", "string"), ("start_line", "integer"), ("end_line", "integer")],
            vec!["path"],
        ),
        tool(
            "replace_in_file",
            "Replace an exact existing snippet once in a file.",
            vec![("path", "string"), ("old_text", "string"), ("new_text", "string")],
            vec!["path", "old_text", "new_text"],
        ),
        tool(
            "write_to_file",
            "Create or overwrite a file relative to the workspace.",
            vec![("path", "string"), ("content", "string")],
            vec!["path", "content"],
        ),
        tool(
            "execute_command",
            "Execute a PowerShell verification command (such as python -m pytest ...) inside the repository.",
            vec![("command", "string")],
            vec!["command"],
        ),
        tool(
            "attempt_completion",
            "Finish the task with a non-empty `result` answering the user (if files were edited, execute_command must return EXIT_CODE=0 after the last edit).",
            vec![("result", "string")],
            vec!["result"],
        ),
    ])
}

struct StreamResult {
    content: String,
    reasoning_preview: String,
    tool_calls: Vec<Json>,
    prompt_tokens: u64,
    completion_tokens: u64,
    cache_n: u64,
    tok_per_sec: f64,
    early_stopped: bool,
}

fn tool_stream_should_stop(early_stop_no_tool: bool, has_tc: bool, content: &str, chunk_toks: u64) -> bool {
    if !early_stop_no_tool || has_tc {
        return false;
    }
    if chunk_toks >= 256 {
        return true;
    }
    chunk_toks >= 64
        && !content.contains('{')
        && !content.contains("<invoke")
        && !content.contains("<tool_call")
}

fn read_next_sse_line<R: BufRead>(reader: &mut R, is_chunked: bool, pending: &mut Vec<u8>) -> Result<Option<String>, String> {
    if !is_chunked {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        return Ok(if n == 0 { None } else { Some(line) });
    }
    loop {
        if let Some(pos) = pending.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = pending.drain(..=pos).collect();
            return Ok(Some(String::from_utf8_lossy(&line_bytes).into_owned()));
        }
        let mut sz_line = String::new();
        if reader.read_line(&mut sz_line).map_err(|e| e.to_string())? == 0 {
            if pending.is_empty() {
                return Ok(None);
            }
            let rest = String::from_utf8_lossy(pending).into_owned();
            pending.clear();
            return Ok(Some(rest));
        }
        let hex = sz_line.trim().split(';').next().unwrap_or("");
        if hex.is_empty() {
            continue;
        }
        let sz = usize::from_str_radix(hex, 16).unwrap_or(0);
        if sz == 0 {
            if pending.is_empty() {
                return Ok(None);
            }
            let rest = String::from_utf8_lossy(pending).into_owned();
            pending.clear();
            return Ok(Some(rest));
        }
        let mut chunk = vec![0u8; sz];
        reader.read_exact(&mut chunk).map_err(|e| e.to_string())?;
        pending.extend_from_slice(&chunk);
        let mut crlf = [0u8; 2];
        let _ = reader.read_exact(&mut crlf);
    }
}

fn post_upstream_chat_streaming(
    state: &Arc<ServerState>,
    task_id: &str,
    turn: usize,
    t0: Instant,
    payload: &Json,
    early_stop_no_tool: bool,
) -> Result<StreamResult, String> {
    let body = payload.encode();
    let mut stream = TcpStream::connect(&state.upstream_host_port)
        .map_err(|e| format!("Upstream {}: {}", state.upstream_host_port, e))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(600)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
    let req = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAuthorization: Bearer kraits\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        state.upstream_host_port,
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).map_err(|e| e.to_string())?;
    let mut is_chunked = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let t = line.trim_end_matches(&['\r', '\n'][..]);
        if t.is_empty() {
            break;
        }
        if let Some((k, v)) = t.split_once(':') {
            if k.trim().eq_ignore_ascii_case("transfer-encoding")
                && v.trim().to_ascii_lowercase().contains("chunked")
            {
                is_chunked = true;
            }
        }
    }

    if !status_line.contains(" 200 ") {
        let mut err_buf = String::new();
        let _ = reader.read_to_string(&mut err_buf);
        return Err(format!("Upstream {}: {}", status_line.trim(), err_buf));
    }

    let mut pending = Vec::new();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tc_map: std::collections::BTreeMap<i64, (String, String, String)> = std::collections::BTreeMap::new();
    let mut ptok = 0u64;
    let mut ctok = 0u64;
    let mut cache_n = 0u64;
    let mut final_tps = 0.0f64;
    let mut first_tok_at: Option<Instant> = None;
    let mut chunk_toks = 0u64;
    let mut early_stopped = false;

    while let Some(raw_line) = read_next_sse_line(&mut reader, is_chunked, &mut pending)? {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("data:") else { continue };
        let data = rest.trim();
        if data == "[DONE]" {
            break;
        }
        let Ok(obj) = Json::parse(data) else { continue };

        if let Some(u) = obj.get("usage") {
            ptok = u.get("prompt_tokens").and_then(|n| n.as_i64()).unwrap_or(ptok as i64) as u64;
            ctok = u.get("completion_tokens").and_then(|n| n.as_i64()).unwrap_or(ctok as i64) as u64;
        }
        if let Some(tm) = obj.get("timings") {
            cache_n = tm.get("cache_n").and_then(|n| n.as_i64()).unwrap_or(cache_n as i64) as u64;
            final_tps = tm.get("predicted_per_second").and_then(|n| n.as_f64()).unwrap_or(final_tps);
        }

        let delta = obj
            .get("choices")
            .and_then(|c| c.as_arr())
            .and_then(|a| a.first())
            .and_then(|c| c.get("delta"));

        if let Some(d) = delta {
            let mut got_tok = false;
            if let Some(rc) = d.get("reasoning_content").and_then(|s| s.as_str()) {
                if !rc.is_empty() {
                    got_tok = true;
                    reasoning.push_str(rc);
                }
            }
            if let Some(txt) = d.get("content").and_then(|s| s.as_str()) {
                if !txt.is_empty() {
                    got_tok = true;
                    content.push_str(txt);
                }
            }
            if let Some(tcs) = d.get("tool_calls").and_then(|a| a.as_arr()) {
                for tc in tcs {
                    got_tok = true;
                    let idx = tc.get("index").and_then(|n| n.as_i64()).unwrap_or(0);
                    let entry = tc_map
                        .entry(idx)
                        .or_insert_with(|| (format!("call_{}", idx), String::new(), String::new()));
                    if let Some(id) = tc.get("id").and_then(|s| s.as_str()) {
                        if !id.is_empty() {
                            entry.0 = id.to_string();
                        }
                    }
                    if let Some(fn_obj) = tc.get("function") {
                        if let Some(n) = fn_obj.get("name").and_then(|s| s.as_str()) {
                            entry.1.push_str(n);
                        }
                        if let Some(a) = fn_obj.get("arguments").and_then(|s| s.as_str()) {
                            entry.2.push_str(a);
                        }
                    }
                }
            }
            if got_tok {
                let first = *first_tok_at.get_or_insert_with(Instant::now);
                chunk_toks += 1;
                let gen_s = first.elapsed().as_secs_f64().max(0.15);
                let live_tps = if chunk_toks >= 2 { chunk_toks as f64 / gen_s } else { 0.0 };
                let preview = if let Some((_, (_, fname, fargs))) = tc_map.iter().next_back() {
                    let arg_tail: String = fargs.chars().rev().take(260).collect::<String>().chars().rev().collect();
                    format!("Calling {}({})", fname, arg_tail)
                } else {
                    reasoning.chars().rev().take(260).collect::<String>().chars().rev().collect()
                };
                if chunk_toks == 1 || chunk_toks % 3 == 0 {
                    let mut map = state.tasks.lock().unwrap();
                    if let Some(tr) = map.get_mut(task_id) {
                        tr.elapsed_s = t0.elapsed().as_secs_f64();
                        tr.live_phase = format!("Turn {} · Generating tokens", turn);
                        tr.live_tps = live_tps;
                        tr.live_tokens = chunk_toks;
                        tr.live_stream = preview;
                    }
                }
                if tool_stream_should_stop(early_stop_no_tool, !tc_map.is_empty(), &content, chunk_toks) {
                    early_stopped = true;
                    break;
                }
            }
        }
    }

    let eff_tps = if final_tps > 0.0 {
        final_tps
    } else if let Some(first) = first_tok_at {
        chunk_toks as f64 / first.elapsed().as_secs_f64().max(0.1)
    } else {
        0.0
    };
    let tool_calls: Vec<Json> = tc_map
        .into_values()
        .map(|(id, name, args)| {
            Json::obj(vec![
                ("id", Json::Str(id)),
                ("type", Json::Str("function".into())),
                ("function", Json::obj(vec![("name", Json::Str(name)), ("arguments", Json::Str(args))])),
            ])
        })
        .collect();
    if ctok == 0 {
        ctok = chunk_toks;
    }

    Ok(StreamResult {
        content,
        reasoning_preview: reasoning.chars().take(180).collect(),
        tool_calls,
        prompt_tokens: ptok,
        completion_tokens: ctok,
        cache_n,
        tok_per_sec: eff_tps,
        early_stopped,
    })
}

pub fn spawn_agent_task(state: Arc<ServerState>, prompt: String) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let task_id = format!("task_{}_{}", ts, (*state.requests_served.lock().unwrap() + 1));
    {
        let mut map = state.tasks.lock().unwrap();
        for tr in map.values_mut() {
            if tr.status == "running" {
                tr.status = "cancelled".into();
                tr.error = "Superseded by new task".into();
            }
        }
        map.insert(
            task_id.clone(),
            TaskRun {
                id: task_id.clone(),
                prompt: prompt.clone(),
                status: "running".into(),
                elapsed_s: 0.0,
                live_phase: "Indexing AST & repo memory...".into(),
                live_tps: 0.0,
                live_tokens: 0,
                total_tokens: 0,
                last_cache_n: 0,
                last_prompt_tokens: 0,
                live_stream: String::new(),
                turns: Vec::new(),
                final_result: String::new(),
                error: String::new(),
                diff: String::new(),
            },
        );
    }

    let tid = task_id.clone();
    std::thread::spawn(move || run_agent_loop(state, tid, prompt));
    task_id
}

fn run_agent_loop(state: Arc<ServerState>, task_id: String, prompt: String) {
    let t0 = Instant::now();
    let repo_root = state.repo_root.clone();
    let repo_display = repo_root.to_string_lossy().into_owned();
    let repo_name = crate::repo::repo_display_name(&repo_root);

    let (needs_ctx, frozen_ctx, _lesson_cnt) = {
        let mut st = state.store.lock().unwrap();
        let _ = st.sync();
        let needed = st.needs_repo_context(&prompt);
        let ctx = if needed { st.build_repo_context(&prompt, 1800) } else { String::new() };
        (needed, ctx, st.lessons.len())
    };

    if !needs_ctx {
        {
            let mut map = state.tasks.lock().unwrap();
            if let Some(tr) = map.get_mut(&task_id) {
                tr.live_phase = "Replying...".into();
                tr.elapsed_s = t0.elapsed().as_secs_f64();
            }
        }
        let chat_sys = format!(
            "You are Kraits, a fast coding assistant bound exclusively to the `{}` repository. Always state cleanly that you are Kraits bound to `{}` when greeting the user, and offer to inspect, edit, or test files in `{}`.",
            repo_name, repo_name, repo_name
        );
        let payload = Json::obj(vec![
            ("model", Json::Str("Qwen3.8-Flash-Next-IQ4_XS".into())),
            ("messages", Json::Arr(vec![
                Json::obj(vec![("role", Json::Str("system".into())), ("content", Json::Str(chat_sys))]),
                Json::obj(vec![("role", Json::Str("user".into())), ("content", Json::Str(prompt))]),
            ])),
            ("temperature", Json::Num(0.4)),
            ("max_tokens", Json::Num(512.0)),
            ("reasoning_budget", Json::Num(0.0)),
            ("chat_template_kwargs", Json::obj(vec![("enable_thinking", Json::Bool(false))])),
            ("stream", Json::Bool(true)),
            ("stream_options", Json::obj(vec![("include_usage", Json::Bool(true))])),
        ]);
        match post_upstream_chat_streaming(&state, &task_id, 1, t0, &payload, false) {
            Ok(sr) => {
                let mut map = state.tasks.lock().unwrap();
                if let Some(tr) = map.get_mut(&task_id) {
                    tr.live_tps = sr.tok_per_sec;
                    tr.total_tokens = sr.completion_tokens;
                    tr.last_cache_n = sr.cache_n;
                    tr.last_prompt_tokens = sr.prompt_tokens;
                    tr.turns.push(TurnTrace {
                        turn: 1,
                        prompt_tokens: sr.prompt_tokens,
                        completion_tokens: sr.completion_tokens,
                        cache_n: sr.cache_n,
                        tok_per_sec: sr.tok_per_sec,
                        wall_s: t0.elapsed().as_secs_f64(),
                        reasoning_preview: sr.reasoning_preview,
                        tools: Vec::new(),
                    });
                    tr.status = "completed".into();
                    tr.live_phase = "Chat reply".into();
                    tr.live_stream.clear();
                    tr.diff.clear();
                    tr.final_result = sr.content;
                    tr.elapsed_s = t0.elapsed().as_secs_f64();
                }
            }
            Err(e) => {
                let mut map = state.tasks.lock().unwrap();
                if let Some(tr) = map.get_mut(&task_id) {
                    tr.status = "failed".into();
                    tr.live_phase = "Failed".into();
                    tr.error = e;
                    tr.elapsed_s = t0.elapsed().as_secs_f64();
                }
            }
        }
        return;
    }

    let sys_prompt = format!(
        "You are Kraits, a senior systems engineer bound to `{}` (`{}`).\n\
         - Output ONLY structured JSON tool calls — NEVER output prose outside `attempt_completion` or `<invoke>` tags.\n\
         - `[Kraits Repo Adaptor]` ALREADY gives the repo AST tree, toolchain, and learned lessons.\n\
         - Use `ast_symbol(path, symbol)` or `search_repo(query)` first; use `read_file` with positive `1 <= start_line <= end_line` (max 100 lines). Never re-read overlapping line ranges.\n\
         - Patch cleanly with `replace_in_file` (or `write_to_file` for new test/source files requested), then run `execute_command` (`python -m pytest ...`, PowerShell syntax, no `&&`).\n\
         - Call `attempt_completion` with a 1-2 sentence plain English summary as soon as `execute_command` passes (or immediately for read-only questions).",
        repo_name, repo_display
    );

    let mut messages = vec![
        Json::obj(vec![("role", Json::Str("system".into())), ("content", Json::Str(sys_prompt))]),
        Json::obj(vec![
            ("role", Json::Str("user".into())),
            ("content", Json::Str(format!("{}\n\n# Task\n{}", frozen_ctx, prompt))),
        ]),
    ];

    let tools_schema = build_tools_schema();
    let readonly_q = is_readonly_question(&prompt);
    let mut verified_pass = false;
    let mut applied_patches: Vec<String> = Vec::new();
    let mut edited_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut seen_reads: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut read_ranges: std::collections::HashMap<String, Vec<(usize, usize)>> = std::collections::HashMap::new();
    let mut seen_actions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut cached_action_outputs: HashMap<String, String> = HashMap::new();
    let mut cached_read_outputs: HashMap<String, String> = HashMap::new();
    let mut consec_edits_by_file: BTreeMap<String, usize> = BTreeMap::new();
    let mut syntax_reverts: usize = 0;
    let mut turns_without_edit: usize = 0;
    let mut total_edit_attempts: usize = 0;
    let mut total_tool_errors: usize = 0;
    let mut last_test_cmd = String::new();
    let mut consec_cmd_fails: usize = 0;
    let mut consec_val_fails: usize = 0;
    let mut consec_identical_repeats: usize = 0;
    let mut last_ok_summary = String::new();
    let mut abort_err: Option<String> = None;
    let mut last_prose = String::new();
    let mut valid_turns_used: usize = 0;
    let max_turns = 9usize;
    let max_total_iters = 14usize;

    for total_iter in 1..=max_total_iters {
        if valid_turns_used >= max_turns {
            break;
        }
        let turn = valid_turns_used + 1;
        let _ = total_iter;
        let turn_t0 = Instant::now();
        compact_messages_for_ctx(&mut messages, 27_500);
        {
            let mut map = state.tasks.lock().unwrap();
            let Some(tr) = map.get_mut(&task_id) else { return };
            if tr.status != "running" {
                return;
            }
            tr.live_phase = format!("Turn {} · Prefilling KV cache...", turn);
            tr.live_tokens = 0;
            tr.live_stream.clear();
            tr.elapsed_s = t0.elapsed().as_secs_f64();
        }
        let completion_turn = !edited_files.is_empty() && verified_pass;
        let expect_tool = !readonly_q || valid_turns_used == 0 || completion_turn;
        let turn_max_tokens = if completion_turn {
            200.0
        } else if readonly_q && valid_turns_used >= 1 {
            384.0
        } else {
            1024.0
        };
        let payload = Json::obj(vec![
            ("model", Json::Str("Qwen3.8-Flash-Next-IQ4_XS".into())),
            ("messages", Json::Arr(messages.clone())),
            ("tools", tools_schema.clone()),
            ("tool_choice", Json::Str("required".into())),
            ("parallel_tool_calls", Json::Bool(true)),
            ("temperature", Json::Num(0.1)),
            ("repeat_penalty", Json::Num(1.0)),
            ("max_tokens", Json::Num(turn_max_tokens)),
            ("reasoning_budget", Json::Num(0.0)),
            ("chat_template_kwargs", Json::obj(vec![("enable_thinking", Json::Bool(false))])),
            ("stream", Json::Bool(true)),
            ("stream_options", Json::obj(vec![("include_usage", Json::Bool(true))])),
        ]);

        let sr = match post_upstream_chat_streaming(&state, &task_id, turn, t0, &payload, expect_tool) {
            Ok(v) => v,
            Err(e) => {
                let mut map = state.tasks.lock().unwrap();
                if let Some(tr) = map.get_mut(&task_id) {
                    tr.status = "failed".into();
                    tr.live_phase = "Failed".into();
                    tr.error = e;
                    tr.elapsed_s = t0.elapsed().as_secs_f64();
                }
                return;
            }
        };

        let ptok = sr.prompt_tokens;
        let ctok = sr.completion_tokens;
        let cache_n = sr.cache_n;
        let tps = sr.tok_per_sec;
        let content_txt = sr.content;
        let reasoning_txt = sr.reasoning_preview;
        let raw_tcs = sr.tool_calls;

        // Keep only tool calls whose JSON arguments parse cleanly so upstream --jinja never 500s
        let mut valid_tcs = Vec::new();
        let mut parsed_args = Vec::new();
        let mut raw_tc_preview = String::new();
        let mut partial_completion_txt: Option<String> = None;
        for tc in raw_tcs {
            let fname = tc.get("function").and_then(|f| f.get("name")).and_then(|s| s.as_str()).unwrap_or("");
            let arg_str = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if raw_tc_preview.is_empty() && (!fname.is_empty() || !arg_str.is_empty()) {
                let tail: String = arg_str.chars().take(110).collect();
                raw_tc_preview = format!("truncated_tool({}: {})", fname, tail);
            }
            if partial_completion_txt.is_none() && fname == "attempt_completion" {
                partial_completion_txt = extract_partial_completion_text(arg_str);
            }
            if let Ok(a) = Json::parse(arg_str) {
                valid_tcs.push(tc);
                parsed_args.push(a);
            }
        }

        let trimmed_prose = content_txt.trim();
        if !trimmed_prose.is_empty() {
            last_prose = trimmed_prose.to_string();
        }

        if valid_tcs.is_empty() {
            let wall = turn_t0.elapsed().as_secs_f64();
            let clean_answer = !trimmed_prose.is_empty() && !trimmed_prose.contains("<invoke") && !trimmed_prose.contains("<tool_call");
            let mut final_prose_opt: Option<String> = if clean_answer && ((edited_files.is_empty() && readonly_q && turn >= 2) || completion_turn) {
                let cand = clean_prose_text(trimmed_prose);
                let ok_summary = {
                    let st = state.store.lock().unwrap();
                    is_clean_english_summary_with_index(&cand, Some(&st.files))
                };
                if ok_summary { Some(cand) } else { None }
            } else {
                None
            };
            if final_prose_opt.is_none() && ((edited_files.is_empty() && readonly_q) || verified_pass) {
                if let Some(partial) = partial_completion_txt {
                    let ok_summary = {
                        let st = state.store.lock().unwrap();
                        is_clean_english_summary_with_index(&partial, Some(&st.files))
                    };
                    if ok_summary {
                        final_prose_opt = Some(partial);
                    }
                }
            }
            if final_prose_opt.is_none() && completion_turn {
                final_prose_opt = Some(deterministic_fix_summary(&edited_files, &last_ok_summary));
            }
            if final_prose_opt.is_none() && readonly_q && edited_files.is_empty() && (raw_tc_preview.starts_with("truncated_tool(attempt_completion") || (!read_ranges.is_empty() && (trimmed_prose.contains("<tool_call") || trimmed_prose.contains("attempt_completion") || turn >= 4))) {
                if let Some((synth, _, _, _, _)) = synthesize_readonly_prose(&state, &task_id, turn, t0, &messages, &tools_schema, &prompt) {
                    final_prose_opt = Some(synth);
                }
            }
            let accept_prose_done = final_prose_opt.is_some();
            let snip: String = if sr.early_stopped {
                format!("(aborted no-tool stream at {} tok) {}", ctok, trimmed_prose.chars().take(80).collect::<String>())
            } else if !trimmed_prose.is_empty() {
                trimmed_prose.chars().take(120).collect()
            } else if !raw_tc_preview.is_empty() {
                raw_tc_preview
            } else if !reasoning_txt.trim().is_empty() {
                format!("reasoning: {}", reasoning_txt.trim().chars().take(100).collect::<String>())
            } else {
                format!("(empty ctok={})", ctok)
            };

            valid_turns_used += 1;
            {
                let mut map = state.tasks.lock().unwrap();
                if let Some(tr) = map.get_mut(&task_id) {
                    if tr.status != "running" {
                        return;
                    }
                    tr.live_tps = tps;
                    tr.total_tokens += ctok;
                    tr.last_cache_n = cache_n;
                    tr.last_prompt_tokens = ptok;
                    let trace_turn = tr.turns.len() + 1;
                    tr.turns.push(TurnTrace {
                        turn: trace_turn,
                        prompt_tokens: ptok,
                        completion_tokens: ctok,
                        cache_n,
                        tok_per_sec: tps,
                        wall_s: wall,
                        reasoning_preview: reasoning_txt,
                        tools: vec![ToolTrace {
                            name: "prose".into(),
                            detail: if snip.is_empty() { "(empty)".into() } else { snip },
                            ok: accept_prose_done,
                        }],
                    });
                    tr.elapsed_s = t0.elapsed().as_secs_f64();
                    if let Some(fin_prose) = final_prose_opt {
                        tr.status = "completed".into();
                        tr.live_phase = if edited_files.is_empty() { "Answered".into() } else { "Verified & Fixed".into() };
                        tr.live_stream.clear();
                        tr.final_result = fin_prose;
                        return;
                    }
                }
            }

            let capped_prose: String = trimmed_prose.chars().take(480).collect();
            messages.push(Json::obj(vec![
                ("role", Json::Str("assistant".into())),
                ("content", Json::Str(capped_prose)),
            ]));
            messages.push(Json::obj(vec![
                ("role", Json::Str("user".into())),
                ("content", Json::Str("Return ONLY a structured JSON tool call (`ast_symbol`, `search_repo`, `read_file`, `replace_in_file`, `execute_command`, or `attempt_completion`).".into())),
            ]));
            continue;
        }

        messages.push(Json::obj(vec![
            ("role", Json::Str("assistant".into())),
            ("content", Json::Str(content_txt)),
            ("tool_calls", Json::Arr(valid_tcs.clone())),
        ]));

        let mut tool_traces = Vec::new();
        let mut finished: Option<String> = None;
        let mut step_ran_real_tool = false;
        let mut step_had_val_miss = false;
        let mut step_had_repeat = false;

        for (tc, args) in valid_tcs.iter().zip(parsed_args.iter()) {
            let name = tc.get("function").and_then(|f| f.get("name")).and_then(|s| s.as_str()).unwrap_or("");
            {
                let mut map = state.tasks.lock().unwrap();
                if let Some(tr) = map.get_mut(&task_id) {
                    tr.live_phase = format!("Turn {} · Running `{}`...", turn, name);
                    tr.elapsed_s = t0.elapsed().as_secs_f64();
                }
            }
            let tid = tc.get("id").and_then(|s| s.as_str()).unwrap_or("call_0");
            let mut ok = true;

            // F16: Pre-validate path and AST symbol BEFORE running any tool
            let val_res = {
                let st = state.store.lock().unwrap();
                prevalidate_tool_call(&st, &repo_root, name, args)
            };
            if let Err((val_detail, val_msg)) = val_res {
                step_had_val_miss = true;
                consec_val_fails += 1;
                if consec_val_fails >= 2 {
                    consec_val_fails = 0;
                    step_ran_real_tool = true;
                    let q = infer_search_token(name, args);
                    let search_out = {
                        let st = state.store.lock().unwrap();
                        run_search_repo_query(&st, &repo_root, &q, "")
                    };
                    let (d, txt, sok) = match search_out {
                        Ok(hits) => (format!("forced search_repo `{}`", q), format!("Validation failed twice; forced search_repo(`{}`) found:\n{}", q, hits), true),
                        Err(e) => (format!("forced search_repo `{}` (0)", q), e, false),
                    };
                    tool_traces.push(ToolTrace { name: "search_repo".into(), detail: d, ok: sok });
                    messages.push(Json::obj(vec![
                        ("role", Json::Str("tool".into())),
                        ("tool_call_id", Json::Str(tid.to_string())),
                        ("content", Json::Str(txt)),
                    ]));
                } else {
                    tool_traces.push(ToolTrace { name: name.to_string(), detail: val_detail, ok: false });
                    messages.push(Json::obj(vec![
                        ("role", Json::Str("tool".into())),
                        ("tool_call_id", Json::Str(tid.to_string())),
                        ("content", Json::Str(val_msg)),
                    ]));
                }
                continue;
            }

            // F14: Block identical repeated tool calls with no intervening edit; serve cached read output in-process
            let action_key = format!("{}:{}", name, args.encode());
            if matches!(name, "ast_symbol" | "read_file" | "search_repo" | "replace_in_file") && seen_actions.get(&action_key).copied().unwrap_or(0) >= 1 {
                step_had_repeat = true;
                let msg = if let Some(cached) = cached_action_outputs.get(&action_key) {
                    format!(
                        "CACHED RESULT (identical `{}` already ran with no intervening edit; do NOT repeat it — act on this or call `attempt_completion`):\n\n{}",
                        name, cached
                    )
                } else {
                    format!(
                        "REPEAT BLOCKED: identical `{}` call already executed with no edit in between. Do NOT repeat it; apply a different edit, test command, or `attempt_completion`.",
                        name
                    )
                };
                tool_traces.push(ToolTrace { name: name.to_string(), detail: format!("blocked repeat `{}` (cached)", name), ok: false });
                messages.push(Json::obj(vec![
                    ("role", Json::Str("tool".into())),
                    ("tool_call_id", Json::Str(tid.to_string())),
                    ("content", Json::Str(msg)),
                ]));
                continue;
            }
            *seen_actions.entry(action_key.clone()).or_insert(0) += 1;
            consec_val_fails = 0;
            step_ran_real_tool = true;
            let mut forced_verify_after: Option<String> = None;

            let (detail_summary, result_txt) = match name {
                "ast_symbol" => {
                    let raw_rel = args.get("path").and_then(|s| s.as_str()).unwrap_or("");
                    let rel = clean_tool_path(raw_rel);
                    let sym = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                    let cache_key = format!("ast:{}::{}", rel, sym);
                    if seen_reads.contains(&cache_key) {
                        step_had_repeat = true;
                        step_ran_real_tool = false;
                        let cached_body = cached_read_outputs.get(&cache_key).cloned().unwrap_or_else(|| {
                            format!("Already read `{} :: {}` earlier (unchanged). Use the code already in context instead of re-reading.", rel, sym)
                        });
                        (format!("{} :: {} (cached)", rel, sym), format!("CACHED RESULT (`{} :: {}` unchanged — do NOT re-read):\n\n{}", rel, sym, cached_body))
                    } else {
                        let st = state.store.lock().unwrap();
                        match st.extract_ast_symbol(&rel, sym) {
                            Some(mut body) => {
                                if body.len() > 5500 {
                                    let cut = body.char_indices().take_while(|(i, _)| *i <= 5400).last().map(|(i, _)| i).unwrap_or(5400);
                                    body.truncate(cut);
                                    body.push_str("\n... (truncated AST output at 5400 chars; read specific child symbol or line range)");
                                }
                                seen_reads.insert(cache_key.clone());
                                cached_read_outputs.insert(cache_key, body.clone());
                                (format!("{} :: {}", rel, sym), body)
                            }
                            None => {
                                ok = false;
                                let skel = st.files.get(&rel).map(|f| f.render_skeleton()).unwrap_or_default();
                                let hint = if skel.is_empty() {
                                    let suggestions = suggest_repo_paths(&st, &rel);
                                    if suggestions.is_empty() {
                                        format!("ERROR: File/symbol `{}` not found in `{}`", sym, rel)
                                    } else {
                                        format!("ERROR: File `{}` not found. Closest indexed files: {}", rel, suggestions.join(", "))
                                    }
                                } else {
                                    format!("ERROR: Symbol `{}` not found in `{}`. File AST symbols:\n{}", sym, rel, skel)
                                };
                                (format!("{} :: {}", rel, sym), hint)
                            }
                        }
                    }
                }
                "search_repo" => {
                    let q = args.get("query").and_then(|s| s.as_str()).unwrap_or("").trim();
                    let p_filter = args.get("path").and_then(|s| s.as_str()).unwrap_or("").replace('\\', "/");
                    if q.is_empty() {
                        ok = false;
                        ("empty query".into(), "ERROR: `query` is required".into())
                    } else {
                        let q_low = q.to_ascii_lowercase();
                        let st = state.store.lock().unwrap();
                        let mut hits = Vec::new();
                        let mut total_chars = 0usize;
                        'outer: for rel in st.files.keys() {
                            if !p_filter.is_empty() && !rel.contains(&p_filter) {
                                continue;
                            }
                            let Ok(raw) = fs::read_to_string(repo_root.join(rel)) else { continue };
                            for (idx, line) in raw.lines().enumerate() {
                                if line.to_ascii_lowercase().contains(&q_low) {
                                    let trimmed = line.trim();
                                    let snip: String = trimmed.chars().take(180).collect();
                                    let entry = format!("{}:{}: {}", rel, idx + 1, snip);
                                    total_chars += entry.len() + 1;
                                    hits.push(entry);
                                    if hits.len() >= 30 || total_chars >= 3500 {
                                        break 'outer;
                                    }
                                }
                            }
                        }
                        if hits.is_empty() {
                            ok = false;
                            (format!("`{}` (0 matches)", q), format!("No matches found for `{}`", q))
                        } else {
                            (format!("`{}` ({} matches)", q, hits.len()), hits.join("\n"))
                        }
                    }
                }
                "read_file" => {
                    let raw_rel = args.get("path").and_then(|s| s.as_str()).unwrap_or("");
                    let rel = clean_tool_path(raw_rel);
                    let s_raw = args.get("start_line").and_then(|n| n.as_i64()).unwrap_or(1);
                    let e_opt = args.get("end_line").and_then(|n| n.as_i64());
                    let s_chk = s_raw.max(1) as usize;
                    let e_chk = e_opt.map(|n| n.max(1) as usize).unwrap_or(s_chk + 99).min(s_chk + 99);
                    let cache_key = format!("read:{}:{}:{}", rel, s_raw, e_opt.unwrap_or(-1));
                    let overlap_hit = read_ranges.get(&rel).and_then(|v| overlaps_seen_range(v, s_chk, e_chk));
                    if seen_reads.contains(&cache_key) || overlap_hit.is_some() {
                        step_had_repeat = true;
                        step_ran_real_tool = false;
                        let ov_note = overlap_hit.map(|(a, b)| format!("L{}-L{}", a, b)).unwrap_or_else(|| format!("L{}-L{}", s_chk, e_chk));
                        let cached_slice = overlap_hit
                            .and_then(|(a, b)| cached_read_outputs.get(&format!("range:{}:{}:{}", rel, a, b)))
                            .or_else(|| cached_read_outputs.get(&cache_key))
                            .cloned();
                        let msg = match cached_slice {
                            Some(prev_txt) => format!("CACHED RESULT (`{}` {} already in context — do NOT re-read; edit or call `attempt_completion`):\n\n{}", rel, ov_note, prev_txt),
                            None => format!("Already read `{}` ({}) in context (unchanged). Do NOT re-read overlapping lines; edit with `replace_in_file` or inspect lines outside {}.", rel, ov_note, ov_note),
                        };
                        (format!("{} (cached)", rel), msg)
                    } else {
                        match safe_rel_path(&repo_root, &rel).and_then(|p| fs::read_to_string(p).map_err(|e| e.to_string())) {
                            Ok(raw) => {
                                let lines: Vec<&str> = raw.lines().collect();
                                if s_raw < 1 || (s_raw as usize) > lines.len() {
                                    ok = false;
                                    (format!("{} (invalid line {})", rel, s_raw), format!("ERROR: start_line {} out of bounds (file {} has {} lines).", s_raw, rel, lines.len()))
                                } else if let Some(e_raw) = e_opt.filter(|&e| e < s_raw) {
                                    ok = false;
                                    (format!("{} (L{}-L{})", rel, s_raw, e_raw), format!("ERROR: invalid range start_line={} > end_line={} (file {} has {} lines).", s_raw, e_raw, rel, lines.len()))
                                } else {
                                    let s_line = s_raw as usize;
                                    let req_end = e_opt.map(|n| n as usize).unwrap_or(s_line + 99);
                                    let e_line = req_end.min(s_line + 99).min(lines.len());
                                    let mut out = String::new();
                                    let mut actual_end = s_line;
                                    for idx in (s_line - 1)..e_line {
                                        let ln = format!("{}: {}\n", idx + 1, lines[idx]);
                                        if out.len() + ln.len() > 4000 {
                                            out.push_str(&format!("... (capped at 4000 chars around line {} of {}; pass start_line={} to continue)", idx + 1, lines.len(), idx + 1));
                                            break;
                                        }
                                        out.push_str(&ln);
                                        actual_end = idx + 1;
                                    }
                                    if e_line < req_end.min(lines.len()) && !out.contains("capped at") {
                                        out.push_str(&format!("... (capped at 100 lines of {}; pass start_line={} or use ast_symbol/search_repo)", lines.len(), e_line + 1));
                                    }
                                    seen_reads.insert(cache_key.clone());
                                    seen_reads.insert(format!("read:{}:{}:{}", rel, s_line, actual_end));
                                    cached_read_outputs.insert(cache_key, out.clone());
                                    cached_read_outputs.insert(format!("range:{}:{}:{}", rel, s_line, actual_end), out.clone());
                                    read_ranges.entry(rel.clone()).or_default().push((s_line, actual_end));
                                    (format!("{} L{}-L{}", rel, s_line, actual_end), out)
                                }
                            }
                            Err(e) => {
                                ok = false;
                                let st = state.store.lock().unwrap();
                                let suggestions = suggest_repo_paths(&st, &rel);
                                let hint = if suggestions.is_empty() {
                                    format!("ERROR: {}", e)
                                } else {
                                    format!("ERROR: {} — Did you mean one of: {}?", e, suggestions.join(", "))
                                };
                                (rel, hint)
                            }
                        }
                    }
                }
                "replace_in_file" => {
                    let rel = args.get("path").and_then(|s| s.as_str()).unwrap_or("");
                    let old_t = args.get("old_text").and_then(|s| s.as_str()).unwrap_or("");
                    let new_t = args.get("new_text").and_then(|s| s.as_str()).unwrap_or("");
                    match safe_rel_path(&repo_root, rel) {
                        Ok(p) => match fs::read_to_string(&p) {
                            Ok(raw) => {
                                let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
                                let body = raw.replace("\r\n", "\n");
                                let old_n = old_t.replace("\r\n", "\n");
                                let new_n = new_t.replace("\r\n", "\n");
                                let cnt = body.matches(&old_n).count();
                                if cnt != 1 {
                                    ok = false;
                                    (rel.to_string(), format!("ERROR: old_text matched {} times in {}; must match once", cnt, rel))
                                } else {
                                    let updated = body.replacen(&old_n, &new_n, 1).replace('\n', eol);
                                    if let Err(e) = fs::write(&p, &updated) {
                                        ok = false;
                                        (rel.to_string(), format!("ERROR: {}", e))
                                    } else if let Err(syn_err) = check_python_syntax(&repo_root, &p) {
                                        let _ = fs::write(&p, &raw);
                                        ok = false;
                                        syntax_reverts += 1;
                                        if syntax_reverts >= 3 {
                                            abort_err = Some(deterministic_failure_summary(&edited_files, "3 Python syntax reverts"));
                                        }
                                        (format!("{} (syntax reverted)", rel), format!("ERROR: Python syntax error (edit reverted cleanly): {}", syn_err))
                                    } else {
                                        verified_pass = false;
                                        consec_cmd_fails = 0;
                                        turns_without_edit = 0;
                                        total_edit_attempts += 1;
                                        seen_actions.clear();
                                        cached_action_outputs.clear();
                                        let norm_rel = rel.replace('\\', "/").trim_start_matches("./").to_string();
                                        cached_read_outputs.retain(|k, _| !k.contains(&norm_rel));
                                        edited_files.insert(norm_rel.clone());
                                        seen_reads.retain(|k| !k.contains(&norm_rel));
                                        read_ranges.remove(&norm_rel);
                                        applied_patches.push(build_unified_hunk(&norm_rel, &body, &old_n, &new_n));
                                        let edit_cnt = {
                                            let e = consec_edits_by_file.entry(norm_rel.clone()).or_insert(0);
                                            *e += 1;
                                            *e
                                        };
                                        if edit_cnt >= 2 {
                                            forced_verify_after = Some(norm_rel.clone());
                                        }
                                        let mut st = state.store.lock().unwrap();
                                        let _ = st.sync();
                                        (rel.to_string(), format!("Replaced 1 occurrence in {} (syntax OK). If all requested edits and new test files are done, run execute_command to verify pytest; otherwise finish any remaining files first.", rel))
                                    }
                                }
                            }
                            Err(e) => {
                                ok = false;
                                let st = state.store.lock().unwrap();
                                let suggestions = suggest_repo_paths(&st, rel);
                                let hint = if suggestions.is_empty() { format!("ERROR: {}", e) } else { format!("ERROR: {} — Did you mean one of: {}?", e, suggestions.join(", ")) };
                                (rel.to_string(), hint)
                            }
                        },
                        Err(e) => {
                            ok = false;
                            (rel.to_string(), format!("ERROR: {}", e))
                        }
                    }
                }
                "write_to_file" => {
                    let rel = args.get("path").and_then(|s| s.as_str()).unwrap_or("");
                    let content = args.get("content").and_then(|s| s.as_str()).unwrap_or("");
                    let norm_rel = rel.replace('\\', "/").trim_start_matches("./").to_string();
                    let write_res = {
                        let mut st = state.store.lock().unwrap();
                        apply_guarded_write_to_file(&repo_root, &mut st, rel, content)
                    };
                    match write_res {
                        Ok(was_new) => {
                            verified_pass = false;
                            consec_cmd_fails = 0;
                            turns_without_edit = 0;
                            total_edit_attempts += 1;
                            seen_actions.clear();
                            cached_action_outputs.clear();
                            cached_read_outputs.retain(|k, _| !k.contains(&norm_rel));
                            edited_files.insert(norm_rel.clone());
                            seen_reads.retain(|k| !k.contains(&norm_rel));
                            read_ranges.remove(&norm_rel);
                            if was_new {
                                let added: Vec<String> = content.lines().map(|l| format!("+{}", l)).collect();
                                applied_patches.push(format!("diff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n@@ -0,0 +1,{1} @@\n{2}", norm_rel, added.len().max(1), added.join("\n")));
                            }
                            let edit_cnt = {
                                let e = consec_edits_by_file.entry(norm_rel.clone()).or_insert(0);
                                *e += 1;
                                *e
                            };
                            if edit_cnt >= 2 {
                                forced_verify_after = Some(norm_rel.clone());
                            }
                            (format!("{} ({} chars)", rel, content.len()), format!("Wrote {} chars to {} (syntax & size OK). Next: run execute_command to verify tests.", content.len(), rel))
                        }
                        Err((det, err_msg)) => {
                            ok = false;
                            if det.contains("syntax reverted") {
                                syntax_reverts += 1;
                                if syntax_reverts >= 3 {
                                    abort_err = Some(deterministic_failure_summary(&edited_files, "3 Python syntax reverts"));
                                }
                            }
                            (det, err_msg)
                        }
                    }
                }
                "execute_command" => {
                    let cmd = args.get("command").and_then(|s| s.as_str()).unwrap_or("").trim();
                    let bad_repl_flag = cmd.contains("python - ") || cmd.contains("python.exe - ");
                    if bad_repl_flag {
                        ok = false;
                        verified_pass = false;
                        consec_cmd_fails += 1;
                        (format!("`{}` -> invalid python flag", cmd), "ERROR: Invalid command (`python -` launches interactive stdin). Run `python -m pytest ...` without stray `-`.".into())
                    } else {
                        consec_edits_by_file.clear();
                        if cmd.to_ascii_lowercase().contains("pytest") {
                            last_test_cmd = cmd.to_string();
                        }
                        let (passed, detail, mut out, pass_sum) = {
                            let st = state.store.lock().unwrap();
                            run_verify_in_repo(&repo_root, &st, cmd)
                        };
                        if passed {
                            verified_pass = true;
                            consec_cmd_fails = 0;
                            last_ok_summary = pass_sum;
                            if !applied_patches.is_empty() {
                                out.push_str("\n\nNext: Tests passed (EXIT_CODE=0). Call attempt_completion immediately with a 1-2 sentence plain English summary; do not re-edit or re-run tests.");
                            }
                        } else {
                            ok = false;
                            verified_pass = false;
                            consec_cmd_fails += 1;
                            if consec_cmd_fails == 2 {
                                out.push_str("\n\nWARNING: Tests failed twice in a row with no edit in between. Inspect the traceback and fix the code with `replace_in_file` before running tests again.");
                            } else if consec_cmd_fails >= 3 {
                                abort_err = Some(deterministic_failure_summary(&edited_files, &format!("{} consecutive failing commands ({})", consec_cmd_fails, cmd)));
                            }
                        }
                        (detail, out)
                    }
                }
                "attempt_completion" => {
                    if !edited_files.is_empty() && !verified_pass {
                        ok = false;
                        ("rejected (unverified)".into(), "ERROR: Run execute_command and observe EXIT_CODE=0 after your last edit before calling attempt_completion.".into())
                    } else if edited_files.is_empty() && !readonly_q {
                        let verify_cmd = {
                            let st = state.store.lock().unwrap();
                            extract_verify_command(&prompt, &last_test_cmd, "", &st.files)
                        };
                        last_test_cmd = verify_cmd.clone();
                        let (v_passed, v_detail, v_out, v_sum) = {
                            let st = state.store.lock().unwrap();
                            run_verify_in_repo(&repo_root, &st, &verify_cmd)
                        };
                        if !v_passed {
                            ok = false;
                            verified_pass = false;
                            consec_cmd_fails += 1;
                            if valid_turns_used + 2 >= max_turns || consec_cmd_fails >= 2 {
                                abort_err = Some(deterministic_failure_summary(&edited_files, "completed with zero edits while toolchain tests are red"));
                            }
                            (
                                format!("rejected (zero edits, tests red: {})", v_detail),
                                format!("ERROR: Refused zero-edit completion because `{}` is RED:\n{}\nYou have made 0 edits. Fix the bug with `replace_in_file` and verify before calling `attempt_completion`.", verify_cmd, v_out),
                            )
                        } else {
                            let extracted = clean_prose_text(&extract_readable_completion(args));
                            let resolved_raw = if !extracted.is_empty() { extracted } else { clean_prose_text(&last_prose) };
                            let summary_clean = {
                                let st = state.store.lock().unwrap();
                                is_clean_english_summary_with_index(&resolved_raw, Some(&st.files))
                            };
                            let final_txt = if summary_clean && resolved_raw.chars().any(|c| c.is_ascii_alphabetic()) {
                                resolved_raw
                            } else {
                                format!("Verified with {} (no code edits required).", v_sum)
                            };
                            finished = Some(final_txt);
                            ("verified completion (no edits needed)".into(), "Acknowledged completion.".into())
                        }
                    } else {
                        let extracted = clean_prose_text(&extract_readable_completion(args));
                        let extracted_len = extracted.len();
                        let mut resolved = if !extracted.is_empty() {
                            extracted
                        } else if !last_prose.is_empty() {
                            clean_prose_text(&last_prose)
                        } else {
                            String::new()
                        };
                        let summary_clean = {
                            let st = state.store.lock().unwrap();
                            is_clean_english_summary_with_index(&resolved, Some(&st.files))
                        };
                        let bad_readonly_stub = !summary_clean || resolved.len() < 120;
                        if readonly_q && bad_readonly_stub {
                            if let Some((synth, _, _, _, _)) = synthesize_readonly_prose(&state, &task_id, turn, t0, &messages, &tools_schema, &prompt) {
                                resolved = synth;
                            }
                        }
                        if !edited_files.is_empty() && verified_pass && !summary_clean {
                            resolved = deterministic_fix_summary(&edited_files, &last_ok_summary);
                        }
                        append_debug_log(
                            &repo_root,
                            &format!(
                                "[completion] task={} turn={} readonly={} raw_args={} extracted_len={} last_prose_len={} resolved_len={}",
                                task_id, turn, readonly_q, args.encode(), extracted_len, last_prose.len(), resolved.len()
                            ),
                        );
                        if resolved.trim().is_empty() || !resolved.chars().any(|c| c.is_ascii_alphabetic()) {
                            ok = false;
                            ("rejected (empty/non-alphabetic result)".into(), "ERROR: `result` must be a non-empty plain-text explanation with English prose.".into())
                        } else {
                            finished = Some(resolved);
                            let lbl = if edited_files.is_empty() { "completed (read-only)" } else { "verified completion" };
                            (lbl.into(), "Acknowledged completion.".into())
                        }
                    }
                }
                other => {
                    ok = false;
                    (other.into(), format!("Unknown tool: {}", other))
                }
            };

            if ok && matches!(name, "ast_symbol" | "read_file" | "search_repo") && !step_had_repeat && !result_txt.starts_with("Already read") {
                cached_action_outputs.insert(action_key.clone(), result_txt.clone());
            }
            if !ok {
                total_tool_errors += 1;
            }
            tool_traces.push(ToolTrace {
                name: name.to_string(),
                detail: detail_summary,
                ok,
            });
            let mut final_tool_txt = result_txt;
            if readonly_q && edited_files.is_empty() && ok && turn >= 1 && matches!(name, "ast_symbol" | "read_file" | "search_repo") {
                final_tool_txt.push_str("\n\nNext: This is a read-only question. If this gives enough information to explain clearly, call `attempt_completion` right now instead of reading extra ranges.");
            }
            if let Some(edited_rel) = forced_verify_after {
                let verify_cmd = {
                    let st = state.store.lock().unwrap();
                    extract_verify_command(&prompt, &last_test_cmd, &edited_rel, &st.files)
                };
                consec_edits_by_file.clear();
                last_test_cmd = verify_cmd.clone();
                let (v_passed, v_detail, v_out, v_sum) = {
                    let st = state.store.lock().unwrap();
                    run_verify_in_repo(&repo_root, &st, &verify_cmd)
                };
                tool_traces.push(ToolTrace {
                    name: "execute_command".into(),
                    detail: format!("{} [forced after 2 edits]", v_detail),
                    ok: v_passed,
                });
                final_tool_txt.push_str(&format!("\n\n[F20 Auto-Verify] Two consecutive edits to `{}` ran `{}`:\n{}", edited_rel, verify_cmd, v_out));
                if v_passed {
                    verified_pass = true;
                    consec_cmd_fails = 0;
                    last_ok_summary = v_sum;
                    final_tool_txt.push_str("\n\nNext: Tests passed (EXIT_CODE=0). Call `attempt_completion` immediately with a 1-2 sentence plain English summary; do not edit again.");
                } else {
                    verified_pass = false;
                    consec_cmd_fails += 1;
                    total_tool_errors += 1;
                    if consec_cmd_fails >= 3 {
                        abort_err = Some(deterministic_failure_summary(&edited_files, &format!("{} consecutive failing verifies ({})", consec_cmd_fails, verify_cmd)));
                    }
                }
            }
            messages.push(Json::obj(vec![
                ("role", Json::Str("tool".into())),
                ("tool_call_id", Json::Str(tid.to_string())),
                ("content", Json::Str(final_tool_txt)),
            ]));
            if finished.is_some() || abort_err.is_some() {
                break;
            }
        }

        // Persist newly observed lessons/hot files live into .kraits/memory.jsonl
        {
            let mut st = state.store.lock().unwrap();
            let _ = st.sync();
            st.learn_from_messages(&messages);
        }

        let git_diff = git_diff_paths(&repo_root, &edited_files);
        let diff_now = if git_diff.trim().is_empty() && !applied_patches.is_empty() {
            applied_patches.join("\n")
        } else {
            git_diff
        };
        if step_had_repeat && !step_ran_real_tool {
            consec_identical_repeats += 1;
            let ro_ready = readonly_q && (!read_ranges.is_empty() || !seen_reads.is_empty()) && consec_identical_repeats >= 1;
            if ro_ready && finished.is_none() {
                if let Some((synth, _, _, _, _)) = synthesize_readonly_prose(&state, &task_id, turn, t0, &messages, &tools_schema, &prompt) {
                    finished = Some(synth);
                }
            }
            if consec_identical_repeats >= 2 && finished.is_none() {
                abort_err = Some(deterministic_failure_summary(&edited_files, "repeated identical action loop with no progress"));
            }
        } else {
            consec_identical_repeats = 0;
        }

        let consumes_turn = step_ran_real_tool || finished.is_some() || abort_err.is_some() || !step_had_val_miss;
        if consumes_turn {
            valid_turns_used += 1;
            if edited_files.is_empty() {
                turns_without_edit += 1;
            }
        }

        if readonly_q && !seen_reads.is_empty() && valid_turns_used >= 3 && finished.is_none() && abort_err.is_none() {
            if let Some((synth, _, _, _, _)) = synthesize_readonly_prose(&state, &task_id, turn, t0, &messages, &tools_schema, &prompt) {
                finished = Some(synth);
            }
        }

        if finished.is_none() && abort_err.is_none() {
            if let Some(reason) = should_abort_loop_guard(
                readonly_q,
                edited_files.is_empty(),
                verified_pass,
                turns_without_edit,
                total_edit_attempts,
                total_tool_errors,
            ) {
                abort_err = Some(deterministic_failure_summary(&edited_files, reason));
            } else if !readonly_q && edited_files.is_empty() && turns_without_edit == 3 {
                if let Some(last_msg) = messages.last_mut().and_then(|m| m.as_obj_mut()) {
                    let prev = last_msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    last_msg.insert(
                        "content".into(),
                        Json::Str(format!("{}\n\nDEADLINE WARNING: 3 turns used with 0 edits. Apply a patch or repro test (`replace_in_file` / `write_to_file`) on the NEXT turn or the task will abort.", prev)),
                    );
                }
            }
        }

        let wall = turn_t0.elapsed().as_secs_f64();
        {
            let mut map = state.tasks.lock().unwrap();
            if let Some(tr) = map.get_mut(&task_id) {
                if tr.status != "running" {
                    return;
                }
                tr.live_tps = tps;
                tr.total_tokens += ctok;
                tr.last_cache_n = cache_n;
                tr.last_prompt_tokens = ptok;
                if consumes_turn {
                    let trace_turn = tr.turns.len() + 1;
                    tr.turns.push(TurnTrace {
                        turn: trace_turn,
                        prompt_tokens: ptok,
                        completion_tokens: ctok,
                        cache_n,
                        tok_per_sec: tps,
                        wall_s: wall,
                        reasoning_preview: reasoning_txt,
                        tools: tool_traces,
                    });
                }
                tr.elapsed_s = t0.elapsed().as_secs_f64();
                tr.diff = diff_now;
                if let Some(ref fin) = finished {
                    tr.status = "completed".into();
                    tr.live_phase = "Verified & Fixed".into();
                    tr.live_stream.clear();
                    tr.final_result = fin.clone();
                }
            }
        }

        if let Some(err) = abort_err {
            let mut map = state.tasks.lock().unwrap();
            if let Some(tr) = map.get_mut(&task_id) {
                tr.status = "failed".into();
                tr.live_phase = "Aborted".into();
                tr.error = err.clone();
                if tr.final_result.is_empty() {
                    tr.final_result = err;
                }
                tr.elapsed_s = t0.elapsed().as_secs_f64();
            }
            return;
        }
        if finished.is_some() {
            return;
        }
    }

    let (final_status, final_phase, final_err, final_res) =
        finalize_max_turns_outcome(&edited_files, verified_pass, &last_ok_summary, "");
    let mut map = state.tasks.lock().unwrap();
    if let Some(tr) = map.get_mut(&task_id) {
        tr.status = final_status.into();
        tr.live_phase = final_phase.into();
        tr.error = final_err;
        if tr.final_result.is_empty() || final_status == "completed" {
            tr.final_result = final_res;
        }
        tr.live_stream.clear();
        tr.elapsed_s = t0.elapsed().as_secs_f64();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::RepoStore;

    #[test]
    fn f16_prevalidates_invented_paths_and_symbols() {
        let dir = std::env::temp_dir().join(format!("kraits_f16_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src/flask")).unwrap();
        fs::write(dir.join("src/flask/helpers.py"), "def stream_with_context(x):\n    return x\n").unwrap();
        fs::write(dir.join("src/flask/app.py"), "class Flask:\n    pass\n").unwrap();
        let mut store = RepoStore::open(&dir);
        let _ = store.sync();

        assert!(prevalidate_tool_call(&store, &dir, "ast_symbol", &Json::obj(vec![
            ("path", Json::Str("src/flask/helpers.py".into())),
            ("symbol", Json::Str("stream_with_context".into())),
        ])).is_ok());

        for bad in ["src/flsksk/app.py", "src\\flask\\helps.py", "src/flask.helpers.py", "src/flask_helpers.py", "src/flask/globals", "src/flask/ctx/ctx", "src/flask/helpers.py', path_absolute_offset>"] {
            let err = prevalidate_tool_call(&store, &dir, "read_file", &Json::obj(vec![("path", Json::Str(bad.into()))])).unwrap_err();
            assert!(err.1.contains("Nearest real paths"), "missing hint for {}: {}", bad, err.1);
        }

        assert!(prevalidate_tool_call(&store, &dir, "read_file", &Json::obj(vec![
            ("path", Json::Str("src/flask/helpers.py".into())),
            ("start_line", Json::Num(101.0)),
            ("end_line", Json::Num(100.0)),
        ])).is_err());

        assert!(prevalidate_tool_call(&store, &dir, "execute_command", &Json::obj(vec![
            ("command", Json::Str("Get-SelectSelect_base_bash__line_iterator__StopIteration".into())),
        ])).is_err());
        assert!(prevalidate_tool_call(&store, &dir, "execute_command", &Json::obj(vec![
            ("command", Json::Str("python - -m pytest tests/test_helpers.py -q".into())),
        ])).is_err());
        assert!(prevalidate_tool_call(&store, &dir, "execute_command", &Json::obj(vec![
            ("command", Json::Str("python -m pytest tests/test_helpers.py -q".into())),
        ])).is_ok());

        let sym_err = prevalidate_tool_call(&store, &dir, "ast_symbol", &Json::obj(vec![
            ("path", Json::Str("src/flask/helpers.py".into())),
            ("symbol", Json::Str("_stream_with_context_generator".into())),
        ])).unwrap_err();
        assert!(sym_err.1.contains("not found in AST index"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn f1_recovers_truncated_attempt_completion_prose() {
        let partial = r#"{"result":"\"\"\",\nExplanation: **How stream_with_context works**\n\nIt wraps the generator and pushes the RequestContext before yielding each chunk, then cleans up safely when exhausted or closed. Extra incomplete tail"#;
        let out = extract_partial_completion_text(partial).unwrap();
        assert!(out.contains("wraps the generator"));
        assert!(!out.starts_with('"'));
        assert!(!out.contains("incomplete tail"));
    }

    #[test]
    fn f15_rejects_gibberish_and_cutoff_summaries() {
        assert!(!is_clean_english_summary("FLAKASHVENV is not defined and FLFKSHVFEH fails."));
        assert!(!is_clean_english_summary("Updated config in src/flask/config.py>:"));
        assert!(!is_clean_english_summary("Fixed trim_namespace condition (now src/flask/config."));
        assert!(!is_clean_english_summary("Fixed helpers so it will check the the the FLASK_DEBUG flag."));
        assert!(!is_clean_english_summary("Updated helpers using 12-char short-c."));
        assert!(!is_clean_english_summary("// Explanation of stream_with_context\nreturn stream_with_context;"));
        assert!(!is_clean_english_summary("Fixed FLASK_DEBUG handling (e \"FLASK_DEBUG\" environment values (e \"FLASK_DEBUG\" environment values."));
        assert!(!is_clean_english_summary("Updated helpers</result></result></invoke>."));
        assert!(!is_clean_english_summary("Updated helpers where count < 5 and fixed branch."));
        assert!(!is_clean_english_summary("[]"));
        assert!(!is_clean_english_summary("1234567890 1234567890."));
        assert!(!is_clean_english_summary("Fixed get_debug_flag in src/flask/helpers.py so that the FLa... it now works correctly."));

        let mut files = BTreeMap::new();
        files.insert("src/flask/helpers.py".into(), AstFile { path: "src/flask/helpers.py".into(), lang: "python".into(), lines: 10, hash: 0, imports: vec![], nodes: vec![] });
        files.insert("tests/test_helpers.py".into(), AstFile { path: "tests/test_helpers.py".into(), lang: "python".into(), lines: 10, hash: 0, imports: vec![], nodes: vec![] });
        assert!(!is_clean_english_summary_with_index("Fixed get_debug_flag in src/flaskl/helpers.py and verified 35 tests pass.", Some(&files)));
        assert!(is_clean_english_summary_with_index("Updated src/flask/helpers.py and verified tests/test_helpers.py::test_get_debug_flag passes.", Some(&files)));
        assert!(is_clean_english_summary("Updated src/flask/helpers.py and verified 35 tests pass."));
    }

    #[test]
    fn f19_aborts_no_tool_prose_streams_early() {
        assert!(!tool_stream_should_stop(true, false, "short prose", 20));
        assert!(tool_stream_should_stop(true, false, "<system_analysis>\nYou've have is to a tool", 64));
        assert!(!tool_stream_should_stop(true, false, "{\"path\": \"src/flask/helpers.py\"}", 64));
        assert!(tool_stream_should_stop(true, false, "{\"noisy\": true, ...}", 256));
        assert!(!tool_stream_should_stop(true, true, "anything", 256));
        assert!(!tool_stream_should_stop(false, false, "long read-only prose answer", 256));
    }

    #[test]
    fn f6_detects_overlapping_ranges() {
        let seen = vec![(1, 140)];
        assert!(overlaps_seen_range(&seen, 2, 141).is_some());
        assert!(overlaps_seen_range(&seen, 141, 240).is_none());
    }

    #[test]
    fn f20_infers_verify_cmd_and_tracks_failure_summary() {
        let mut files = BTreeMap::new();
        files.insert("src/flask/helpers.py".into(), AstFile { path: "src/flask/helpers.py".into(), lang: "python".into(), lines: 10, hash: 0, imports: vec![], nodes: vec![] });
        files.insert("tests/test_helpers.py".into(), AstFile { path: "tests/test_helpers.py".into(), lang: "python".into(), lines: 10, hash: 0, imports: vec![], nodes: vec![] });
        assert_eq!(
            extract_verify_command("Fix bug and verify with `python -m pytest tests/test_helpers.py -q`.", "", "src/flask/helpers.py", &files),
            "python -m pytest tests/test_helpers.py -q"
        );
        assert_eq!(
            extract_verify_command("Fix helpers.py.", "", "src/flask/helpers.py", &files),
            "python -m pytest tests/test_helpers.py -q"
        );
        assert_eq!(
            extract_verify_command("Fix helpers.py.", "python -m pytest tests/test_config.py -q", "src/flask/helpers.py", &files),
            "python -m pytest tests/test_config.py -q"
        );
        let mut edited = BTreeSet::new();
        edited.insert("src/flask/helpers.py".to_string());
        assert_eq!(
            deterministic_failure_summary(&edited, "3 Python syntax reverts"),
            "Aborted (3 Python syntax reverts) after editing src/flask/helpers.py."
        );
    }

    #[test]
    fn f21_refuses_shrinking_or_zero_symbol_write_to_file() {
        let dir = std::env::temp_dir().join(format!("kraits_f21_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src/flask")).unwrap();
        let original = "class Flask:\n    def route(self, rule):\n        return rule\n\n    def run(self):\n        return True\n";
        let app_path = dir.join("src/flask/app.py");
        fs::write(&app_path, original).unwrap();
        let mut store = RepoStore::open(&dir);
        let _ = store.sync();

        let err = apply_guarded_write_to_file(&dir, &mut store, "src/flask/app.py", "pass\n").unwrap_err();
        assert!(err.0.contains("write refused"));
        assert!(err.1.contains("Refused write_to_file"));
        assert_eq!(fs::read_to_string(&app_path).unwrap(), original);

        let ok_new = apply_guarded_write_to_file(&dir, &mut store, "src/flask/new_mod.py", "def ok():\n    return 1\n");
        assert!(ok_new.unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn f22_fix_prompts_never_classify_readonly() {
        assert!(!is_readonly_question("Inspect src/flask/helpers.py, fix the bug with `replace_in_file`, and verify with pytest."));
        assert!(!is_readonly_question("Call write_to_file on tests/test_helpers.py."));
        assert!(is_readonly_question("Explain how stream_with_context in src/flask/helpers.py works."));
    }

    #[test]
    fn f24_max_turns_with_green_verify_completes() {
        let mut edited = BTreeSet::new();
        edited.insert("src/flask/helpers.py".to_string());
        let (st, phase, err, res) = finalize_max_turns_outcome(
            &edited,
            true,
            "`python -m pytest tests/test_helpers.py -q` -> 35 passed in 0.28s",
            "",
        );
        assert_eq!(st, "completed");
        assert_eq!(phase, "Verified & Fixed");
        assert!(err.is_empty());
        assert!(res.contains("src/flask/helpers.py") && res.contains("35 passed"));

        let (st_f, _, err_f, res_f) = finalize_max_turns_outcome(&edited, false, "", "");
        assert_eq!(st_f, "failed");
        assert_eq!(err_f, "Exceeded max_turns (9)");
        assert!(res_f.contains("Aborted"));
    }

    #[test]
    fn loop_guard_aborts_exploration_and_oscillation_loops() {
        assert_eq!(should_abort_loop_guard(false, true, false, 4, 0, 0), None);
        assert!(should_abort_loop_guard(false, true, false, 5, 0, 0).is_some());
        assert_eq!(should_abort_loop_guard(true, true, false, 6, 0, 0), None);
        assert!(should_abort_loop_guard(false, false, false, 0, 4, 0).is_some());
        assert_eq!(should_abort_loop_guard(false, false, true, 0, 4, 5), None);
        assert!(should_abort_loop_guard(false, false, false, 0, 2, 5).is_some());
    }
}
