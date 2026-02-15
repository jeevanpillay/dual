/// E2E integration tests for Dual.
///
/// These tests require Docker and tmux to be available on the system.
/// Run with: `cargo test --test e2e`
///
/// Tests are marked `#[ignore]` so they don't run during `cargo test` (which runs
/// unit tests only). Run ignored tests with: `cargo test --test e2e -- --ignored`
/// Or run all tests including ignored: `cargo test --test e2e -- --include-ignored`
mod fixtures;
mod harness;

use std::process::Command;

// ─── Clone Tests ───────────────────────────────────────────────────────────

#[test]
fn clone_creates_workspace() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();

    let clone_dir =
        dual::clone::clone_workspace(&ws_root, "test-app", &repo_dir.to_string_lossy(), "main")
            .expect("clone should succeed");

    assert!(clone_dir.exists());
    assert!(clone_dir.join(".git").exists());
    assert!(clone_dir.join("package.json").exists());
    assert!(clone_dir.join("server.js").exists());
}

#[test]
fn clone_filesystem_layout() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();

    let clone_dir =
        dual::clone::clone_workspace(&ws_root, "test-app", &repo_dir.to_string_lossy(), "main")
            .expect("clone should succeed");

    // Layout: {workspace_root}/test-app/main/
    assert_eq!(clone_dir, workspace_root.join("test-app").join("main"));
}

#[test]
fn clone_idempotent() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let url = repo_dir.to_string_lossy().to_string();

    let dir1 = dual::clone::clone_workspace(&ws_root, "test-app", &url, "main")
        .expect("first clone should succeed");
    let dir2 = dual::clone::clone_workspace(&ws_root, "test-app", &url, "main")
        .expect("second clone should succeed (idempotent)");

    assert_eq!(dir1, dir2);
}

// ─── Container Tests (require Docker) ──────────────────────────────────────

#[test]
#[ignore] // Requires Docker
fn container_lifecycle() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let url = repo_dir.to_string_lossy().to_string();

    // Clone workspace first (container needs bind mount target)
    dual::clone::clone_workspace(&ws_root, "test-app", &url, "main").expect("clone should succeed");

    // Use test-prefixed container name
    let container_name = f.container_name();
    f.register_container(container_name.clone());

    // Create container with explicit name
    let workspace_dir = dual::config::workspace_dir(&ws_root, "test-app", "main");
    let args = dual::container::build_create_args(
        &container_name,
        &workspace_dir,
        "node:20",
        &std::collections::HashMap::new(),
        &["node_modules".to_string()],
    );
    let output = Command::new("docker").args(&args).output().unwrap();
    assert!(
        output.status.success(),
        "docker create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Status should be Stopped (created but not started)
    assert_eq!(
        dual::container::status(&container_name),
        dual::container::ContainerStatus::Stopped
    );

    // Start
    dual::container::start(&container_name).expect("start should succeed");
    assert_eq!(
        dual::container::status(&container_name),
        dual::container::ContainerStatus::Running
    );

    // Stop
    dual::container::stop(&container_name).expect("stop should succeed");
    assert_eq!(
        dual::container::status(&container_name),
        dual::container::ContainerStatus::Stopped
    );

    // Destroy (remove)
    dual::container::destroy(&container_name).expect("destroy should succeed");
    assert_eq!(
        dual::container::status(&container_name),
        dual::container::ContainerStatus::Missing
    );
}

#[test]
#[ignore] // Requires Docker
fn container_exec_exit_codes() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let url = repo_dir.to_string_lossy().to_string();

    dual::clone::clone_workspace(&ws_root, "test-app", &url, "main").expect("clone should succeed");

    let container_name = f.container_name();
    f.register_container(container_name.clone());

    let workspace_dir = dual::config::workspace_dir(&ws_root, "test-app", "main");
    let args = dual::container::build_create_args(
        &container_name,
        &workspace_dir,
        "node:20",
        &std::collections::HashMap::new(),
        &["node_modules".to_string()],
    );
    let output = Command::new("docker").args(&args).output().unwrap();
    assert!(output.status.success());

    dual::container::start(&container_name).expect("start should succeed");

    // Exit code 0 (success)
    let code = dual::container::exec(&container_name, &["true"], false).unwrap();
    assert_eq!(code, 0);

    // Exit code 1 (failure)
    let code = dual::container::exec(&container_name, &["false"], false).unwrap();
    assert_eq!(code, 1);

    // Exit code 42 (arbitrary)
    let code = dual::container::exec(&container_name, &["sh", "-c", "exit 42"], false).unwrap();
    assert_eq!(code, 42);
}

