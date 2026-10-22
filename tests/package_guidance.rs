use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn run(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_scoutpack"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for directory in ["apps/api/src", "apps/apiary", "apps/web/src"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"test":"ROOT_CHECK"}}"#,
    )
    .unwrap();
    // These sort before apps/api, exceeding the old global 25-chunk signal limit.
    for n in 0..30 {
        let directory = root.join(format!("a-fixtures/{n:02}"));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("package.json"), r#"{"scripts":{"test":"DUPLICATE_CHECK","lint":"UNRELATED_ONLY"},"dependencies":{"next":"1"}}"#).unwrap();
    }
    fs::write(
        root.join("apps/api/package.json"),
        r#"{"scripts":{"test":"DUPLICATE_CHECK","lint":"API_ONLY"},"dependencies":{"vite":"1"}}"#,
    )
    .unwrap();
    fs::write(
        root.join("apps/apiary/package.json"),
        r#"{"scripts":{"test":"PREFIX_NEIGHBOR"},"dependencies":{"jest":"1"}}"#,
    )
    .unwrap();
    fs::write(
        root.join("apps/web/package.json"),
        r#"{"scripts":{"test":"WEB_ONLY"},"dependencies":{"react":"1"}}"#,
    )
    .unwrap();
    fs::write(
        root.join("apps/api/src/login.ts"),
        "export function sentinelLogin() { return 'BASE'; }\n",
    )
    .unwrap();
    temp
}

fn assert_api_guidance(packet: &str) {
    for expected in [
        "ROOT_CHECK",
        "API_ONLY",
        "DUPLICATE_CHECK",
        "Vite",
        "apps/api/package.json",
    ] {
        assert!(packet.contains(expected), "missing {expected}: {packet}");
    }
    for unrelated in [
        "UNRELATED_ONLY",
        "PREFIX_NEIGHBOR",
        "WEB_ONLY",
        "Next.js",
        "Jest",
        "React",
    ] {
        assert!(
            !packet.contains(unrelated),
            "unrelated {unrelated}: {packet}"
        );
    }
}

#[test]
fn working_tree_guidance_uses_selected_paths_and_keeps_command_locations() {
    let temp = fixture();
    let packet = run(
        temp.path(),
        &["context", "sentinelLogin", "--budget", "3000"],
    );
    assert_api_guidance(&packet);
    assert!(packet.contains("run in `apps/api`"), "{packet}");
    assert!(packet.contains("run in `.`"), "{packet}");

    let small = run(
        temp.path(),
        &["context", "sentinelLogin", "--budget", "350", "--json"],
    );
    let json: serde_json::Value = serde_json::from_str(&small).unwrap();
    assert!(json["estimated_tokens"].as_u64().unwrap() <= 350);
    let packet = json["packet"].as_str().unwrap();
    assert!(!packet.contains("UNRELATED_ONLY"), "{packet}");

    let empty = run(
        temp.path(),
        &["context", "zzzznotfound", "--budget", "3000"],
    );
    assert!(!empty.contains("ROOT_CHECK"), "{empty}");
    assert!(!empty.contains("UNRELATED_ONLY"), "{empty}");
    assert!(!empty.contains("Next.js"), "{empty}");
}

#[test]
fn git_guidance_uses_affected_paths_and_committed_configuration() {
    let temp = fixture();
    let root = temp.path();
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.email", "scoutpack@example.com"]);
    git(root, &["config", "user.name", "ScoutPack Test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "base"]);
    git(root, &["checkout", "-b", "feature"]);
    fs::write(
        root.join("apps/api/src/login.ts"),
        "export function sentinelLogin() { return 'COMMITTED'; }\n",
    )
    .unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "change API"]);
    fs::write(
        root.join("apps/api/package.json"),
        r#"{"scripts":{"test":"DIRTY_ONLY"},"dependencies":{"django":"1"}}"#,
    )
    .unwrap();
    for scope in [
        vec!["--branch"],
        vec!["--diff", "main..HEAD"],
        vec!["--since", "main"],
    ] {
        let mut args = vec!["context", "sentinelLogin", "--budget", "3000"];
        args.extend(scope);
        let packet = run(root, &args);
        assert_api_guidance(&packet);
        assert!(!packet.contains("DIRTY_ONLY"), "{packet}");
        assert!(!packet.contains("Django"), "{packet}");
    }
    // A deleted source still identifies its package even though it has no head snippet.
    git(root, &["rm", "apps/api/src/login.ts"]);
    git(root, &["commit", "-m", "delete API source"]);
    let packet = run(
        root,
        &[
            "context",
            "sentinelLogin",
            "--diff",
            "HEAD~1..HEAD",
            "--budget",
            "3000",
        ],
    );
    assert_api_guidance(&packet);
    assert!(
        !packet.contains("```file:apps/api/src/login.ts"),
        "{packet}"
    );
    assert!(!root.join(".scoutpack").exists());
}

#[test]
fn cross_package_moves_keep_both_packages_but_not_siblings() {
    let temp = fixture();
    let root = temp.path();
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.email", "scoutpack@example.com"]);
    git(root, &["config", "user.name", "ScoutPack Test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "base"]);
    git(
        root,
        &["mv", "apps/api/src/login.ts", "apps/web/src/login.ts"],
    );
    git(root, &["commit", "-m", "move login between packages"]);
    let packet = run(
        root,
        &[
            "context",
            "sentinelLogin",
            "--diff",
            "HEAD~1..HEAD",
            "--budget",
            "3000",
        ],
    );
    for expected in ["API_ONLY", "WEB_ONLY", "Vite", "React"] {
        assert!(packet.contains(expected), "{packet}");
    }
    assert!(!packet.contains("PREFIX_NEIGHBOR"), "{packet}");
    assert!(!packet.contains("UNRELATED_ONLY"), "{packet}");
}

#[test]
fn mcp_context_scopes_guidance_without_changing_the_command_catalog() {
    let temp = fixture();
    let mut server = Command::new(env!("CARGO_BIN_EXE_scoutpack"))
        .args(["mcp", "."])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let requests = [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"package-test","version":"0.0.0"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"context","arguments":{"task":"sentinelLogin","budget":3000}}}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"commands","arguments":{}}}),
    ];
    for request in requests {
        writeln!(server.stdin.as_mut().unwrap(), "{request}").unwrap();
    }
    drop(server.stdin.take());
    let output = server.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let messages: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let packet =
        &messages.iter().find(|m| m["id"] == 2).unwrap()["result"]["structuredContent"]["packet"];
    assert_api_guidance(packet.as_str().unwrap());
    let catalog =
        &messages.iter().find(|m| m["id"] == 3).unwrap()["result"]["structuredContent"]["commands"];
    let catalog = catalog.as_array().unwrap();
    assert!(catalog
        .iter()
        .any(|command| command["command"] == "UNRELATED_ONLY"));
    assert_eq!(
        catalog
            .iter()
            .filter(|command| command["command"] == "DUPLICATE_CHECK")
            .count(),
        1
    );
    let duplicate = catalog
        .iter()
        .find(|command| command["command"] == "DUPLICATE_CHECK")
        .unwrap();
    assert_eq!(duplicate["source"], "a-fixtures/00/package.json");
}
