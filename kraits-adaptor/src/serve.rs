//! OpenAI-compatible HTTP/SSE proxy that adapts Kraits (`:8099`) to a local repo.

use crate::agent::{git_diff_repo, handle_demo_action, spawn_agent_task, TaskMap};
use crate::ast::fnv1a;
use crate::json::Json;
use crate::repo::{canonical_repo_path, repo_display_name, repo_fingerprint, RepoStore};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct ServerState {
    pub repo_root: PathBuf,
    pub store: Mutex<RepoStore>,
    pub upstream_host_port: String,
    pub requests_served: Mutex<u64>,
    /// Frozen Turn-1 repo context per conversation prefix hash so KV cache stays 100% invariant across turns.
    pub session_prefixes: Mutex<HashMap<u64, String>>,
    pub tasks: TaskMap,
}

fn parse_upstream(url: &str) -> String {
    let s = url.trim().trim_start_matches("http://").trim_start_matches("https://");
    let host_port = s.split('/').next().unwrap_or("127.0.0.1:8099");
    if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{}:80", host_port)
    }
}

pub fn run_server(repo_root: PathBuf, listen: &str, upstream_url: &str) -> Result<(), String> {
    let store = RepoStore::open(&repo_root);
    let ast_count = store.files.len();
    let lesson_count = store.lessons.len();
    let upstream_hp = parse_upstream(upstream_url);
    let state = Arc::new(ServerState {
        repo_root,
        store: Mutex::new(store),
        upstream_host_port: upstream_hp.clone(),
        requests_served: Mutex::new(0),
        session_prefixes: Mutex::new(HashMap::new()),
        tasks: Mutex::new(HashMap::new()),
    });

    let listener = TcpListener::bind(listen).map_err(|e| format!("Bind {}: {}", listen, e))?;
    println!(
        "[kraits-adaptor] locked to 1 repo: {} (fp={})",
        repo_display_name(&state.repo_root),
        &repo_fingerprint(&state.repo_root)[..8]
    );
    println!(
        "[kraits-adaptor] listening on http://{} -> upstream http://{} (repo files={}, learned_lessons={})",
        listen, upstream_hp, ast_count, lesson_count
    );

    for stream in listener.incoming() {
        let Ok(client) = stream else { continue };
        let st = Arc::clone(&state);
        std::thread::spawn(move || {
            let _ = handle_client(client, st);
        });
    }
    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> Result<(String, String, Vec<(String, String)>, Vec<u8>), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut first = String::new();
    let n = reader.read_line(&mut first).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("EOF".into());
    }
    let parts: Vec<&str> = first.trim_end().split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Malformed HTTP request line".into());
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = Vec::new();
    let mut content_len = 0usize;
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
            let kl = k.trim().to_ascii_lowercase();
            let vv = v.trim().to_string();
            if kl == "content-length" {
                content_len = vv.parse::<usize>().unwrap_or(0);
            }
            headers.push((k.trim().to_string(), vv));
        }
    }

    let mut body = vec![0u8; content_len];
    if content_len > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    Ok((method, path, headers, body))
}

fn write_response(client: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let hdr = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = client.write_all(hdr.as_bytes());
    let _ = client.write_all(body.as_bytes());
    let _ = client.flush();
}

