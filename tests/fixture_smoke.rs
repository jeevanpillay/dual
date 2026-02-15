mod fixtures;
mod harness;

use std::process::Command;

#[test]
fn fixture_repo_is_valid_git_repo() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);

    // Verify .git exists
    assert!(repo_dir.join(".git").exists());

    // Verify git status is clean
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo_dir)
        .output()
        .expect("git status failed");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty(), "repo has uncommitted changes");
}

#[test]
fn fixture_repo_contains_expected_files() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);

    assert!(repo_dir.join("package.json").exists());
    assert!(repo_dir.join("server.js").exists());
}

#[test]
fn fixture_repo_package_json_has_scripts() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);

    let contents = std::fs::read_to_string(repo_dir.join("package.json")).unwrap();
    assert!(contents.contains("\"start\""));
    assert!(contents.contains("\"dev\""));
    assert!(contents.contains("node server.js"));
}

#[test]
fn fixture_repo_cloneable_locally() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);

    // Clone to a new location using --local
    let clone_dir = temp.join("clone-test");
    let output = Command::new("git")
        .args([
            "clone",
            "--local",
            "-b",
            "main",
            &repo_dir.to_string_lossy(),
            &clone_dir.to_string_lossy(),
        ])
        .output()
        .expect("git clone failed");

    assert!(
        output.status.success(),
        "git clone --local failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(clone_dir.join(".git").exists());
    assert!(clone_dir.join("package.json").exists());
    assert!(clone_dir.join("server.js").exists());
}

#[test]
fn fixture_state_creates_valid_state() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");

    assert_eq!(state.all_workspaces().len(), 1);
    let ws = &state.all_workspaces()[0];
    assert_eq!(ws.repo, "test-app");
    assert_eq!(ws.branch, "main");
    assert_eq!(ws.url, repo_dir.to_string_lossy());
}

#[test]
fn fixture_state_workspace_dir_correct() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let ws_dir = dual::config::workspace_dir(&ws_root, "test-app", "main");
    assert!(ws_dir.starts_with(&workspace_root));
    assert!(ws_dir.ends_with("test-app/main"));
}

#[test]
fn fixture_state_with_clone_module() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let url = repo_dir.to_string_lossy().to_string();

    // Verify the URL is detected as local
    assert!(dual::clone::is_local_path(&url));

    // Clone using the dual clone module
    let clone_dir = dual::clone::clone_workspace(&ws_root, "test-app", &url, "main")
        .expect("clone should succeed");

    assert!(clone_dir.join(".git").exists());
    assert!(clone_dir.join("package.json").exists());
}
