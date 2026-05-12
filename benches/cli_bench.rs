use criterion::{criterion_group, criterion_main, Criterion};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

fn scoutpack_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_scoutpack") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary = manifest_dir.join("target/release/scoutpack");
    if !binary.exists() {
        let status = Command::new("cargo")
            .args(["build", "--release", "--bin", "scoutpack"])
            .current_dir(&manifest_dir)
            .status()
            .expect("run cargo build for benchmark binary");
        assert!(
            status.success(),
            "cargo build --release --bin scoutpack failed"
        );
    }
    binary
}

fn workspace() -> &'static PathBuf {
    static WORKSPACE: OnceLock<PathBuf> = OnceLock::new();
    WORKSPACE.get_or_init(|| {
        let root = env::temp_dir().join(format!("scoutpack-bench-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create benchmark temp root");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/nextjs-basic");
        let repo = root.join("nextjs-basic");
        copy_dir(&fixture, &repo);
        run_scoutpack(&repo, &["init"]);
        repo
    })
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create copied directory");
    for entry in fs::read_dir(src).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).expect("copy fixture file");
        }
    }
}

fn run_scoutpack(cwd: &Path, args: &[&str]) {
    let output = Command::new(scoutpack_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run scoutpack");
    assert!(
        output.status.success(),
        "scoutpack {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bench_cli(c: &mut Criterion) {
    let repo = workspace();
    run_scoutpack(repo, &["pack", "."]);

    c.bench_function("pack_nextjs_basic_incremental", |b| {
        b.iter(|| run_scoutpack(repo, &["pack", "."]));
    });

    c.bench_function("search_nextjs_basic_auth", |b| {
        b.iter(|| run_scoutpack(repo, &["search", "auth middleware", "--limit", "5"]));
    });

    c.bench_function("context_nextjs_basic_login", |b| {
        b.iter(|| {
            run_scoutpack(
                repo,
                &["context", "fix login redirect loop", "--budget", "2500"],
            )
        });
    });
}

criterion_group!(benches, bench_cli);
criterion_main!(benches);
