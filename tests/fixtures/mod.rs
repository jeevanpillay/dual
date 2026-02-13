use std::path::{Path, PathBuf};
use std::process::Command;

/// Create a minimal monorepo fixture as a local git repo.
///
/// The fixture contains:
/// - `package.json` with a start script
/// - `server.js` — minimal Node.js HTTP server on port 3000
/// - An initial git commit so `git clone --local` works
///
/// Returns the path to the fixture repo directory.
pub fn create_fixture_repo(parent_dir: &Path) -> PathBuf {
    let repo_dir = parent_dir.join("fixture-repo");
    std::fs::create_dir_all(&repo_dir).expect("failed to create fixture repo dir");

    // Initialize git repo
    let output = Command::new("git")
        .args(["init", "--initial-branch", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("git not found");
    assert!(output.status.success(), "git init failed");

    // Configure git user for commits
    let _ = Command::new("git")
        .args(["config", "user.email", "test@dual.dev"])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Dual Test"])
        .current_dir(&repo_dir)
        .output();

    // Create package.json
    let package_json = r#"{
  "name": "dual-test-fixture",
  "version": "0.0.1",
  "scripts": {
    "start": "node server.js",
    "dev": "node server.js"
  }
}
"#;
    std::fs::write(repo_dir.join("package.json"), package_json)
        .expect("failed to write package.json");

    // Create server.js — minimal HTTP server
    let server_js = r#"const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("dual-test-ok\n");
});
server.listen(3000, () => {
  console.log("listening on :3000");
});
"#;
    std::fs::write(repo_dir.join("server.js"), server_js).expect("failed to write server.js");

    // Git add and commit
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("git add failed");
    assert!(output.status.success(), "git add failed");

    let output = Command::new("git")
        .args(["commit", "-m", "initial fixture"])
        .current_dir(&repo_dir)
        .output()
        .expect("git commit failed");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    repo_dir
}

/// Generate a dual.toml config string pointing at a fixture repo.
///
/// The config sets workspace_root to the given directory and defines
/// a single repo using the fixture path as a local URL.
pub fn fixture_config_toml(
    workspace_root: &Path,
    fixture_repo: &Path,
    repo_name: &str,
    branch: &str,
    ports: &[u16],
) -> String {
    let ports_str: Vec<String> = ports.iter().map(|p| p.to_string()).collect();
    format!(
        r#"workspace_root = "{workspace_root}"

[[repos]]
name = "{repo_name}"
url = "{fixture_repo}"
branches = ["{branch}"]
ports = [{ports}]
"#,
        workspace_root = workspace_root.display(),
        repo_name = repo_name,
        fixture_repo = fixture_repo.display(),
        branch = branch,
        ports = ports_str.join(", "),
    )
}
