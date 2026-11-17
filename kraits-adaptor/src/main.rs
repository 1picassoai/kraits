//! Kraits Repo Adaptor CLI & Server (`kraits-adaptor`).

mod agent;
mod ast;
mod json;
mod repo;
mod serve;

use repo::{default_adaptor_lock_path, enforce_single_repo_lock, RepoStore};
use std::env;
use std::path::PathBuf;
use std::process;

fn arg_val(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn print_usage() {
    eprintln!(
        "Usage:
  kraits-adaptor index  --repo <dir>
  kraits-adaptor ast    --repo <dir> <rel-file>
  kraits-adaptor symbol --repo <dir> <rel-file> <symbol>
  kraits-adaptor query  --repo <dir> <query> [--budget <chars>]
  kraits-adaptor learn  --repo <dir> --kind <kind> --sig <sig> --detail <detail> [--file <rel>]
  kraits-adaptor serve  --repo <dir> [--listen 127.0.0.1:8100] [--upstream http://127.0.0.1:8099]"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }
    let cmd = args[1].as_str();
    let repo_dir = PathBuf::from(arg_val(&args, "--repo").unwrap_or_else(|| ".".into()));

    if !matches!(cmd, "index" | "purge" | "ast" | "symbol" | "query" | "learn" | "serve") {
        print_usage();
        process::exit(1);
    }

    let lock_path = default_adaptor_lock_path();
    let lock_info = match enforce_single_repo_lock(&lock_path, &repo_dir) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(3);
        }
    };
    let repo_dir = PathBuf::from(&lock_info.repo_path);

    match cmd {
        "purge" => {
            let raw_hf_before = std::fs::read_to_string(repo_dir.join(".kraits/ast_index.json"))
                .ok()
                .and_then(|s| crate::json::Json::parse(&s).ok())
                .and_then(|v| v.get("hot_files").and_then(|x| x.as_obj()).map(|o| o.len()))
                .unwrap_or(0);
            let raw_les_before = std::fs::read_to_string(repo_dir.join(".kraits/memory.jsonl"))
                .ok()
                .map(|s| s.lines().filter(|l| l.contains("\"kind\"")).count())
                .unwrap_or(0);
            let mut store = RepoStore::open(&repo_dir);
            let _ = store.purge_unindexed();
            let (upd, rem) = store.sync();
            println!(
                "PURGE fp={}: hot_files {} -> {}, lessons {} -> {}, files={}, sync(upd={}, rem={})",
                repo::repo_fingerprint(&repo_dir),
                raw_hf_before,
                store.hot_files.len(),
                raw_les_before,
                store.lessons.len(),
                store.files.len(),
                upd,
                rem
            );
        }
        "index" => {
            let store = RepoStore::open(&repo_dir);
            println!(
                "Indexed {} source file(s), {} manifest(s), {} learned lesson(s) in {}",
                store.files.len(),
                store.manifests.len(),
                store.lessons.len(),
                repo_dir.display()
            );
            for af in store.files.values() {
                print!("{}", af.render_skeleton());
            }
        }
        "ast" => {
            let store = RepoStore::open(&repo_dir);
            let rel = args.last().cloned().unwrap_or_default().replace('\\', "/");
            if let Some(af) = store.files.get(&rel) {
                println!("{}", af.to_json().encode());
            } else {
                eprintln!("File not in AST index: {}", rel);
                process::exit(2);
            }
        }
        "symbol" => {
            if args.len() < 4 {
                print_usage();
                process::exit(1);
            }
            let store = RepoStore::open(&repo_dir);
            let rel = args[args.len() - 2].replace('\\', "/");
            let sym = &args[args.len() - 1];
            match store.extract_ast_symbol(&rel, sym) {
                Some(body) => println!("{}", body),
                None => {
                    eprintln!("Symbol `{}` not found in `{}`", sym, rel);
                    process::exit(2);
                }
            }
        }
        "query" => {
            let store = RepoStore::open(&repo_dir);
            let budget = arg_val(&args, "--budget")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(2800);
            let query = args.last().cloned().unwrap_or_default();
            println!("{}", store.build_repo_context(&query, budget));
        }
        "learn" => {
            let mut store = RepoStore::open(&repo_dir);
            let kind = arg_val(&args, "--kind").unwrap_or_else(|| "convention".into());
            let sig = arg_val(&args, "--sig").unwrap_or_else(|| "lesson".into());
            let detail = arg_val(&args, "--detail").unwrap_or_default();
            let files: Vec<String> = arg_val(&args, "--file").into_iter().collect();
            store.record_lesson(&kind, &sig, &detail, files);
            println!("Recorded lesson (total={})", store.lessons.len());
        }
        "serve" => {
            let listen = arg_val(&args, "--listen").unwrap_or_else(|| "127.0.0.1:8100".into());
            let upstream = arg_val(&args, "--upstream").unwrap_or_else(|| "http://127.0.0.1:8099".into());
            if let Err(e) = serve::run_server(repo_dir, &listen, &upstream) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        _ => {
            print_usage();
            process::exit(1);
        }
    }
}
