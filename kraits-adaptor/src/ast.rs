//! Multi-language structural AST parser (C/C++, TypeScript/JS, Rust, Python, Go, Java).

use crate::json::Json;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lang {
    Cpp,
    TypeScript,
    Rust,
    Python,
    Go,
    Java,
}

impl Lang {
    pub fn from_ext(path: &str) -> Option<Self> {
        let lower = path.to_ascii_lowercase();
        let ext = lower.rsplit('.').next()?;
        match ext {
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h" | "c" => Some(Lang::Cpp),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(Lang::TypeScript),
            "rs" => Some(Lang::Rust),
            "py" => Some(Lang::Python),
            "go" => Some(Lang::Go),
            "java" => Some(Lang::Java),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Lang::Cpp => "cpp",
            Lang::TypeScript => "ts",
            Lang::Rust => "rust",
            Lang::Python => "python",
            Lang::Go => "go",
            Lang::Java => "java",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AstNode {
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
    pub body_hash: u64,
    pub calls: Vec<String>,
    pub children: Vec<AstNode>,
}

impl AstNode {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("kind", Json::Str(self.kind.clone())),
            ("name", Json::Str(self.name.clone())),
            ("signature", Json::Str(self.signature.clone())),
            ("start_line", Json::Num(self.start_line as f64)),
            ("end_line", Json::Num(self.end_line as f64)),
            ("body_hash", Json::Str(format!("{:016x}", self.body_hash))),
            ("calls", Json::Arr(self.calls.iter().cloned().map(Json::Str).collect())),
            ("children", Json::Arr(self.children.iter().map(AstNode::to_json).collect())),
        ])
    }

    pub fn from_json(v: &Json) -> Option<Self> {
        Some(AstNode {
            kind: v.get("kind")?.as_str()?.to_string(),
            name: v.get("name")?.as_str()?.to_string(),
            signature: v.get("signature")?.as_str()?.to_string(),
            start_line: v.get("start_line")?.as_i64()? as usize,
            end_line: v.get("end_line")?.as_i64()? as usize,
            body_hash: u64::from_str_radix(v.get("body_hash")?.as_str()?, 16).unwrap_or(0),
            calls: v
                .get("calls")?
                .as_arr()?
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            children: v
                .get("children")?
                .as_arr()?
                .iter()
                .filter_map(AstNode::from_json)
                .collect(),
        })
    }

    pub fn render_tree(&self, indent: usize, out: &mut String) {
        for _ in 0..indent {
            out.push_str("  ");
        }
        let show_kind = !(self.signature.starts_with(&self.kind)
            || self.signature.contains(&format!("{} ", self.kind)));
        if show_kind {
            out.push_str(&format!(
                "{} {} [L{}-L{}]",
                self.kind, self.signature, self.start_line, self.end_line
            ));
        } else {
            out.push_str(&format!("{} [L{}-L{}]", self.signature, self.start_line, self.end_line));
        }
        if !self.calls.is_empty() {
            let preview: Vec<&str> = self.calls.iter().take(6).map(|s| s.as_str()).collect();
            out.push_str(&format!(" -> calls: {}", preview.join(", ")));
        }
        out.push('\n');
        for ch in &self.children {
            ch.render_tree(indent + 1, out);
        }
    }
}

#[derive(Clone, Debug)]
pub struct AstFile {
    pub path: String,
    pub lang: String,
    pub lines: usize,
    pub hash: u64,
    pub imports: Vec<String>,
    pub nodes: Vec<AstNode>,
}

