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
    assert!(context_out.contains("vitest"), "{context_out}");

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
}