fn write_json_response(client: &mut TcpStream, status: &str, body: &str) {
    write_response(client, status, "application/json", body);
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let qs = path.split_once('?')?.1;
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(url_decode(v));
            }
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(b) = u8::from_str_radix(hex, 16) {
                        out.push(b);
                        i += 3;
                        continue;
                    }
                }
                out.push(bytes[i]);
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn handle_client(mut client: TcpStream, state: Arc<ServerState>) -> Result<(), String> {
    let _ = client.set_read_timeout(Some(Duration::from_secs(600)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(600)));
    let (method, path, headers, mut body) = read_http_request(&mut client)?;

    if let Some((_, hdr_repo)) = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("x-kraits-repo")) {
        let locked_path = canonical_repo_path(&state.repo_root);
        let req_norm = hdr_repo.trim().strip_prefix(r"\\?\").unwrap_or(hdr_repo.trim());
        let locked_name = repo_display_name(&state.repo_root);
        if !req_norm.is_empty() && req_norm != locked_path && req_norm != locked_name {
            let msg = format!("Repo Lock Active: adaptor locked to '{}' ({}).", locked_name, locked_path);
            write_json_response(&mut client, "403 Forbidden", &Json::obj(vec![("error", Json::Str(msg))]).encode());
            return Ok(());
        }
    }

    if method == "GET" && (path == "/" || path == "/ui") {
        write_response(&mut client, "200 OK", "text/html; charset=utf-8", include_str!("ui.html"));
        return Ok(());
    }

    if method == "GET" && path.starts_with("/kraits/file") {
        let rel = query_param(&path, "path").unwrap_or_default().replace('\\', "/");
        let st = state.store.lock().unwrap();
        if let Some(af) = st.files.get(&rel) {
            let resp = Json::obj(vec![
                ("path", Json::Str(af.path.clone())),
                ("lines", Json::Num(af.lines as f64)),
                ("skeleton", Json::Str(af.render_skeleton())),
            ]);
            write_json_response(&mut client, "200 OK", &resp.encode());
        } else {
            write_json_response(&mut client, "404 Not Found", r#"{"error":"file not indexed"}"#);
        }
        return Ok(());
    }

    if method == "GET" && path == "/kraits/memory" {
        let mut st = state.store.lock().unwrap();
        let _ = st.sync();
        let lessons: Vec<Json> = st
            .lessons
            .iter()
            .rev()
            .map(|l| {
                Json::obj(vec![
                    ("kind", Json::Str(l.kind.clone())),
                    ("signature", Json::Str(l.signature.clone())),
                    ("detail", Json::Str(l.detail.clone())),
                    ("hits", Json::Num(l.hits as f64)),
                ])
            })
            .collect();
        let hot: Vec<Json> = st
            .hot_files
            .iter()
            .map(|(p, c)| Json::obj(vec![("path", Json::Str(p.clone())), ("hits", Json::Num(*c as f64))]))
            .collect();
        let resp = Json::obj(vec![("lessons", Json::Arr(lessons)), ("hot_files", Json::Arr(hot))]);
        write_json_response(&mut client, "200 OK", &resp.encode());
        return Ok(());
    }

    if method == "GET" && path == "/kraits/diff" {
        let resp = Json::obj(vec![("diff", Json::Str(git_diff_repo(&state.repo_root)))]);
        write_json_response(&mut client, "200 OK", &resp.encode());
        return Ok(());
    }

    if method == "POST" && path == "/kraits/demo" {
        let raw = String::from_utf8_lossy(&body);
        if let Ok(v) = Json::parse(&raw) {
            let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
            let target = v.get("target").and_then(|x| x.as_str()).unwrap_or("");
            match handle_demo_action(&state, action, target) {
                Ok(resp) => write_json_response(&mut client, "200 OK", &resp.encode()),
                Err(e) => write_json_response(&mut client, "400 Bad Request", &Json::obj(vec![("error", Json::Str(e))]).encode()),
            }
        } else {
            write_json_response(&mut client, "400 Bad Request", r#"{"error":"invalid json"}"#);
        }
        return Ok(());
    }

    if method == "POST" && path == "/kraits/task" {
        let raw = String::from_utf8_lossy(&body);
        if let Ok(v) = Json::parse(&raw) {
            let prompt = v.get("prompt").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
            if prompt.is_empty() {
                write_json_response(&mut client, "400 Bad Request", r#"{"error":"prompt required"}"#);
            } else {
                let tid = spawn_agent_task(Arc::clone(&state), prompt);
                write_json_response(&mut client, "200 OK", &Json::obj(vec![("task_id", Json::Str(tid))]).encode());
            }
        } else {
            write_json_response(&mut client, "400 Bad Request", r#"{"error":"invalid json"}"#);
        }
        return Ok(());
    }

    if method == "GET" && path.starts_with("/kraits/task") {
        let tid = query_param(&path, "id").unwrap_or_default();
        let map = state.tasks.lock().unwrap();
        if let Some(tr) = map.get(&tid) {
            write_json_response(&mut client, "200 OK", &tr.to_json().encode());
        } else {
            write_json_response(&mut client, "404 Not Found", r#"{"error":"task not found"}"#);
        }
        return Ok(());
    }

    if method == "GET" && path == "/kraits/repo" {
        let mut st = state.store.lock().unwrap();
        let (upd, rem) = st.sync();
        let reqs = *state.requests_served.lock().unwrap();
        let files: Vec<Json> = st
            .files
            .values()
            .map(|f| {
                Json::obj(vec![
                    ("path", Json::Str(f.path.clone())),
                    ("lang", Json::Str(f.lang.clone())),
                    ("lines", Json::Num(f.lines as f64)),
                    ("symbols", Json::Num(f.nodes.len() as f64)),
                ])
            })
            .collect();
        let resp = Json::obj(vec![
            ("kraits_adaptor", Json::Bool(true)),
            ("version", Json::Str(env!("CARGO_PKG_VERSION").into())),
            ("repo_locked", Json::Bool(true)),
            ("repo_name", Json::Str(repo_display_name(&state.repo_root))),
            ("repo_fingerprint", Json::Str(repo_fingerprint(&state.repo_root))),
            ("repo", Json::Str(state.repo_root.to_string_lossy().into_owned())),
            ("sync_updated", Json::Num(upd as f64)),
            ("sync_removed", Json::Num(rem as f64)),
            ("files", Json::Arr(files)),
            ("toolchain", Json::Arr(st.toolchain.iter().cloned().map(Json::Str).collect())),
            ("lessons_count", Json::Num(st.lessons.len() as f64)),
            ("requests_served", Json::Num(reqs as f64)),
        ]);
        write_json_response(&mut client, "200 OK", &resp.encode());
        return Ok(());
    }

    if method == "POST" && path == "/kraits/learn" {
        let raw = String::from_utf8_lossy(&body);
        if let Ok(v) = Json::parse(&raw) {
            let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("convention");
            let sig = v.get("signature").and_then(|x| x.as_str()).unwrap_or("note");
            let det = v.get("detail").and_then(|x| x.as_str()).unwrap_or("");
            let files: Vec<String> = v
                .get("files")
                .and_then(|x| x.as_arr())
                .map(|a| a.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let mut st = state.store.lock().unwrap();
            st.record_lesson(kind, sig, det, files);
            write_json_response(&mut client, "200 OK", &Json::obj(vec![("ok", Json::Bool(true)), ("lessons", Json::Num(st.lessons.len() as f64))]).encode());
        } else {
            write_json_response(&mut client, "400 Bad Request", r#"{"error":"invalid json"}"#);
        }
        return Ok(());
    }

    if method == "POST" && path.starts_with("/v1/chat/completions") {
        *state.requests_served.lock().unwrap() += 1;
        if let Ok(mut payload) = Json::parse(&String::from_utf8_lossy(&body)) {
            let mut st = state.store.lock().unwrap();
            let (_upd, _rem) = st.sync();
            if let Some(msgs) = payload.get("messages").and_then(|x| x.as_arr()).cloned() {
                st.learn_from_messages(&msgs);
            }

            // Compute a stable key from system prompt + original first user message.
            let sys_txt = payload
                .get("messages")
                .and_then(|x| x.as_arr())
                .and_then(|arr| arr.iter().find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system")))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let first_user_idx = payload
                .get("messages")
                .and_then(|x| x.as_arr())
                .and_then(|arr| arr.iter().position(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")));

            if let Some(u_idx) = first_user_idx {
                let first_user_txt = payload
                    .get("messages")
                    .and_then(|x| x.as_arr())
                    .and_then(|arr| arr.get(u_idx))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let last_user_txt = payload
                    .get("messages")
                    .and_then(|x| x.as_arr())
                    .and_then(|arr| arr.iter().rev().find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let repo_name = repo_display_name(&state.repo_root);
                let has_system = payload
                    .get("messages")
                    .and_then(|x| x.as_arr())
                    .is_some_and(|arr| arr.iter().any(|m| m.get("role").and_then(|r| r.as_str()) == Some("system")));

                if st.needs_repo_context(&first_user_txt) || st.needs_repo_context(last_user_txt) {
                    let conv_key = fnv1a(format!("{}\x00{}", sys_txt, first_user_txt).as_bytes());
                    let frozen_ctx = {
                        let mut cache = state.session_prefixes.lock().unwrap();
                        if let Some(existing) = cache.get(&conv_key) {
                            existing.clone()
                        } else {
                            let built = st.build_repo_context(&first_user_txt, 1800);
                            cache.insert(conv_key, built.clone());
                            built
                        }
                    };
                    // Always apply the exact frozen prefix onto first user message across EVERY turn
                    // so llama-server's KV cache hits 100% of prior context!
                    if let Some(arr) = payload.as_obj_mut().and_then(|o| o.get_mut("messages")).and_then(|m| m.as_arr_mut()) {
                        if let Some(obj) = arr.get_mut(u_idx).and_then(|m| m.as_obj_mut()) {
                            obj.insert("content".into(), Json::Str(format!("{}\n\n# Task\n{}", frozen_ctx, first_user_txt)));
                        }
                    }
                } else if !has_system {
                    let chat_sys = format!(
                        "You are Kraits, a fast coding assistant bound exclusively to the `{}` repository. Always state cleanly that you are Kraits bound to `{}` when greeting the user, and offer to inspect, edit, or test files in `{}`.",
                        repo_name, repo_name, repo_name
                    );
                    if let Some(arr) = payload.as_obj_mut().and_then(|o| o.get_mut("messages")).and_then(|m| m.as_arr_mut()) {
                        arr.insert(0, Json::obj(vec![("role", Json::Str("system".into())), ("content", Json::Str(chat_sys))]));
                    }
                }
            }
            body = payload.encode().into_bytes();
        }
    }

    // Proxy upstream transparently (HTTP/1.1 streaming-safe via Connection: close)
    let mut up = TcpStream::connect(&state.upstream_host_port)
        .map_err(|e| format!("Upstream {}: {}", state.upstream_host_port, e))?;
    let _ = up.set_read_timeout(Some(Duration::from_secs(600)));
    let _ = up.set_write_timeout(Some(Duration::from_secs(600)));

    let mut req_head = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, path, state.upstream_host_port);
    for (k, v) in &headers {
        let kl = k.to_ascii_lowercase();
        if kl == "host" || kl == "content-length" || kl == "connection" || kl == "accept-encoding" {
            continue;
        }
        req_head.push_str(&format!("{}: {}\r\n", k, v));
    }
    req_head.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()));

    up.write_all(req_head.as_bytes()).map_err(|e| e.to_string())?;
    if !body.is_empty() {
        up.write_all(&body).map_err(|e| e.to_string())?;
    }
    up.flush().map_err(|e| e.to_string())?;

    let mut buf = [0u8; 8192];
    loop {
        match up.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if client.write_all(&buf[..n]).is_err() {
                    break;
                }
                let _ = client.flush();
            }
            Err(_) => break,
        }
    }
    Ok(())
}
