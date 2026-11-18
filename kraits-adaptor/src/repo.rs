//! Incremental repo AST index and episodic learning store (`<repo>/.kraits/`).

use crate::ast::{fnv1a, parse_ast, AstFile, AstNode};
use crate::json::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct Lesson {
    pub kind: String,
    pub signature: String,
    pub detail: String,
    pub files: Vec<String>,
    pub hits: i64,
    pub ts: i64,
}

impl Lesson {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("kind", Json::Str(self.kind.clone())),
            ("signature", Json::Str(self.signature.clone())),
            ("detail", Json::Str(self.detail.clone())),
            ("files", Json::Arr(self.files.iter().cloned().map(Json::Str).collect())),
            ("hits", Json::Num(self.hits as f64)),
            ("ts", Json::Num(self.ts as f64)),
        ])
    }

    pub fn from_json(v: &Json) -> Option<Self> {
        Some(Lesson {
            kind: v.get("kind")?.as_str()?.to_string(),
            signature: v.get("signature")?.as_str()?.to_string(),
            detail: v.get("detail")?.as_str()?.to_string(),
            files: v
                .get("files")
                .and_then(|x| x.as_arr())
                .map(|a| a.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
                .unwrap_or_default(),
            hits: v.get("hits").and_then(|x| x.as_i64()).unwrap_or(1),
            ts: v.get("ts").and_then(|x| x.as_i64()).unwrap_or(0),
        })
    }
}

pub struct RepoStore {
    pub root: PathBuf,
    pub files: BTreeMap<String, AstFile>,
    pub manifests: Vec<String>,
    pub toolchain: Vec<String>,
    pub lessons: Vec<Lesson>,
    pub hot_files: BTreeMap<String, i64>,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".kraits" | "target" | "node_modules" | "__pycache__" | ".next" | "dist" | "build" | ".vscode"
    )
}

