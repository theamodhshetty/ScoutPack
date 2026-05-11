use std::{fs, path::Path, process::Command};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_scoutpack")
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

#[test]
fn init_pack_search_context_stats_work() {
    let temp = tempfile::tempdir().unwrap();
    copy_dir(Path::new("tests/fixtures/nextjs-basic"), temp.path());

    let init = Command::new(bin())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let second_init = Command::new(bin())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        second_init.status.success(),
        "{}",
        String::from_utf8_lossy(&second_init.stderr)
    );

    let pack = Command::new(bin())
        .arg("pack")
        .arg(".")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        pack.status.success(),
        "{}",
        String::from_utf8_lossy(&pack.stderr)
    );
    assert!(temp.path().join(".scoutpack/pack.sqlite").exists());

    let second_pack = Command::new(bin())
        .arg("pack")
        .arg(".")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        second_pack.status.success(),
        "{}",
        String::from_utf8_lossy(&second_pack.stderr)
    );
    let second_pack_out = String::from_utf8_lossy(&second_pack.stdout);
    assert!(
        second_pack_out.contains("Indexed 0 files"),
        "{second_pack_out}"
    );
    assert!(second_pack_out.contains("reused "), "{second_pack_out}");
    assert!(
        second_pack_out.contains("added 0 chunks"),
        "{second_pack_out}"
    );

    let search = Command::new(bin())
        .args(["search", "auth middleware", "--limit", "5"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        search.status.success(),
        "{}",
        String::from_utf8_lossy(&search.stderr)
    );
    let search_out = String::from_utf8_lossy(&search.stdout);
    assert!(
        search_out.contains("src/middleware/auth.ts"),
        "{search_out}"
    );
    assert!(
        search_out
            .find("src/middleware/auth.ts")
            .unwrap_or(usize::MAX)
            < search_out.find("README.md").unwrap_or(usize::MAX),
        "{search_out}"
    );

    let search_json = Command::new(bin())
        .args(["search", "auth middleware", "--limit", "5", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        search_json.status.success(),
        "{}",
        String::from_utf8_lossy(&search_json.stderr)
    );
    let search_json_value: serde_json::Value = serde_json::from_slice(&search_json.stdout).unwrap();
    assert_eq!(search_json_value["query"], "auth middleware");
    assert!(search_json_value["results"].is_array());

    let context = Command::new(bin())
        .args(["context", "fix login redirect loop", "--budget", "2000"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    let context_out = String::from_utf8_lossy(&context.stdout);
    assert!(context_out.contains("# ScoutPack Context"), "{context_out}");
    assert!(
        context_out.contains("src/app/login/page.tsx"),
        "{context_out}"
    );
    assert!(context_out.contains("src/lib/session.ts"), "{context_out}");
    assert!(
        context_out
            .find("src/app/login/page.tsx")
            .unwrap_or(usize::MAX)
            < context_out.find("README.md").unwrap_or(usize::MAX),
        "{context_out}"
    );
    assert!(context_out.contains("(source:"), "{context_out}");
    assert!(context_out.contains("vitest"), "{context_out}");

    let context_json = Command::new(bin())
        .args([
            "context",
            "fix login redirect loop",
            "--budget",
            "2000",
            "--json",
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        context_json.status.success(),
        "{}",
        String::from_utf8_lossy(&context_json.stderr)
    );
    let context_json_value: serde_json::Value =
        serde_json::from_slice(&context_json.stdout).unwrap();
    assert_eq!(context_json_value["task"], "fix login redirect loop");
    assert!(context_json_value["packet"]
        .as_str()
        .unwrap()
        .contains("Risks:"));

    let tiny_context = Command::new(bin())
        .args(["context", "fix login redirect loop", "--budget", "300"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        tiny_context.status.success(),
        "{}",
        String::from_utf8_lossy(&tiny_context.stderr)
    );
    let tiny_context_out = String::from_utf8_lossy(&tiny_context.stdout);
    assert!(tiny_context_out.contains("Commands:"), "{tiny_context_out}");
    assert!(tiny_context_out.contains("Risks:"), "{tiny_context_out}");
    assert!(
        tiny_context_out.contains("Budget exhausted")
            || tiny_context_out.contains("Unknown from index"),
        "{tiny_context_out}"
    );

    let stats = Command::new(bin())
        .arg("stats")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        stats.status.success(),
        "{}",
        String::from_utf8_lossy(&stats.stderr)
    );
    let stats_out = String::from_utf8_lossy(&stats.stdout);
    assert!(stats_out.contains("Files:"), "{stats_out}");

    let stats_json = Command::new(bin())
        .args(["stats", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        stats_json.status.success(),
        "{}",
        String::from_utf8_lossy(&stats_json.stderr)
    );
    let stats_json_value: serde_json::Value = serde_json::from_slice(&stats_json.stdout).unwrap();
    assert!(stats_json_value["file_count"].as_i64().unwrap() > 0);
}