impl AstFile {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("path", Json::Str(self.path.clone())),
            ("lang", Json::Str(self.lang.clone())),
            ("lines", Json::Num(self.lines as f64)),
            ("hash", Json::Str(format!("{:016x}", self.hash))),
            ("imports", Json::Arr(self.imports.iter().cloned().map(Json::Str).collect())),
            ("nodes", Json::Arr(self.nodes.iter().map(AstNode::to_json).collect())),
        ])
    }

    pub fn from_json(v: &Json) -> Option<Self> {
        Some(AstFile {
            path: v.get("path")?.as_str()?.to_string(),
            lang: v.get("lang")?.as_str()?.to_string(),
            lines: v.get("lines")?.as_i64()? as usize,
            hash: u64::from_str_radix(v.get("hash")?.as_str()?, 16).unwrap_or(0),
            imports: v
                .get("imports")?
                .as_arr()?
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            nodes: v
                .get("nodes")?
                .as_arr()?
                .iter()
                .filter_map(AstNode::from_json)
                .collect(),
        })
    }

    pub fn render_skeleton(&self) -> String {
        let mut out = format!("{} ({}, {} lines)\n", self.path, self.lang, self.lines);
        if !self.imports.is_empty() {
            let preview: Vec<&str> = self.imports.iter().take(5).map(|s| s.as_str()).collect();
            out.push_str(&format!("  imports: {}\n", preview.join("; ")));
        }
        for n in &self.nodes {
            n.render_tree(1, &mut out);
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokKind {
    Ident,
    Num,
    Str,
    Sym(char),
    Op2(String),
    Preproc,
    Newline,
}

#[derive(Clone, Debug)]
struct Tok {
    kind: TokKind,
    text: String,
    line: usize,
    offset: usize,
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn lex(src: &str, hash_comment: bool, is_rust: bool) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut toks = Vec::new();
    let n = b.len();

    while i < n {
        let ch = b[i];
        if ch == b'\r' {
            i += 1;
            continue;
        }
        if ch == b'\n' {
            toks.push(Tok { kind: TokKind::Newline, text: "\n".into(), line, offset: i });
            line += 1;
            i += 1;
            continue;
        }
        if ch == b' ' || ch == b'\t' {
            i += 1;
            continue;
        }
        // Line comment
        if ch == b'/' && i + 1 < n && b[i + 1] == b'/' {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if hash_comment && ch == b'#' {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment (with nested /* */ handling for Rust safety)
        if ch == b'/' && i + 1 < n && b[i + 1] == b'*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if b[i] == b'\n' {
                    line += 1;
                    i += 1;
                } else if b[i] == b'/' && i + 1 < n && b[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if b[i] == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }
        // Preprocessor line (#include / #pragma / etc. in C/C++)
        if !hash_comment && ch == b'#' {
            let start = i;
            let sline = line;
            while i < n && b[i] != b'\n' {
                if b[i] == b'\\' && i + 1 < n && b[i + 1] == b'\n' {
                    line += 1;
                    i += 2;
                    continue;
                }
                i += 1;
            }
            let text = String::from_utf8_lossy(&b[start..i]).trim().to_string();
            toks.push(Tok { kind: TokKind::Preproc, text, line: sline, offset: start });
            continue;
        }
        // Rust raw strings r#"..."#
        if ch == b'r' && i + 1 < n && (b[i + 1] == b'"' || b[i + 1] == b'#') {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while j < n && b[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < n && b[j] == b'"' {
                let start = i;
                let sline = line;
                i = j + 1;
                while i < n {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    if b[i] == b'"' {
                        let mut k = 0usize;
                        while k < hashes && i + 1 + k < n && b[i + 1 + k] == b'#' {
                            k += 1;
                        }
                        if k == hashes {
                            i += 1 + hashes;
                            break;
                        }
                    }
                    i += 1;
                }
                toks.push(Tok {
                    kind: TokKind::Str,
                    text: String::from_utf8_lossy(&b[start..i]).into_owned(),
                    line: sline,
                    offset: start,
                });
                continue;
            }
        }
        // Quoted strings, template strings, and char literals (avoid consuming Rust lifetimes `'a` / `'static`)
        if ch == b'\'' && is_rust {
            let is_char_lit = if i + 1 < n && b[i + 1] == b'\\' {
                (i + 2..n.min(i + 12)).any(|k| b[k] == b'\'')
            } else {
                // Single UTF-8 scalar closed by `'`
                src[i + 1..]
                    .chars()
                    .next()
                    .map_or(false, |c| c != '\n' && b.get(i + 1 + c.len_utf8()) == Some(&b'\''))
            };
            if !is_char_lit {
                toks.push(Tok { kind: TokKind::Sym('\''), text: "'".into(), line, offset: i });
                i += 1;
                continue;
            }
        }
        if ch == b'"' || ch == b'\'' || ch == b'`' {
            let q = ch;
            let start = i;
            let sline = line;
            i += 1;
            while i < n {
                if b[i] == b'\n' {
                    line += 1;
                }
                if b[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if b[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
            toks.push(Tok {
                kind: TokKind::Str,
                text: String::from_utf8_lossy(&b[start..i]).into_owned(),
                line: sline,
                offset: start,
            });
            continue;
        }
        // Identifiers
        if ch.is_ascii_alphabetic() || ch == b'_' || ch == b'$' {
            let start = i;
            while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
                i += 1;
            }
            toks.push(Tok {
                kind: TokKind::Ident,
                text: String::from_utf8_lossy(&b[start..i]).into_owned(),
                line,
                offset: start,
            });
            continue;
        }
        // Numbers
        if ch.is_ascii_digit() {
            let start = i;
            while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                i += 1;
            }
            toks.push(Tok {
                kind: TokKind::Num,
                text: String::from_utf8_lossy(&b[start..i]).into_owned(),
                line,
                offset: start,
            });
            continue;
        }
        // Multi-char operators relevant to signatures (`->`, `=>`, `::`)
        if i + 1 < n {
            let pair = &b[i..i + 2];
            if pair == b"->" || pair == b"=>" || pair == b"::" {
                toks.push(Tok {
                    kind: TokKind::Op2(String::from_utf8_lossy(pair).into_owned()),
                    text: String::from_utf8_lossy(pair).into_owned(),
                    line,
                    offset: i,
                });
                i += 2;
                continue;
            }
        }
        toks.push(Tok {
            kind: TokKind::Sym(ch as char),
            text: (ch as char).to_string(),
            line,
            offset: i,
        });
        i += 1;
    }
    toks
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else" | "for" | "while" | "switch" | "case" | "return" | "break" | "continue"
            | "match" | "loop" | "catch" | "try" | "throw" | "new" | "delete" | "sizeof"
            | "typeof" | "await" | "yield" | "let" | "const" | "var" | "mut" | "pub"
            | "fn" | "async" | "unsafe" | "struct" | "class" | "enum" | "impl" | "trait"
            | "interface" | "type" | "namespace" | "mod" | "use" | "import" | "export"
            | "from" | "public" | "private" | "protected" | "static" | "virtual" | "inline"
            | "override" | "template" | "typename" | "void" | "bool" | "int" | "size_t"
            | "auto" | "string" | "true" | "false" | "nullptr" | "null" | "undefined"
            | "this" | "self" | "Self" | "super" | "Ok" | "Err" | "Some" | "None"
    )
}

fn collect_calls(toks: &[Tok], own_name: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    for idx in 0..toks.len() {
        if toks[idx].kind == TokKind::Ident {
            let name = &toks[idx].text;
            if name == own_name || is_keyword(name) || name.len() <= 1 {
                continue;
            }
            if let Some(next) = toks.get(idx + 1) {
                if matches!(next.kind, TokKind::Sym('(') | TokKind::Sym('<')) {
                    seen.insert(name.clone());
                }
            }
        }
    }
    seen.into_iter().take(16).collect()
}

fn find_matching_brace(toks: &[Tok], open_idx: usize) -> usize {
    let mut depth = 0usize;
    for (idx, t) in toks.iter().enumerate().skip(open_idx) {
        match t.kind {
            TokKind::Sym('{') => depth += 1,
            TokKind::Sym('}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return idx;
                }
            }
            _ => {}
        }
    }
    toks.len().saturating_sub(1)
}

fn clean_sig(src: &str, start_off: usize, end_off: usize) -> String {
    let slice = src.get(start_off..end_off).unwrap_or("");
    let mut compact: String = slice.split_whitespace().collect::<Vec<_>>().join(" ");
    for label in ["public:", "private:", "protected:"] {
        if let Some(rest) = compact.strip_prefix(label) {
            compact = rest.trim_start().to_string();
        }
    }
    if compact.len() > 140 {
        format!("{}...", &compact[..137])
    } else {
        compact
    }
}

fn non_nl(toks: &[Tok], mut idx: usize) -> usize {
    while idx < toks.len() && toks[idx].kind == TokKind::Newline {
        idx += 1;
    }
    idx
}

pub fn parse_ast(rel_path: &str, src: &str) -> Option<AstFile> {
    let lang = Lang::from_ext(rel_path)?;
    let hash = fnv1a(src.as_bytes());
    let lines = src.lines().count().max(1);
    let toks = lex(src, matches!(lang, Lang::Python), matches!(lang, Lang::Rust));
    let mut imports = Vec::new();
    for t in &toks {
        if t.kind == TokKind::Preproc && t.text.starts_with("#include") {
            imports.push(t.text.clone());
        }
    }

    let nodes = if matches!(lang, Lang::Python) {
        parse_python_ast(src, &toks, &mut imports)
    } else {
        extract_brace_imports(&toks, &lang, &mut imports);
        parse_brace_scope(&toks, 0, toks.len(), src, &lang, false)
    };

    Some(AstFile {
        path: rel_path.to_string(),
        lang: lang.name().to_string(),
        lines,
        hash,
        imports,
        nodes,
    })
}

fn extract_brace_imports(toks: &[Tok], lang: &Lang, imports: &mut Vec<String>) {
    let mut i = 0usize;
    while i < toks.len() {
        if toks[i].kind == TokKind::Ident
            && ((matches!(lang, Lang::Rust) && toks[i].text == "use")
                || (matches!(lang, Lang::TypeScript | Lang::Java) && toks[i].text == "import"))
        {
            let mut parts = Vec::new();
            let mut j = i;
            while j < toks.len()
                && toks[j].kind != TokKind::Sym(';')
                && toks[j].kind != TokKind::Newline
            {
                parts.push(toks[j].text.as_str());
                j += 1;
            }
            if !parts.is_empty() {
                let s = parts.join(" ");
                if s.len() <= 120 {
                    imports.push(s);
                }
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
}

fn parse_brace_scope(
    toks: &[Tok],
    start: usize,
    end: usize,
    src: &str,
    lang: &Lang,
    inside_type: bool,
) -> Vec<AstNode> {
    let mut nodes = Vec::new();
    let mut i = start;
    while i < end {
        let idx = non_nl(toks, i);
        if idx >= end {
            break;
        }
        // Skip standalone semicolons / braces
        if matches!(toks[idx].kind, TokKind::Preproc | TokKind::Sym(';')) {
            i = idx + 1;
            continue;
        }
        // Read current statement/declaration header up to `;` or `{`
        let stmt_start = idx;
        let mut scan = idx;
        let mut paren_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut found_brace: Option<usize> = None;
        let mut found_semi: Option<usize> = None;
        let mut has_parens = false;

        while scan < end {
            match &toks[scan].kind {
                TokKind::Sym('(') => {
                    paren_depth += 1;
                    has_parens = true;
                }
                TokKind::Sym(')') => paren_depth = paren_depth.saturating_sub(1),
                TokKind::Sym('<') if paren_depth == 0 => angle_depth += 1,
                TokKind::Sym('>') if paren_depth == 0 => angle_depth = angle_depth.saturating_sub(1),
                TokKind::Sym('{') if paren_depth == 0 => {
                    // Check if previous token is `=` (initializer list / object literal / TS `export const foo = {`)
                    let prev_non_nl = (stmt_start..scan)
                        .rev()
                        .find(|&k| toks[k].kind != TokKind::Newline);
                    let is_eq_rhs = prev_non_nl.map_or(false, |k| toks[k].kind == TokKind::Sym('='));
                    let is_arrow = prev_non_nl.map_or(false, |k| toks[k].text == "=>");
                    if is_eq_rhs && !is_arrow {
                        // Skip balanced brace literal until matching `}`
                        scan = find_matching_brace(toks, scan) + 1;
                        continue;
                    }
                    let _ = angle_depth;
                    found_brace = Some(scan);
                    break;
                }
                TokKind::Sym(';') if paren_depth == 0 => {
                    found_semi = Some(scan);
                    break;
                }
                _ => {}
            }
            scan += 1;
        }

        if let Some(brace_idx) = found_brace {
            let close_idx = find_matching_brace(toks, brace_idx).min(end.saturating_sub(1));
            let hdr = &toks[stmt_start..brace_idx];
            if let Some((kind, name)) = classify_decl(hdr, lang, inside_type, has_parens) {
                let sig = clean_sig(src, toks[stmt_start].offset, toks[brace_idx].offset);
                let start_line = toks[stmt_start].line;
                let end_line = toks[close_idx].line;
                let body_slice = src
                    .get(toks[brace_idx].offset..=toks[close_idx].offset)
                    .unwrap_or("");
                let body_hash = fnv1a(body_slice.as_bytes());
                let is_container = matches!(
                    kind.as_str(),
                    "namespace" | "mod" | "class" | "struct" | "trait" | "interface" | "impl"
                );
                let children = if is_container && brace_idx + 1 < close_idx {
                    parse_brace_scope(toks, brace_idx + 1, close_idx, src, lang, kind != "namespace" && kind != "mod")
                } else {
                    Vec::new()
                };
                let calls = if is_container {
                    Vec::new()
                } else {
                    collect_calls(&toks[brace_idx + 1..close_idx], &name)
                };
                nodes.push(AstNode {
                    kind,
                    name,
                    signature: sig,
                    start_line,
                    end_line,
                    body_hash,
                    calls,
                    children,
                });
            }
            i = close_idx + 1;
            continue;
        }

        if let Some(semi_idx) = found_semi {
            let hdr = &toks[stmt_start..semi_idx];
            if let Some((kind, name)) = classify_signature_only(hdr, lang) {
                let sig = clean_sig(src, toks[stmt_start].offset, toks[semi_idx].offset);
                nodes.push(AstNode {
                    kind,
                    name,
                    signature: sig,
                    start_line: toks[stmt_start].line,
                    end_line: toks[semi_idx].line,
                    body_hash: 0,
                    calls: Vec::new(),
                    children: Vec::new(),
                });
            }
            i = semi_idx + 1;
            continue;
        }

        i = scan.max(idx + 1);
    }
    nodes
}

fn classify_decl(
    hdr: &[Tok],
    lang: &Lang,
    inside_type: bool,
    has_parens: bool,
) -> Option<(String, String)> {
    let words: Vec<&Tok> = hdr
        .iter()
        .filter(|t| t.kind != TokKind::Newline)
        .collect();
    if words.is_empty() {
        return None;
    }

    // Look for container keywords outside `<...>` / `(...)`
    let mut paren = 0usize;
    let mut angle = 0usize;
    for (idx, t) in words.iter().enumerate() {
        match t.kind {
            TokKind::Sym('(') => paren += 1,
            TokKind::Sym(')') => paren = paren.saturating_sub(1),
            TokKind::Sym('<') if paren == 0 => angle += 1,
            TokKind::Sym('>') if paren == 0 => angle = angle.saturating_sub(1),
            TokKind::Ident if paren == 0 && angle == 0 => {
                let w = t.text.as_str();
                match w {
                    "namespace" | "mod" => {
                        let name = words.get(idx + 1).map(|x| x.text.clone()).unwrap_or_else(|| "anon".into());
                        return Some((w.to_string(), name));
                    }
                    "class" | "struct" | "enum" | "trait" | "interface" => {
                        if let Some(name_tok) = words.iter().skip(idx + 1).find(|x| x.kind == TokKind::Ident) {
                            return Some((w.to_string(), name_tok.text.clone()));
                        }
                    }
                    "impl" if matches!(lang, Lang::Rust) => {
                        let rest: Vec<&str> = words.iter().skip(idx + 1).map(|x| x.text.as_str()).collect();
                        let name = rest.iter().rfind(|&&s| s.chars().next().map_or(false, |c| c.is_ascii_alphabetic()))
                            .unwrap_or(&"impl");
                        return Some(("impl".into(), (*name).to_string()));
                    }
                    "fn" if matches!(lang, Lang::Rust) => {
                        if let Some(name_tok) = words.get(idx + 1) {
                            let k = if inside_type { "method" } else { "fn" };
                            return Some((k.into(), name_tok.text.clone()));
                        }
                    }
                    "func" if matches!(lang, Lang::Go) => {
                        for k in idx + 1..words.len() {
                            if words[k].kind == TokKind::Ident {
                                let kind = if words[idx + 1].kind == TokKind::Sym('(') { "method" } else { "fn" };
                                return Some((kind.into(), words[k].text.clone()));
                            }
                        }
                    }
                    "function" if matches!(lang, Lang::TypeScript) => {
                        if let Some(name_tok) = words.get(idx + 1) {
                            return Some(("fn".into(), name_tok.text.clone()));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Arrow function (`const foo = (...) => {`) in TS/JS
    if matches!(lang, Lang::TypeScript) && words.iter().any(|t| t.text == "=>") {
        if let Some(eq_idx) = words.iter().position(|t| t.kind == TokKind::Sym('=')) {
            if let Some(name_tok) = words[..eq_idx].iter().rev().find(|t| t.kind == TokKind::Ident) {
                return Some(("fn".into(), name_tok.text.clone()));
            }
        }
    }

    // Function / C++ method / TS class method (`name(...) {`)
    if has_parens {
        // Ignore control-flow statements (`if (...) {`, `while (...) {`, `catch (...) {`)
        let first_ident = words.iter().find(|t| t.kind == TokKind::Ident).map(|t| t.text.as_str());
        if matches!(first_ident, Some("if" | "for" | "while" | "switch" | "catch" | "try" | "else" | "match")) {
            return None;
        }
        // Find the `(` at paren depth 0 that isn't `decltype` / attribute
        for idx in 1..words.len() {
            if words[idx].kind == TokKind::Sym('(') && words[idx - 1].kind == TokKind::Ident {
                let nm = &words[idx - 1].text;
                if !is_keyword(nm) {
                    let kind = if inside_type { "method" } else { "fn" };
                    return Some((kind.into(), nm.clone()));
                }
            }
        }
    }

    None
}

fn classify_signature_only(hdr: &[Tok], lang: &Lang) -> Option<(String, String)> {
    let words: Vec<&Tok> = hdr.iter().filter(|t| t.kind != TokKind::Newline).collect();
    if matches!(lang, Lang::TypeScript) {
        if let Some(pos) = words.iter().position(|t| t.text == "type" && t.kind == TokKind::Ident) {
            if let Some(name) = words.get(pos + 1) {
                return Some(("type".into(), name.text.clone()));
            }
        }
    }
    if matches!(lang, Lang::Rust) {
        if let Some(pos) = words.iter().position(|t| t.text == "type" && t.kind == TokKind::Ident) {
            if let Some(name) = words.get(pos + 1) {
                return Some(("type".into(), name.text.clone()));
            }
        }
    }
    None
}

fn parse_python_ast(src: &str, toks: &[Tok], imports: &mut Vec<String>) -> Vec<AstNode> {
    let lines: Vec<&str> = src.lines().collect();
    let mut stack: Vec<(usize, AstNode)> = Vec::new();
    let mut top: Vec<AstNode> = Vec::new();

    let flush_stack = |stack: &mut Vec<(usize, AstNode)>, top: &mut Vec<AstNode>, min_indent: usize| {
        while let Some((ind, _)) = stack.last() {
            if *ind < min_indent {
                break;
            }
            let (_, node) = stack.pop().unwrap();
            if let Some((_, parent)) = stack.last_mut() {
                parent.children.push(node);
            } else {
                top.push(node);
            }
        }
    };

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        let indent = raw.len() - raw.trim_start().len();
        if indent == 0 && (trimmed.starts_with("import ") || trimmed.starts_with("from ")) && trimmed.len() <= 120 {
            if imports.len() < 12 {
                imports.push(trimmed.to_string());
            }
            continue;
        }
        if trimmed.starts_with("class ") || trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            flush_stack(&mut stack, &mut top, indent);
            let mut end = idx + 1;
            let mut paren_d: i32 = 0;
            for ch in trimmed.chars() {
                if ch == '(' || ch == '[' { paren_d += 1; }
                else if ch == ')' || ch == ']' { paren_d = paren_d.saturating_sub(1); }
            }
            let mut hdr_end = idx;
            while paren_d > 0 && hdr_end + 1 < lines.len() {
                hdr_end += 1;
                for ch in lines[hdr_end].chars() {
                    if ch == '(' || ch == '[' { paren_d += 1; }
                    else if ch == ')' || ch == ']' { paren_d = paren_d.saturating_sub(1); }
                }
                end = end.max(hdr_end + 1);
            }
            while end < lines.len() {
                let l = lines[end];
                let lt = l.trim();
                if !lt.is_empty() && !lt.starts_with('#') && (l.len() - l.trim_start().len()) <= indent {
                    break;
                }
                end += 1;
            }
            let mut end_trim = end;
            while end_trim > idx + 1 && lines[end_trim - 1].trim().is_empty() {
                end_trim -= 1;
            }
            let (kind, rest) = if trimmed.starts_with("class ") {
                ("class", &trimmed[6..])
            } else if trimmed.starts_with("async def ") {
                ("fn", &trimmed[10..])
            } else {
                ("fn", &trimmed[4..])
            };
            let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("anon");
            let sig_lines: Vec<&str> = lines[idx..=hdr_end].iter().map(|s| s.trim()).collect();
            let mut sig = sig_lines.join(" ").trim_end_matches(':').to_string();
            if sig.len() > 140 {
                sig = format!("{}...", &sig[..137]);
            }
            let body = lines[idx..end_trim].join("\n");
            let in_class = stack.last().map_or(false, |(_, p)| p.kind == "class");
            let line_toks: Vec<Tok> = toks
                .iter()
                .filter(|t| t.line >= idx + 1 && t.line <= end_trim)
                .cloned()
                .collect();
            let calls = if kind == "class" { Vec::new() } else { collect_calls(&line_toks, name) };
            let node = AstNode {
                kind: if in_class && kind == "fn" { "method".into() } else { kind.into() },
                name: name.to_string(),
                signature: sig,
                start_line: idx + 1,
                end_line: end_trim,
                body_hash: fnv1a(body.as_bytes()),
                calls,
                children: Vec::new(),
            };
            stack.push((indent, node));
        }
    }
    flush_stack(&mut stack, &mut top, 0);
    top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpp_class_and_methods() {
        let code = r#"
#include <mutex>
template <typename K>
class KeyedRwLockMap {
public:
    void lock_shared(K k) {
        std::unique_lock<std::mutex> lk(mtx_);
        cv_.wait(lk);
    }
    void unlock_shared(K k) {
        cleanup(k);
    }
};
"#;
        let ast = parse_ast("cpp/keyed_rwlock.hpp", code).unwrap();
        assert_eq!(ast.lang, "cpp");
        assert_eq!(ast.imports, vec!["#include <mutex>"]);
        assert_eq!(ast.nodes.len(), 1);
        assert_eq!(ast.nodes[0].kind, "class");
        assert_eq!(ast.nodes[0].name, "KeyedRwLockMap");
        assert_eq!(ast.nodes[0].children.len(), 2);
        assert_eq!(ast.nodes[0].children[0].name, "lock_shared");
        assert!(ast.nodes[0].children[1].calls.contains(&"cleanup".to_string()));
    }

    #[test]
    fn parses_ts_and_rust_trees() {
        let ts = "export interface Opts { ttlMs: number; }\nexport class DataLoader<K, V> {\n  async load(key: K): Promise<V> { return helper(key); }\n}\nexport const run = () => { return 1; };";
        let ast = parse_ast("ts/loader.ts", ts).unwrap();
        assert!(ast.nodes.iter().any(|n| n.kind == "interface" && n.name == "Opts"));
        let cls = ast.nodes.iter().find(|n| n.kind == "class" && n.name == "DataLoader").unwrap();
        assert_eq!(cls.children[0].name, "load");
        assert!(ast.nodes.iter().any(|n| n.kind == "fn" && n.name == "run"));

        let rs = "use std::sync::Mutex;\npub struct Processor<'a> { tag: &'a str }\nimpl<'a> Processor<'a> {\n  pub fn kind_name(&self) -> &'static str { \"ok\" }\n  pub fn apply(&mut self) -> Result<(), ()> { Ok(()) }\n}";
        let rast = parse_ast("rust_tx/src/lib.rs", rs).unwrap();
        assert_eq!(rast.nodes.len(), 2);
        assert_eq!(rast.nodes[1].kind, "impl");
        assert_eq!(rast.nodes[1].children.len(), 2);
        assert_eq!(rast.nodes[1].children[0].name, "kind_name");
        assert_eq!(rast.nodes[1].children[1].name, "apply");
    }

    #[test]
    fn parses_python_nested_class_and_imports() {
        let py = "from dataclasses import dataclass\n\nclass App:\n    def route(self, rule: str) -> str:\n        return normalize(rule)\n\n    async def dispatch(self):\n        return handle()\n";
        let ast = parse_ast("src/app.py", py).unwrap();
        assert_eq!(ast.lang, "python");
        assert_eq!(ast.imports, vec!["from dataclasses import dataclass"]);
        assert_eq!(ast.nodes.len(), 1);
        assert_eq!(ast.nodes[0].name, "App");
        assert_eq!(ast.nodes[0].children.len(), 2);
        assert_eq!(ast.nodes[0].children[0].name, "route");
        assert!(ast.nodes[0].children[0].calls.contains(&"normalize".to_string()));
        assert_eq!(ast.nodes[0].children[1].name, "dispatch");
    }
}