fn walk_repo(root: &Path, cur: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(cur) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if !should_skip_dir(&name) {
                walk_repo(root, &path, out);
            }
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn rel_str(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

impl RepoStore {
    pub fn open(root: &Path) -> Self {
        let mut store = RepoStore {
            root: root.to_path_buf(),
            files: BTreeMap::new(),
            manifests: Vec::new(),
            toolchain: Vec::new(),
            lessons: Vec::new(),
            hot_files: BTreeMap::new(),
        };
        store.load_state();
        store.sync();
        store
    }

    fn kraits_dir(&self) -> PathBuf {
        self.root.join(".kraits")
    }

    fn index_path(&self) -> PathBuf {
        self.kraits_dir().join("ast_index.json")
    }

    fn memory_path(&self) -> PathBuf {
        self.kraits_dir().join("memory.jsonl")
    }

    fn load_state(&mut self) {
        let cur_fp = repo_fingerprint(&self.root);
        let canon_root = canonical_repo_path(&self.root);
        let mut fp_mismatch = false;

        if let Ok(raw) = fs::read_to_string(self.index_path()) {
            if let Ok(v) = Json::parse(&raw) {
                if let Some(stored_fp) = v.get("repo_fingerprint").and_then(|x| x.as_str()) {
                    if !stored_fp.is_empty() && stored_fp != cur_fp {
                        fp_mismatch = true;
                        eprintln!(
                            "[kraits-adaptor] REFUSED stale ast_index.json (fp={} != expected={}); re-indexing clean.",
                            stored_fp, cur_fp
                        );
                    }
                }
                if !fp_mismatch {
                    if let Some(arr) = v.get("files").and_then(|x| x.as_arr()) {
                        for item in arr {
                            if let Some(af) = AstFile::from_json(item) {
                                self.files.insert(af.path.clone(), af);
                            }
                        }
                    }
                    if let Some(hf) = v.get("hot_files").and_then(|x| x.as_obj()) {
                        for (k, n) in hf {
                            if let Some(val) = n.as_i64() {
                                if let Some(clean_k) = self.validate_hot_path_with_root(k, &canon_root) {
                                    self.hot_files.insert(clean_k, val);
                                }
                            }
                        }
                    }
                }
            }
        }
        if fp_mismatch {
            return;
        }
        if let Ok(raw) = fs::read_to_string(self.memory_path()) {
            for line in raw.lines() {
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                if let Ok(v) = Json::parse(t) {
                    if let Some(l_fp) = v.get("repo_fingerprint").and_then(|x| x.as_str()) {
                        if !l_fp.is_empty() && l_fp != cur_fp {
                            eprintln!("[kraits-adaptor] REFUSED mismatched memory.jsonl (fp={}); clearing.", l_fp);
                            self.lessons.clear();
                            break;
                        }
                    }
                    if let Some(mut l) = Lesson::from_json(&v) {
                        if Self::is_clean_lesson_text(&l.signature) && Self::is_clean_lesson_text(&l.detail) {
                            l.files = l
                                .files
                                .iter()
                                .filter_map(|f| self.validate_hot_path_with_root(f, &canon_root))
                                .collect();
                            self.lessons.push(l);
                        }
                    }
                }
            }
        }
    }

    fn is_clean_lesson_text(s: &str) -> bool {
        let low = s.to_ascii_lowercase();
        !low.contains("</tool_call>")
            && !low.contains("</user>")
            && !low.contains("<function=")
            && !low.contains("validation error:")
            && !low.contains("minio-frontend")
            && !low.contains("devinfra-frontend")
            && !low.contains("[\"file_\"]")
    }

    fn validate_hot_path_with_root(&self, raw: &str, canon_root: &str) -> Option<String> {
        let t = raw.trim();
        if t.is_empty() || t.len() > 200 {
            return None;
        }
        if t.chars().any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '[' | ']' | '"' | '\'' | ':' | '=' | ',')) {
            return None;
        }
        if t.contains("..") {
            return None;
        }
        let norm = t.replace('\\', "/").trim_start_matches("./").to_string();
        if norm.starts_with('/') {
            return None;
        }
        if self.files.contains_key(&norm) {
            return Some(norm);
        }
        let full = self.root.join(&norm);
        if full.is_file() {
            if let Ok(c) = fs::canonicalize(&full) {
                let cs = c.to_string_lossy().into_owned();
                let cs_clean = cs.strip_prefix(r"\\?\").unwrap_or(&cs).to_string();
                let sep_ok = cs_clean.eq_ignore_ascii_case(canon_root)
                    || cs_clean
                        .to_ascii_lowercase()
                        .starts_with(&format!("{}{}", canon_root.to_ascii_lowercase(), std::path::MAIN_SEPARATOR));
                if sep_ok {
                    return Some(norm);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn validate_hot_path(&self, raw: &str) -> Option<String> {
        let canon_root = canonical_repo_path(&self.root);
        self.validate_hot_path_with_root(raw, &canon_root)
    }

    pub fn purge_unindexed(&mut self) -> (usize, usize, usize, usize) {
        let canon_root = canonical_repo_path(&self.root);
        let hf_before = self.hot_files.len();
        let kept_hf: BTreeMap<String, i64> = self
            .hot_files
            .iter()
            .filter_map(|(k, v)| self.validate_hot_path_with_root(k, &canon_root).map(|ck| (ck, *v)))
            .collect();
        self.hot_files = kept_hf;
        let hf_after = self.hot_files.len();

        let les_before = self.lessons.len();
        let mut kept_les = Vec::new();
        for mut l in std::mem::take(&mut self.lessons) {
            if !Self::is_clean_lesson_text(&l.signature) || !Self::is_clean_lesson_text(&l.detail) {
                continue;
            }
            l.files = l
                .files
                .iter()
                .filter_map(|f| self.validate_hot_path_with_root(f, &canon_root))
                .collect();
            kept_les.push(l);
        }
        self.lessons = kept_les;
        let les_after = self.lessons.len();
        self.save_lessons();
        self.save_index();
        (hf_before, hf_after, les_before, les_after)
    }

    pub fn sync(&mut self) -> (usize, usize) {
        let mut paths = Vec::new();
        walk_repo(&self.root, &self.root, &mut paths);
        let mut seen = BTreeSet::new();
        let mut updated = 0usize;
        self.manifests.clear();
        self.toolchain.clear();

        for p in paths {
            let rel = rel_str(&self.root, &p);
            let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(
                fname,
                "Cargo.toml" | "package.json" | "tsconfig.json" | "CMakeLists.txt" | "Makefile" | "pyproject.toml"
            ) {
                self.manifests.push(rel.clone());
                self.inspect_manifest(&rel, &p);
            }

            let Some(meta) = fs::metadata(&p).ok() else { continue };
            if meta.len() > 512 * 1024 {
                continue;
            }
            let Ok(bytes) = fs::read(&p) else { continue };
            let hash = fnv1a(&bytes);
            if crate::ast::Lang::from_ext(&rel).is_none() {
                continue;
            }
            seen.insert(rel.clone());
            if self.files.get(&rel).map_or(false, |f| f.hash == hash) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            if let Some(ast) = parse_ast(&rel, &text) {
                self.files.insert(rel, ast);
                updated += 1;
            }
        }

        let before = self.files.len();
        self.files.retain(|k, _| seen.contains(k));
        let removed = before.saturating_sub(self.files.len());
        let (_, hf_after, _, _) = {
            let canon_root = canonical_repo_path(&self.root);
            let old_hf = self.hot_files.len();
            self.hot_files.retain(|k, _| self.files.contains_key(k) || self.root.join(k).is_file() && canonical_repo_path(&self.root.join(k)).starts_with(&canon_root));
            (old_hf, self.hot_files.len(), 0usize, 0usize)
        };
        let _ = hf_after;
        if updated > 0 || removed > 0 || !self.index_path().exists() {
            self.save_index();
        }
        (updated, removed)
    }

    fn inspect_manifest(&mut self, rel: &str, p: &Path) {
        let Ok(raw) = fs::read_to_string(p) else { return };
        if rel.ends_with("Cargo.toml") {
            let dir = Path::new(rel).parent().and_then(|x| x.to_str()).unwrap_or("");
            let prefix = if dir.is_empty() { "cargo test".into() } else { format!("cd {} && cargo test", dir) };
            self.toolchain.push(format!("Rust workspace ({}) -> verify with `{}`", rel, prefix));
        } else if rel.ends_with("package.json") {
            if let Ok(v) = Json::parse(&raw) {
                let mut scripts = Vec::new();
                if let Some(sc) = v.get("scripts").and_then(|x| x.as_obj()) {
                    for (k, val) in sc {
                        scripts.push(format!("{}={}", k, val.as_str().unwrap_or("")));
                    }
                }
                self.toolchain.push(format!("Node/TS ({}) scripts: [{}]", rel, scripts.join(", ")));
            }
        } else if rel.ends_with("tsconfig.json") {
            self.toolchain.push(format!("TypeScript config ({}) -> typecheck with `tsc --noEmit`", rel));
        } else if rel.ends_with("pyproject.toml") {
            self.toolchain.push(format!("Python project ({}) -> verify with `python -m pytest`", rel));
        } else if rel.ends_with("CMakeLists.txt") || rel.ends_with("Makefile") {
            self.toolchain.push(format!("Build manifest ({})", rel));
        }
    }

    pub fn save_index(&self) {
        let _ = fs::create_dir_all(self.kraits_dir());
        let canon_root = canonical_repo_path(&self.root);
        let files_json = Json::Arr(self.files.values().map(AstFile::to_json).collect());
        let mut hf_map = BTreeMap::new();
        for (k, v) in &self.hot_files {
            if let Some(clean_k) = self.validate_hot_path_with_root(k, &canon_root) {
                hf_map.insert(clean_k, Json::Num(*v as f64));
            }
        }
        let doc = Json::obj(vec![
            ("repo_fingerprint", Json::Str(repo_fingerprint(&self.root))),
            ("files", files_json),
            ("manifests", Json::Arr(self.manifests.iter().cloned().map(Json::Str).collect())),
            ("hot_files", Json::Obj(hf_map)),
        ]);
        let _ = fs::write(self.index_path(), doc.encode());
    }

    fn save_lessons(&self) {
        let _ = fs::create_dir_all(self.kraits_dir());
        let mut out = String::new();
        let meta = Json::obj(vec![("repo_fingerprint", Json::Str(repo_fingerprint(&self.root)))]);
        out.push_str(&meta.encode());
        out.push('\n');
        for l in &self.lessons {
            out.push_str(&l.to_json().encode());
            out.push('\n');
        }
        let _ = fs::write(self.memory_path(), out);
    }

    pub fn record_lesson(&mut self, kind: &str, signature: &str, detail: &str, files: Vec<String>) {
        if !Self::is_clean_lesson_text(signature) || !Self::is_clean_lesson_text(detail) {
            return;
        }
        let norm_sig: String = signature.split_whitespace().collect::<Vec<_>>().join(" ");
        let norm_sig = if norm_sig.len() > 160 { format!("{}...", &norm_sig[..157]) } else { norm_sig };
        let norm_det: String = detail.split_whitespace().collect::<Vec<_>>().join(" ");
        let norm_det = if norm_det.len() > 360 { format!("{}...", &norm_det[..357]) } else { norm_det };
        let ts = now_secs();
        let canon_root = canonical_repo_path(&self.root);
        let valid_files: Vec<String> = files
            .into_iter()
            .filter_map(|f| self.validate_hot_path_with_root(&f, &canon_root))
            .collect();

        for f in &valid_files {
            *self.hot_files.entry(f.clone()).or_insert(0) += 1;
        }

        if let Some(existing) = self
            .lessons
            .iter_mut()
            .find(|l| l.kind == kind && l.signature == norm_sig)
        {
            existing.hits += 1;
            existing.ts = ts;
            if !norm_det.is_empty() && norm_det.len() >= existing.detail.len() / 2 {
                existing.detail = norm_det;
            }
            for f in valid_files {
                if !existing.files.contains(&f) {
                    existing.files.push(f);
                }
            }
        } else {
            self.lessons.push(Lesson {
                kind: kind.to_string(),
                signature: norm_sig,
                detail: norm_det,
                files: valid_files,
                hits: 1,
                ts,
            });
            if self.lessons.len() > 250 {
                self.lessons.sort_by_key(|l| -(l.hits * 100000 + l.ts));
                self.lessons.truncate(200);
            }
        }
        self.save_lessons();
        self.save_index();
    }

    pub fn learn_from_messages(&mut self, messages: &[Json]) {
        let mut last_error: Option<(String, Vec<String>)> = None;
        let mut recent_edits: Vec<(String, String)> = Vec::new();
        let mut id_to_call: BTreeMap<String, (String, String)> = BTreeMap::new();
        let mut id_to_ok: BTreeMap<String, bool> = BTreeMap::new();
        let canon_root = canonical_repo_path(&self.root);

        for msg in messages {
            if msg.get("role").and_then(|x| x.as_str()) == Some("tool") {
                let tid = msg.get("tool_call_id").and_then(|x| x.as_str()).unwrap_or("");
                let content = msg.get("content").and_then(|x| x.as_str()).unwrap_or("");
                let ok = !tid.is_empty()
                    && !content.starts_with("ERROR:")
                    && !content.starts_with("VALIDATION ERROR:")
                    && !content.starts_with("REPEAT BLOCKED:");
                id_to_ok.insert(tid.to_string(), ok);
            }
        }

        for msg in messages {
            let role = msg.get("role").and_then(|x| x.as_str()).unwrap_or("");
            if role == "assistant" {
                if let Some(tcs) = msg.get("tool_calls").and_then(|x| x.as_arr()) {
                    for tc in tcs {
                        let id = tc.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let fn_obj = tc.get("function");
                        let name = fn_obj.and_then(|f| f.get("name")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let args_str = fn_obj.and_then(|f| f.get("arguments")).and_then(|x| x.as_str()).unwrap_or("");
                        if !id.is_empty() {
                            id_to_call.insert(id.clone(), (name.clone(), args_str.to_string()));
                        }
                        if matches!(name.as_str(), "write_to_file" | "replace_in_file") && id_to_ok.get(&id).copied().unwrap_or(false) {
                            if let Ok(a) = Json::parse(args_str) {
                                if let Some(path) = a.get("path").and_then(|x| x.as_str()) {
                                    if let Some(clean) = self.validate_hot_path_with_root(path, &canon_root) {
                                        let note = if name == "replace_in_file" {
                                            let new_t = a.get("new_text").and_then(|x| x.as_str()).unwrap_or("");
                                            format!("Patched `{}`: {}", clean, new_t.chars().take(180).collect::<String>())
                                        } else {
                                            format!("Wrote `{}`", clean)
                                        };
                                        recent_edits.push((clean, note));
                                    }
                                }
                            }
                        }
                    }
                }
            } else if role == "tool" {
                let tid = msg.get("tool_call_id").and_then(|x| x.as_str()).unwrap_or("");
                let content = msg.get("content").and_then(|x| x.as_str()).unwrap_or("");
                let Some((tool_name, args_str)) = id_to_call.get(tid) else { continue };
                if tool_name == "execute_command" {
                    let cmd = Json::parse(args_str)
                        .ok()
                        .and_then(|a| a.get("command").and_then(|x| x.as_str()).map(str::to_string))
                        .unwrap_or_default();
                    let failed = content.contains("EXIT_CODE=")
                        && !content.contains("EXIT_CODE=0")
                        || content.contains("TIMEOUT after");
                    let passed = content.contains("EXIT_CODE=0");
                    let touched = extract_mentioned_files(content, &self.files);

                    if failed {
                        if let Some(err_line) = first_diagnostic_line(content) {
                            last_error = Some((err_line.clone(), touched.clone()));
                            let files = if !touched.is_empty() {
                                touched
                            } else {
                                recent_edits.last().map(|(f, _)| vec![f.clone()]).unwrap_or_default()
                            };
                            self.record_lesson("error_pitfall", &err_line, &format!("Command `{}` hit: {}", cmd, err_line), files);
                        }
                    } else if passed {
                        let is_verify_cmd = !cmd.is_empty()
                            && cmd.len() <= 140
                            && (cmd.contains("pytest") || cmd.contains("cargo") || cmd.contains("clang") || cmd.contains("tsc") || cmd.contains("node ") || cmd.contains("python "));
                        if is_verify_cmd {
                            self.record_lesson("verified_cmd", &cmd, &format!("Passed cleanly (`EXIT_CODE=0`): `{}`", cmd), touched.clone());
                        }
                        if let Some((err_sig, err_files)) = last_error.take() {
                            if let Some((edit_file, edit_note)) = recent_edits.last() {
                                let mut files = err_files;
                                if !files.contains(edit_file) {
                                    files.push(edit_file.clone());
                                }
                                self.record_lesson(
                                    "error_fix",
                                    &err_sig,
                                    &format!("Fixed via {} -> verified by `{}`", edit_note, cmd),
                                    files,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn needs_repo_context(&self, query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return false;
        }
        let ql = q.to_ascii_lowercase();
        for pat in [".py", ".rs", ".js", ".ts", ".toml", ".json", ".md", "/", "::", "()"] {
            if ql.contains(pat) {
                return true;
            }
        }
        let terms = tokenize_query(q);
        const GREETINGS: &[&str] = &[
            "hello", "hi", "hey", "yo", "sup", "thanks", "thank", "morning",
            "evening", "afternoon", "goodnight", "bye", "ok", "okay", "cool",
        ];
        if !terms.is_empty() && terms.iter().all(|t| GREETINGS.contains(&t.as_str())) {
            return false;
        }
        if is_meta_capability_question(&ql, &terms) {
            return false;
        }
        const CODE_WORDS: &[&str] = &[
            "fix", "bug", "test", "tests", "pytest", "cargo", "build", "edit", "patch",
            "implement", "add", "create", "write", "refactor", "debug", "error", "fail",
            "failing", "traceback", "function", "method", "class", "file", "repo",
            "code", "diff", "endpoint", "route", "api", "run", "check", "verify",
            "symbol", "ast", "src", "inspect",
        ];
        if terms.iter().any(|t| CODE_WORDS.contains(&t.as_str())) {
            return true;
        }
        let rname = repo_display_name(&self.root).to_ascii_lowercase();
        if terms.iter().any(|t| t.len() >= 4 && !GREETINGS.contains(&t.as_str()) && rname.contains(t.as_str())) {
            return true;
        }
        for af in self.files.values() {
            let lp = af.path.to_ascii_lowercase();
            let in_src = lp.starts_with("src/") || lp.starts_with("tests/");
            if terms.iter().any(|t| {
                t.len() >= 5
                    && !GREETINGS.contains(&t.as_str())
                    && (lp.contains(t.as_str()) || (in_src && ast_contains_term(&af.nodes, t)))
            }) {
                return true;
            }
        }
        false
    }

    pub fn build_repo_context(&self, query: &str, budget_chars: usize) -> String {
        let terms = tokenize_query(query);
        let mut out = String::from("[Kraits Repo Adaptor — AST Tree & Learned Repo Memory]\n");
        if !self.toolchain.is_empty() {
            out.push_str("# Detected Toolchain & Build Commands\n");
            for t in self.toolchain.iter().take(6) {
                out.push_str("- ");
                out.push_str(t);
                out.push('\n');
            }
        }

        // Rank learned lessons
        if !self.lessons.is_empty() {
            let mut scored_lessons: Vec<(i64, &Lesson)> = self
                .lessons
                .iter()
                .map(|l| {
                    let mut sc = l.hits * 10;
                    if l.kind == "error_fix" {
                        sc += 25;
                    } else if l.kind == "verified_cmd" {
                        sc += 15;
                    }
                    let hay = format!("{} {} {}", l.signature, l.detail, l.files.join(" ")).to_ascii_lowercase();
                    for t in &terms {
                        if hay.contains(t.as_str()) {
                            sc += 30;
                        }
                    }
                    (sc, l)
                })
                .collect();
            scored_lessons.sort_by_key(|(s, l)| (-*s, -l.ts));
            out.push_str("# Learned Repo Lessons (from prior runs in this repo)\n");
            for (_, l) in scored_lessons.into_iter().take(6) {
                out.push_str(&format!("- [{} x{}] {}: {}\n", l.kind, l.hits, l.signature, l.detail));
            }
        }

        // Rank AST files
        if self.files.is_empty() {
            out.push_str("# AST Tree: (repo has no source files yet — create new files cleanly)\n");
        } else {
            let wants_tests = terms.iter().any(|t| t == "test" || t == "tests" || t == "pytest");
            let mut ranked: Vec<(i64, &AstFile)> = self
                .files
                .values()
                .map(|af| {
                    let mut sc = *self.hot_files.get(&af.path).unwrap_or(&0) * 8;
                    let low_path = af.path.to_ascii_lowercase();
                    if low_path.starts_with("src/") {
                        sc += 18;
                    }
                    if wants_tests && (low_path.ends_with("conftest.py") || low_path == "tests/test_basic.py") {
                        sc += 14;
                    }
                    if low_path.starts_with("examples/") || low_path.starts_with("docs/") {
                        sc -= 8;
                    }
                    if af.nodes.is_empty() && af.lines < 15 {
                        sc -= 14;
                    }
                    for t in &terms {
                        if low_path.contains(t.as_str()) {
                            sc += 40;
                        }
                        if ast_contains_term(&af.nodes, t) {
                            sc += 25;
                        }
                    }
                    (sc, af)
                })
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.path.cmp(&b.1.path)));

            out.push_str("# AST Code Tree\n");
            let mut omitted = 0usize;
            for (_, af) in ranked {
                let skel = af.render_skeleton();
                if out.len() + skel.len() > budget_chars {
                    let remain = budget_chars.saturating_sub(out.len());
                    if remain > 280 && skel.len() > remain {
                        let clipped = focus_skeleton(&skel, &terms, 16);
                        if clipped.len() + 45 < remain {
                            out.push_str(&clipped);
                            out.push_str("\n  ... (focused AST slice; use ast_symbol for full method)\n");
                            continue;
                        }
                    }
                    omitted += 1;
                    continue;
                }
                out.push_str(&skel);
            }
            if omitted > 0 {
                out.push_str(&format!("... (+{} other indexed files omitted for budget)\n", omitted));
            }
        }
        // Automatically attach the focused AST method/function body for any `<file>:<line>` diagnostic in query
        for (rel, line_no) in extract_file_line_refs(query, &self.files).into_iter().take(2) {
            if let Some(snip) = self.extract_ast_at_line(&rel, line_no) {
                if out.len() + snip.len() + 32 <= budget_chars + 1200 {
                    out.push_str("# Focused AST Node At Diagnostic\n");
                    out.push_str(&snip);
                    out.push('\n');
                }
            }
        }
        out
    }

    pub fn extract_ast_symbol(&self, rel_path: &str, symbol: &str) -> Option<String> {
        let af = self.files.get(rel_path)?;
        let bare = symbol.rsplit('.').next().unwrap_or(symbol);
        let stripped = bare.trim_start_matches('_');
        let node = find_node_by_name(&af.nodes, bare)
            .or_else(|| find_node_by_name(&af.nodes, symbol))
            .or_else(|| (!stripped.is_empty()).then(|| find_node_by_name(&af.nodes, stripped)).flatten())
            .or_else(|| find_node_ci(&af.nodes, bare))
            .or_else(|| (!stripped.is_empty()).then(|| find_node_ci(&af.nodes, stripped)).flatten())?;
        self.render_node_body(rel_path, node)
    }

    pub fn extract_ast_at_line(&self, rel_path: &str, line: usize) -> Option<String> {
        let af = self.files.get(rel_path)?;
        let node = find_deepest_at_line(&af.nodes, line)?;
        self.render_node_body(rel_path, node)
    }

    fn render_node_body(&self, rel_path: &str, node: &AstNode) -> Option<String> {
        let full = self.root.join(rel_path);
        let raw = fs::read_to_string(full).ok()?;
        let lines: Vec<&str> = raw.lines().collect();
        let s = node.start_line.saturating_sub(1);
        let e = node.end_line.min(lines.len());
        let body = if e.saturating_sub(s) > 140 {
            let head_end = (s + 25).min(e);
            let tail_start = e.saturating_sub(25).max(head_end);
            let mut buf = lines[s..head_end].join("\n");
            buf.push_str(&format!("\n... ({} middle lines omitted; inspect child symbol or read lines) ...", tail_start - head_end));
            for ch in &node.children {
                buf.push_str(&format!(
                    "\n  {} {} [L{}-L{}]",
                    ch.kind, ch.signature, ch.start_line, ch.end_line
                ));
            }
            if tail_start < e {
                buf.push('\n');
                buf.push_str(&lines[tail_start..e].join("\n"));
            }
            buf
        } else {
            lines[s..e].join("\n")
        };
        Some(format!(
            "{}:{}-{} ({} {}):\n{}",
            rel_path,
            node.start_line,
            node.end_line,
            node.kind,
            node.signature,
            body
        ))
    }
}

fn find_deepest_at_line<'a>(nodes: &'a [AstNode], line: usize) -> Option<&'a AstNode> {
    for n in nodes {
        if line >= n.start_line && line <= n.end_line {
            if let Some(ch) = find_deepest_at_line(&n.children, line) {
                return Some(ch);
            }
            return Some(n);
        }
    }
    None
}

fn extract_file_line_refs(query: &str, files: &BTreeMap<String, AstFile>) -> Vec<(String, usize)> {
    let norm = query.replace('\\', "/");
    let mut out = Vec::new();
    for path in files.keys() {
        let base = Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
        for needle in [path.as_str(), base] {
            if needle.is_empty() {
                continue;
            }
            if let Some(pos) = norm.find(&format!("{}:", needle)) {
                let after = &norm[pos + needle.len() + 1..];
                let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(ln) = digits.parse::<usize>() {
                    out.push((path.clone(), ln));
                    break;
                }
            }
        }
    }
    out
}

fn find_node_by_name<'a>(nodes: &'a [AstNode], sym: &str) -> Option<&'a AstNode> {
    for n in nodes {
        if n.name == sym {
            return Some(n);
        }
        if let Some(ch) = find_node_by_name(&n.children, sym) {
            return Some(ch);
        }
    }
    None
}

fn find_node_ci<'a>(nodes: &'a [AstNode], sym: &str) -> Option<&'a AstNode> {
    for n in nodes {
        if n.name.eq_ignore_ascii_case(sym) {
            return Some(n);
        }
        if let Some(ch) = find_node_ci(&n.children, sym) {
            return Some(ch);
        }
    }
    None
}

fn ast_contains_term(nodes: &[AstNode], term: &str) -> bool {
    for n in nodes {
        if n.name.to_ascii_lowercase().contains(term) || n.signature.to_ascii_lowercase().contains(term) {
            return true;
        }
        if ast_contains_term(&n.children, term) {
            return true;
        }
    }
    false
}

fn focus_skeleton(skel: &str, terms: &[String], max_lines: usize) -> String {
    let lines: Vec<&str> = skel.lines().collect();
    if lines.len() <= max_lines {
        return skel.to_string();
    }
    let mut picked = vec![false; lines.len()];
    for i in 0..lines.len().min(3) {
        picked[i] = true;
    }
    for (idx, line) in lines.iter().enumerate() {
        let low = line.to_ascii_lowercase();
        let is_container = line.starts_with("  class ") || line.starts_with("  struct ") || line.starts_with("  impl ");
        let matches = terms.iter().any(|t| low.contains(t.as_str()));
        if is_container || matches {
            picked[idx] = true;
        }
    }
    for i in lines.len().saturating_sub(2)..lines.len() {
        picked[i] = true;
    }
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if picked[idx] {
            out.push(*line);
            if out.len() >= max_lines {
                break;
            }
        }
    }
    out.join("\n")
}

fn tokenize_query(q: &str) -> Vec<String> {
    q.split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.len() >= 3)
        .take(24)
        .collect()
}

fn extract_mentioned_files(text: &str, files: &BTreeMap<String, AstFile>) -> Vec<String> {
    let norm = text.replace('\\', "/");
    let mut hits = Vec::new();
    for path in files.keys() {
        let base = Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
        if norm.contains(path) || (!base.is_empty() && norm.contains(base)) {
            hits.push(path.clone());
        }
    }
    hits
}

fn first_diagnostic_line(output: &str) -> Option<String> {
    for line in output.lines() {
        let t = line.trim();
        let low = t.to_ascii_lowercase();
        if low.contains("error:")
            || low.contains("error[e")
            || low.contains("assertionerror")
            || low.contains("typeerror")
            || low.contains("attributeerror")
            || t.starts_with("E   ")
            || low.contains("timeout after")
            || low.contains("assertion")
            || low.contains("segfault")
            || low.contains("exception")
            || low.contains("failed")
        {
            return Some(t.to_string());
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct RepoLockInfo {
    pub repo_path: String,
    pub repo_name: String,
    pub fingerprint: String,
    pub locked_at: i64,
}

impl RepoLockInfo {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("repo_path", Json::Str(self.repo_path.clone())),
            ("repo_name", Json::Str(self.repo_name.clone())),
            ("fingerprint", Json::Str(self.fingerprint.clone())),
            ("locked_at", Json::Num(self.locked_at as f64)),
        ])
    }

    pub fn from_json(v: &Json) -> Option<Self> {
        Some(RepoLockInfo {
            repo_path: v.get("repo_path")?.as_str()?.to_string(),
            repo_name: v.get("repo_name")?.as_str()?.to_string(),
            fingerprint: v.get("fingerprint")?.as_str()?.to_string(),
            locked_at: v.get("locked_at").and_then(|n| n.as_i64()).unwrap_or(0),
        })
    }
}

pub fn canonical_repo_path(root: &Path) -> String {
    let p = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let s = p.to_string_lossy().into_owned();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn git_stdout(root: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub fn repo_display_name(root: &Path) -> String {
    if let Some(url) = git_stdout(root, &["config", "--get", "remote.origin.url"]) {
        let trimmed = url.trim_end_matches(".git").trim_end_matches('/');
        let parts: Vec<&str> = trimmed.split(['/', ':']).filter(|x| !x.is_empty()).collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        }
    }
    Path::new(&canonical_repo_path(root))
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("repo")
        .to_string()
}

pub fn repo_fingerprint(root: &Path) -> String {
    let cpath = canonical_repo_path(root);
    let git_root_commit = git_stdout(root, &["rev-list", "--max-parents=0", "HEAD"]).unwrap_or_default();
    let origin = git_stdout(root, &["config", "--get", "remote.origin.url"]).unwrap_or_default();
    let seed = if !git_root_commit.is_empty() || !origin.is_empty() {
        format!("{}|{}|{}", cpath, git_root_commit, origin)
    } else {
        cpath
    };
    format!("{:016x}", fnv1a(seed.as_bytes()))
}

pub fn default_adaptor_lock_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let mut s = exe.as_os_str().to_os_string();
        s.push(".repo.lock");
        let p = PathBuf::from(s);
        if p.exists() {
            return p;
        }
        if let Some(parent) = p.parent() {
            if fs::metadata(parent).map(|m| !m.permissions().readonly()).unwrap_or(false) {
                return p;
            }
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".kraits").join("adaptor.repo.lock")
}

/// Enforces the 1-repo feature lock: binds this adaptor instance to a single repository on first run
/// (or validates against compile-time `KRAITS_LOCKED_REPO`) and rejects any attempt to reuse it on another repo.
pub fn enforce_single_repo_lock(lock_file: &Path, root: &Path) -> Result<RepoLockInfo, String> {
    if !root.is_dir() {
        return Err(format!("Repository directory does not exist: {}", root.display()));
    }
    let req_path = canonical_repo_path(root);
    let req_name = repo_display_name(root);
    let req_fp = repo_fingerprint(root);

    if let Some(baked) = option_env!("KRAITS_LOCKED_REPO") {
        let baked_norm = canonical_repo_path(Path::new(baked));
        if baked_norm != req_path && baked != req_name && baked != req_fp {
            return Err(format!(
                "Repo Lock Active (compile-time): this Kraits Repo Adaptor is locked to '{}'. Cannot bind to '{}' ({}).",
                baked, req_name, req_path
            ));
        }
    }

    if let Ok(raw) = fs::read_to_string(lock_file) {
        if let Ok(v) = Json::parse(&raw) {
            if let Some(locked) = RepoLockInfo::from_json(&v) {
                if locked.repo_path == req_path && locked.fingerprint == req_fp {
                    return Ok(locked);
                }
                return Err(format!(
                    "Repo Lock Active: this Kraits Repo Adaptor is permanently locked to 1 repository ('{}' at {}). Refusing to open '{}' ({}).",
                    locked.repo_name, locked.repo_path, req_name, req_path
                ));
            }
        }
        return Err(format!("Corrupt repo lock file {}; refusing to unlock.", lock_file.display()));
    }

    let info = RepoLockInfo {
        repo_path: req_path,
        repo_name: req_name,
        fingerprint: req_fp,
        locked_at: now_secs(),
    };
    if let Some(parent) = lock_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let encoded = info.to_json().encode();
    fs::write(lock_file, &encoded).map_err(|e| format!("Write repo lock {}: {}", lock_file.display(), e))?;
    let local_lock = root.join(".kraits").join("repo.lock.json");
    let _ = fs::create_dir_all(root.join(".kraits"));
    let _ = fs::write(local_lock, &encoded);
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_single_repo_lock() {
        let base = std::env::temp_dir().join(format!("kraits_lock_test_{}", now_secs()));
        let _ = fs::remove_dir_all(&base);
        let repo_a = base.join("repo_a");
        let repo_b = base.join("repo_b");
        let lock_file = base.join("kraits-adaptor.exe.repo.lock");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();

        let locked = enforce_single_repo_lock(&lock_file, &repo_a).expect("first repo should bind lock");
        assert_eq!(locked.repo_name, "repo_a");
        enforce_single_repo_lock(&lock_file, &repo_a).expect("same repo should pass lock");
        let err = enforce_single_repo_lock(&lock_file, &repo_b).unwrap_err();
        assert!(err.contains("Repo Lock Active"), "unexpected error: {}", err);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn incremental_sync_and_learning_persists() {
        let tmp = std::env::temp_dir().join(format!("kraits_adaptor_test_{}", now_secs()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(
            tmp.join("src/map.cpp"),
            "template <typename K>\nclass KeyedRwLockMap {\npublic:\n  void lock_shared(K k) {}\n};\n",
        )
        .unwrap();

        let mut store = RepoStore::open(&tmp);
        assert_eq!(store.files.len(), 1);
        store.record_lesson(
            "error_fix",
            "error: member access into incomplete type Entry",
            "Allocate Entry via entries_.try_emplace(k, std::make_unique<Entry>()) instead of entries_[k].get()",
            vec!["src/map.cpp".into()],
        );
        // Second open keeps AST hash cache and reloads learned lesson
        let store2 = RepoStore::open(&tmp);
        assert_eq!(store2.lessons.len(), 1);
        let ctx = store2.build_repo_context("fix Entry nullptr lock_shared", 4000);
        assert!(ctx.contains("KeyedRwLockMap"));
        assert!(ctx.contains("try_emplace"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn f17_purges_hallucinated_hot_files_and_rejects_mismatched_fp() {
        let tmp = std::env::temp_dir().join(format!("kraits_f17_test_{}", now_secs()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join("src/ok.py"), "def ok():\n    return 1\n").unwrap();

        let mut store = RepoStore::open(&tmp);
        assert!(store.validate_hot_path("src/ok.py").is_some());
        assert!(store.validate_hot_path(r"C:\Users\vinnj\minio-frontend\foo.ts").is_none());
        assert!(store.validate_hot_path("[\"file_\"]</> </tmp><user>").is_none());
        assert!(store.validate_hot_path("src/flsksk/app.py").is_none());

        store.hot_files.insert(r"C:\Users\vinnj\minio-frontend\foo.ts".into(), 12);
        store.hot_files.insert("src/ok.py".into(), 3);
        let (hb, ha, _, _) = store.purge_unindexed();
        assert_eq!(hb, 2);
        assert_eq!(ha, 1);
        assert!(store.hot_files.contains_key("src/ok.py"));

        // Mismatched fingerprint in ast_index.json refuses load
        let idx_p = tmp.join(".kraits/ast_index.json");
        let fake_idx = Json::obj(vec![
            ("repo_fingerprint", Json::Str("deadbeef00000000".into())),
            ("files", Json::Arr(vec![])),
            ("hot_files", Json::Obj(BTreeMap::from([("src/ok.py".into(), Json::Num(9.0))]))),
        ]);
        fs::write(&idx_p, fake_idx.encode()).unwrap();
        let mut store_reopen = RepoStore::open(&tmp);
        assert!(store_reopen.hot_files.is_empty());
        let _ = store_reopen.sync();

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn meta_capability_questions_skip_repo_context() {
        let tmp = std::env::temp_dir().join(format!("kraits_meta_q_{}", now_secs()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src/flask")).unwrap();
        fs::write(tmp.join("src/flask/helpers.py"), "def get_debug_flag():\n    return False\n").unwrap();
        let store = RepoStore::open(&tmp);
        assert!(!store.needs_repo_context("Can you work on any project or just flask?"));
        assert!(!store.needs_repo_context("Are you tied to this repo only?"));
        assert!(!store.needs_repo_context("What can you do?"));
        assert!(store.needs_repo_context("Fix get_debug_flag in src/flask/helpers.py"));
        assert!(store.needs_repo_context("Explain helpers.py"));
        let _ = fs::remove_dir_all(&tmp);
    }
}

fn is_meta_capability_question(ql: &str, terms: &[String]) -> bool {
    const ACTION_WORDS: &[&str] = &[
        "fix", "bug", "test", "tests", "pytest", "cargo", "build", "edit", "patch",
        "implement", "add", "create", "write", "refactor", "debug", "error", "fail",
        "failing", "traceback", "inspect", "explain", "diff", "verify", "run",
    ];
    if terms.iter().any(|t| ACTION_WORDS.contains(&t.as_str())) {
        return false;
    }
    const META_PHRASES: &[&str] = &[
        "any project", "any repo", "any codebase", "other project", "other repo",
        "different project", "different repo", "who are you", "what are you",
        "what can you do", "what do you do", "how do you work", "work on any",
    ];
    if META_PHRASES.iter().any(|p| ql.contains(p)) {
        return true;
    }
    const SCOPE_WORDS: &[&str] = &["just", "only", "tied", "bound", "locked", "limited"];
    const TARGET_WORDS: &[&str] = &["work", "support", "project", "projects", "repo", "repos", "code", "codebase"];
    terms.iter().any(|t| SCOPE_WORDS.contains(&t.as_str()))
        && terms.iter().any(|t| TARGET_WORDS.contains(&t.as_str()))
}