#[test]
#[ignore] // Requires Docker
fn bind_mount_host_to_container() {
    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let repo_dir = fixtures::create_fixture_repo(&temp);
    let workspace_root = temp.join("workspaces");

    let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
    let ws_root = state.workspace_root();
    let url = repo_dir.to_string_lossy().to_string();

    let clone_dir = dual::clone::clone_workspace(&ws_root, "test-app", &url, "main")
        .expect("clone should succeed");

    let container_name = f.container_name();
    f.register_container(container_name.clone());

    let args = dual::container::build_create_args(
        &container_name,
        &clone_dir,
        "node:20",
        &std::collections::HashMap::new(),
        &["node_modules".to_string()],
    );
    let output = Command::new("docker").args(&args).output().unwrap();
    assert!(output.status.success());

    dual::container::start(&container_name).expect("start should succeed");

    // Write a file on the host
    let test_content = format!("dual-bind-mount-test-{}", f.id);
    std::fs::write(clone_dir.join("bind-test.txt"), &test_content).unwrap();

    // Read the file from inside the container
    let output = Command::new("docker")
        .args(["exec", &container_name, "cat", "/workspace/bind-test.txt"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let container_content = String::from_utf8_lossy(&output.stdout);
    assert_eq!(container_content.trim(), test_content);
}

#[test]
#[ignore] // Requires Docker
fn network_isolation_same_port() {
    let mut f1 = harness::TestFixture::new();
    let mut f2 = harness::TestFixture::new();

    // Set up fixture repos and workspaces for both
    let temp1 = f1.temp_dir();
    let repo1 = fixtures::create_fixture_repo(&temp1);
    let ws_root1 = temp1.join("workspaces");
    let state1 = fixtures::fixture_state(&ws_root1, &repo1, "test-app-1", "main");
    let root1 = state1.workspace_root();
    let clone1 =
        dual::clone::clone_workspace(&root1, "test-app-1", &repo1.to_string_lossy(), "main")
            .unwrap();

    let temp2 = f2.temp_dir();
    let repo2 = fixtures::create_fixture_repo(&temp2);
    let ws_root2 = temp2.join("workspaces");
    let state2 = fixtures::fixture_state(&ws_root2, &repo2, "test-app-2", "main");
    let root2 = state2.workspace_root();
    let clone2 =
        dual::clone::clone_workspace(&root2, "test-app-2", &repo2.to_string_lossy(), "main")
            .unwrap();

    // Create and start both containers
    let name1 = f1.container_name();
    let name2 = f2.container_name();
    f1.register_container(name1.clone());
    f2.register_container(name2.clone());

    let empty_env = std::collections::HashMap::new();
    let default_vols = ["node_modules".to_string()];
    let args1 =
        dual::container::build_create_args(&name1, &clone1, "node:20", &empty_env, &default_vols);
    let args2 =
        dual::container::build_create_args(&name2, &clone2, "node:20", &empty_env, &default_vols);

    let out1 = Command::new("docker").args(&args1).output().unwrap();
    let out2 = Command::new("docker").args(&args2).output().unwrap();
    assert!(out1.status.success());
    assert!(out2.status.success());

    dual::container::start(&name1).unwrap();
    dual::container::start(&name2).unwrap();

    // Both containers should be running
    assert_eq!(
        dual::container::status(&name1),
        dual::container::ContainerStatus::Running
    );
    assert_eq!(
        dual::container::status(&name2),
        dual::container::ContainerStatus::Running
    );

    // Start node server on :3000 in both containers (bind same port, different namespaces)
    let start1 = Command::new("docker")
        .args([
            "exec",
            "-d",
            &name1,
            "node",
            "-e",
            "require('http').createServer((q,s)=>{s.end('c1')}).listen(3000)",
        ])
        .output()
        .unwrap();
    let start2 = Command::new("docker")
        .args([
            "exec",
            "-d",
            &name2,
            "node",
            "-e",
            "require('http').createServer((q,s)=>{s.end('c2')}).listen(3000)",
        ])
        .output()
        .unwrap();
    assert!(start1.status.success());
    assert!(start2.status.success());

    // Give servers a moment to start
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Curl from inside each container to verify both are running on :3000
    let curl1 = Command::new("docker")
        .args(["exec", &name1, "curl", "-s", "http://localhost:3000"])
        .output()
        .unwrap();
    let curl2 = Command::new("docker")
        .args(["exec", &name2, "curl", "-s", "http://localhost:3000"])
        .output()
        .unwrap();

    let body1 = String::from_utf8_lossy(&curl1.stdout);
    let body2 = String::from_utf8_lossy(&curl2.stdout);

    assert_eq!(body1.trim(), "c1", "container 1 should serve 'c1'");
    assert_eq!(body2.trim(), "c2", "container 2 should serve 'c2'");
}

// ─── Tmux Tests (require tmux) ─────────────────────────────────────────────

#[test]
#[ignore] // Requires tmux
fn tmux_session_lifecycle() {
    if !dual::tmux::is_available() {
        eprintln!("tmux not available, skipping");
        return;
    }

    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let session_name = f.session_name();
    f.register_tmux_session(session_name.clone());

    // Create detached session
    dual::tmux::create_session(&session_name, &temp, None).expect("create should succeed");

    // Session should be alive
    assert!(dual::tmux::is_alive(&session_name));

    // Destroy session
    dual::tmux::destroy(&session_name).expect("destroy should succeed");

    // Session should no longer be alive
    assert!(!dual::tmux::is_alive(&session_name));
}

#[test]
#[ignore] // Requires tmux
fn tmux_send_keys() {
    if !dual::tmux::is_available() {
        eprintln!("tmux not available, skipping");
        return;
    }

    let mut f = harness::TestFixture::new();
    let temp = f.temp_dir();
    let session_name = f.session_name();
    f.register_tmux_session(session_name.clone());

    dual::tmux::create_session(&session_name, &temp, None).expect("create should succeed");

    // Send a command that creates a file
    let marker = format!("dual-tmux-test-{}", f.short_id);
    dual::tmux::send_keys(&session_name, &format!("echo '{}' > marker.txt", marker))
        .expect("send_keys should succeed");

    // Poll for marker file with timeout (tmux send-keys is async)
    let marker_path = temp.join("marker.txt");
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    while !marker_path.exists() {
        assert!(
            start.elapsed() < timeout,
            "marker file should exist (timed out after {timeout:?})"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let contents = std::fs::read_to_string(&marker_path).unwrap();
    assert_eq!(contents.trim(), marker);
}
