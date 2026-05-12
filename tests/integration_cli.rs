use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

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
    assert!(context_json_value["packet"]
        .as_str()
        .unwrap()
        .contains("Token Budget Summary:"));

    let context_format_json = Command::new(bin())
        .args([
            "context",
            "fix login redirect loop",
            "--budget",
            "2000",
            "--format",
            "json",
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        context_format_json.status.success(),
        "{}",
        String::from_utf8_lossy(&context_format_json.stderr)
    );
    let context_format_json_value: serde_json::Value =
        serde_json::from_slice(&context_format_json.stdout).unwrap();
    assert_eq!(context_format_json_value["task"], "fix login redirect loop");

    let context_xml = Command::new(bin())
        .args([
            "context",
            "fix login redirect loop",
            "--budget",
            "2000",
            "--format",
            "xml",
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        context_xml.status.success(),
        "{}",
        String::from_utf8_lossy(&context_xml.stderr)
    );
    let context_xml_out = String::from_utf8_lossy(&context_xml.stdout);
    assert!(context_xml_out.contains("<scoutpack_context>"));
    assert!(context_xml_out.contains("<estimated_tokens>"));
    assert!(context_xml_out.contains("&lt;"));

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

    let completions = Command::new(bin())
        .args(["completions", "zsh"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        completions.status.success(),
        "{}",
        String::from_utf8_lossy(&completions.stderr)
    );
    let completions_out = String::from_utf8_lossy(&completions.stdout);
    assert!(completions_out.contains("#compdef scoutpack"));
    assert!(completions_out.contains("completions"));

    let mut mcp = Command::new(bin())
        .args(["mcp", "."])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = mcp.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-11-25","capabilities":{{}},"clientInfo":{{"name":"integration-test","version":"0.0.0"}}}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized","params":{{}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"stats","arguments":{{}}}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"search","arguments":{{"query":"auth middleware","limit":3}}}}}}"#
        )
        .unwrap();
    }
    drop(mcp.stdin.take());
    let mcp_output = mcp.wait_with_output().unwrap();
    assert!(
        mcp_output.status.success(),
        "{}",
        String::from_utf8_lossy(&mcp_output.stderr)
    );
    let mcp_stdout = String::from_utf8_lossy(&mcp_output.stdout);
    let messages: Vec<serde_json::Value> = mcp_stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tools = messages.iter().find(|message| message["id"] == 2).unwrap()["result"]["tools"]
        .as_array()
        .unwrap();
    assert!(tools.iter().any(|tool| tool["name"] == "context"));
    assert!(tools.iter().any(|tool| tool["name"] == "file_summary"));
    let mcp_stats = messages.iter().find(|message| message["id"] == 3).unwrap()["result"]
        ["structuredContent"]
        .clone();
    assert!(mcp_stats["file_count"].as_i64().unwrap() > 0);
    let mcp_search = messages.iter().find(|message| message["id"] == 4).unwrap()["result"]
        ["structuredContent"]
        .clone();
    assert!(mcp_search["results"]
        .as_array()
        .unwrap()
        .iter()
        .any(|result| result["path"] == "src/middleware/auth.ts"));
}

#[test]
fn javascript_fixture_indexes_symbols_and_routes() {
    let temp = tempfile::tempdir().unwrap();
    copy_dir(Path::new("tests/fixtures/express-basic"), temp.path());

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

    let auth_search = Command::new(bin())
        .args(["search", "requireAuth", "--limit", "5"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        auth_search.status.success(),
        "{}",
        String::from_utf8_lossy(&auth_search.stderr)
    );
    let auth_out = String::from_utf8_lossy(&auth_search.stdout);
    assert!(auth_out.contains("src/server.js"), "{auth_out}");
    assert!(auth_out.contains("[function] requireAuth"), "{auth_out}");

    let route_search = Command::new(bin())
        .args(["search", "auth session", "--limit", "5"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        route_search.status.success(),
        "{}",
        String::from_utf8_lossy(&route_search.stderr)
    );
    let route_out = String::from_utf8_lossy(&route_search.stdout);
    assert!(
        route_out.contains("[route-handler] GET /auth/session"),
        "{route_out}"
    );

    let component_search = Command::new(bin())
        .args(["search", "LoginButton", "--limit", "5"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        component_search.status.success(),
        "{}",
        String::from_utf8_lossy(&component_search.stderr)
    );
    let component_out = String::from_utf8_lossy(&component_search.stdout);
    assert!(
        component_out.contains("src/LoginButton.jsx"),
        "{component_out}"
    );
    assert!(
        component_out.contains("[component] LoginButton"),
        "{component_out}"
    );
}

#[test]
fn python_and_go_config_fixtures_emit_commands_and_frameworks() {
    let python_temp = tempfile::tempdir().unwrap();
    copy_dir(
        Path::new("tests/fixtures/fastapi-config"),
        python_temp.path(),
    );

    let init = Command::new(bin())
        .arg("init")
        .current_dir(python_temp.path())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let pack = Command::new(bin())
        .args(["pack", "."])
        .current_dir(python_temp.path())
        .output()
        .unwrap();
    assert!(
        pack.status.success(),
        "{}",
        String::from_utf8_lossy(&pack.stderr)
    );

    let python_context = Command::new(bin())
        .args(["context", "fix login validation", "--budget", "2500"])
        .current_dir(python_temp.path())
        .output()
        .unwrap();
    assert!(
        python_context.status.success(),
        "{}",
        String::from_utf8_lossy(&python_context.stderr)
    );
    let python_out = String::from_utf8_lossy(&python_context.stdout);
    assert!(python_out.contains("FastAPI"), "{python_out}");
    assert!(python_out.contains("Pydantic"), "{python_out}");
    assert!(python_out.contains("pytest"), "{python_out}");
    assert!(python_out.contains("ruff check ."), "{python_out}");

    let go_temp = tempfile::tempdir().unwrap();
    copy_dir(Path::new("tests/fixtures/go-config"), go_temp.path());

    let init = Command::new(bin())
        .arg("init")
        .current_dir(go_temp.path())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let pack = Command::new(bin())
        .args(["pack", "."])
        .current_dir(go_temp.path())
        .output()
        .unwrap();
    assert!(
        pack.status.success(),
        "{}",
        String::from_utf8_lossy(&pack.stderr)
    );

    let go_context = Command::new(bin())
        .args(["context", "fix login route", "--budget", "2500"])
        .current_dir(go_temp.path())
        .output()
        .unwrap();
    assert!(
        go_context.status.success(),
        "{}",
        String::from_utf8_lossy(&go_context.stderr)
    );
    let go_out = String::from_utf8_lossy(&go_context.stdout);
    assert!(go_out.contains("Gin"), "{go_out}");
    assert!(go_out.contains("Chi"), "{go_out}");
    assert!(go_out.contains("go test ./..."), "{go_out}");
    assert!(go_out.contains("go build ./..."), "{go_out}");
}
