# devcontainer.json Fallback Reader — Implementation Plan

## Overview

Add devcontainer.json as a fallback configuration source when no `.dual.toml` exists. This gives Dual zero-config compatibility with the ~15% of repos that already have `.devcontainer/` directories, while keeping `.dual.toml` as the primary config for Dual-specific concerns.

Scope includes full spec-compatible parsing for the ~5 fields we read, plus adding `docker build` support to handle `build.dockerfile`.

## Current State Analysis

- `RepoHints` defined at `src/config.rs:18-46` — 7 fields: `image`, `ports`, `setup`, `env`, `extra_commands`, `anonymous_volumes`, `shared`
- `load_hints()` at `src/config.rs:77-89` — reads `.dual.toml`, returns `RepoHints::default()` if missing
- `container::create()` at `src/container.rs:22-50` — accepts `image: &str`, no build support
- `cmd_launch()` at `src/main.rs:364` — loads hints, creates container, runs setup
- No `serde_json` dependency — only `toml` + `serde` in `Cargo.toml:17,21`

### Key Discoveries:
- `load_hints()` already returns defaults when `.dual.toml` is missing (`config.rs:80-82`) — the fallback insertion point is clean
- Container creation accepts image as a plain string (`container.rs:25`) — we can pass a built image tag the same way
- Serde `#[serde(default)]` and `#[serde(rename_all)]` patterns are well-established in the codebase
- Test patterns use `parse_hints()` string-based parsing and temp dir filesystem tests

## Desired End State

When a workspace has no `.dual.toml` but has a `.devcontainer/devcontainer.json` (or root `devcontainer.json`):
1. Dual reads the devcontainer.json and maps supported fields to `RepoHints`
2. If `build.dockerfile` is specified, Dual builds the image via `docker build` before creating the container
3. If `image` is specified, it maps directly to `hints.image`
4. `forwardPorts`, `containerEnv`, `postCreateCommand` map to `ports`, `env`, `setup`
5. Volume-type mounts targeting `/workspace/*` map to `anonymous_volumes`
6. `.dual.toml` always takes priority — devcontainer.json is only read as fallback

### Verification:
- `cargo test` passes with new unit tests covering all devcontainer.json format variants
- `cargo clippy` clean
- Manual: clone a repo with `.devcontainer/devcontainer.json`, run `dual launch`, verify container uses the devcontainer-specified image/ports/env

## What We're NOT Doing

- **Dev container features** — No OCI artifact pulling, no `install.sh` execution. Repos needing features should use `build.dockerfile` pointing to their own Dockerfile.
- **Lifecycle hooks beyond `postCreateCommand`** — No `postStartCommand`, `postAttachCommand`. Only `postCreateCommand` maps to `setup`.
- **Variable substitution** — No `${localEnv:VAR}` or `${containerWorkspaceFolder}` expansion in `containerEnv`. Literal values only.
- **Docker Compose** — No `dockerComposeFile` support. Conflicts with Dual's one-container-per-workspace model.
- **IDE customizations** — All `customizations.*` fields ignored.
- **Container user/security** — `remoteUser`, `containerUser`, `privileged`, `capAdd` ignored. Dual controls these.

## Implementation Approach

Three phases, each independently testable:

1. **Parser** — New `src/devcontainer.rs` module with types and parsing logic
2. **Config Integration** — Wire parser into `load_hints()` fallback chain, add `DockerfileBuild` to `RepoHints`
3. **Docker Build** — Add `build_image()` to `container.rs`, modify `cmd_launch()` flow

---

## Phase 1: Parser & Types

### Overview
Create `src/devcontainer.rs` with the `DevcontainerJson` struct and multi-format enum types. Add `serde_json` dependency.

### Changes Required:

#### 1. Add `serde_json` dependency
**File**: `Cargo.toml`
**Changes**: Add `serde_json` to `[dependencies]`

```toml
[dependencies]
# ... existing deps ...
serde_json = "1"
```

#### 2. Register new module
**File**: `src/lib.rs`
**Changes**: Add `pub mod devcontainer;`

```rust
pub mod backend;
pub mod cli;
pub mod clone;
pub mod config;
pub mod container;
pub mod devcontainer;  // new
pub mod proxy;
pub mod shared;
pub mod shell;
pub mod state;
pub mod tmux_backend;
pub mod tui;
```

#### 3. Create devcontainer parser module
**File**: `src/devcontainer.rs` (new)
**Changes**: Full module with types, parsing, and path resolution

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::RepoHints;

/// Subset of devcontainer.json fields that Dual consumes.
/// See: https://containers.dev/implementors/json_reference/
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevcontainerJson {
    /// Docker image to use (mutually exclusive with `build`)
    pub image: Option<String>,

    /// Build configuration for Dockerfile-based images
    pub build: Option<BuildConfig>,

    /// Ports to forward from the container
    pub forward_ports: Option<Vec<PortSpec>>,

    /// Environment variables for the container
    pub container_env: Option<HashMap<String, String>>,

    /// Command to run after container creation
    pub post_create_command: Option<CommandSpec>,

    /// Mount configurations
    pub mounts: Option<Vec<MountSpec>>,
}

/// Build configuration from devcontainer.json.
#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    /// Path to Dockerfile (relative to devcontainer.json location)
    pub dockerfile: Option<String>,

    /// Build context path (relative to devcontainer.json location)
    pub context: Option<String>,

    /// Docker build arguments
    pub args: Option<HashMap<String, String>>,

    /// Build target stage for multi-stage builds
    pub target: Option<String>,
}

/// Port specification — devcontainer allows integers or strings.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PortSpec {
    Number(u16),
    String(String),
}

impl PortSpec {
    /// Convert to u16, parsing strings as integers.
    pub fn to_port(&self) -> Option<u16> {
        match self {
            PortSpec::Number(n) => Some(*n),
            PortSpec::String(s) => s.parse::<u16>().ok(),
        }
    }
}

/// Command specification — devcontainer allows string, array, or object forms.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    /// Single shell command: "pnpm install"
    String(String),
    /// Exec form: ["pnpm", "install"]
    Array(Vec<String>),
    /// Parallel commands: {"install": "pnpm install", "build": "pnpm build"}
    Object(HashMap<String, StringOrArray>),
}

impl CommandSpec {
    /// Flatten to a single shell command string.
    /// - String: returned as-is
    /// - Array: joined with spaces
    /// - Object: values joined with " && "
    pub fn to_shell_command(&self) -> String {
        match self {
            CommandSpec::String(s) => s.clone(),
            CommandSpec::Array(arr) => arr.join(" "),
            CommandSpec::Object(map) => {
                let mut commands: Vec<&str> = map
                    .values()
                    .map(|v| match v {
                        StringOrArray::String(s) => s.as_str(),
                        StringOrArray::Array(arr) => {
                            // Can't return a joined string as &str easily,
                            // so we'll handle this differently below
                            ""
                        }
                    })
                    .collect();
                // Re-do with owned strings to handle array values
                let commands: Vec<String> = map
                    .values()
                    .map(|v| match v {
                        StringOrArray::String(s) => s.clone(),
                        StringOrArray::Array(arr) => arr.join(" "),
                    })
                    .collect();
                commands.join(" && ")
            }
        }
    }
}

/// Value in a command object — can be string or array.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrArray {
    String(String),
    Array(Vec<String>),
}

/// Mount specification — devcontainer allows string or object forms.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MountSpec {
    /// Docker mount syntax: "type=volume,target=/workspace/node_modules"
    String(String),
    /// Structured mount object
    Object(MountObject),
}

/// Structured mount configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountObject {
    #[serde(rename = "type")]
    pub mount_type: Option<String>,
    pub source: Option<String>,
    pub target: String,
}

const WORKSPACE_PREFIX: &str = "/workspace/";

impl MountSpec {
    /// Extract anonymous volume path if this is a volume mount targeting /workspace/*.
    /// Returns the relative path (e.g., "node_modules" from "/workspace/node_modules").
    pub fn as_anonymous_volume(&self) -> Option<String> {
        match self {
            MountSpec::String(s) => {
                // Parse "type=volume,target=/workspace/node_modules" format
                let mut mount_type = None;
                let mut target = None;
                let mut has_source = false;
                for part in s.split(',') {
                    let (key, value) = part.split_once('=')?;
                    match key.trim() {
                        "type" => mount_type = Some(value.trim()),
                        "target" | "dst" | "destination" => target = Some(value.trim()),
                        "source" | "src" => has_source = true,
                        _ => {}
                    }
                }
                if mount_type == Some("volume") && !has_source {
                    target
                        .filter(|t| t.starts_with(WORKSPACE_PREFIX))
                        .map(|t| t[WORKSPACE_PREFIX.len()..].to_string())
                }
                None // String format parsing is best-effort
            }
            MountSpec::Object(obj) => {
                if obj.mount_type.as_deref() == Some("volume") && obj.source.is_none() {
                    if obj.target.starts_with(WORKSPACE_PREFIX) {
                        return Some(obj.target[WORKSPACE_PREFIX.len()..].to_string());
                    }
                }
                None
            }
        }
    }
}

/// Resolve the path to devcontainer.json in a workspace directory.
///
/// Checks in order:
/// 1. `.devcontainer/devcontainer.json`
/// 2. `.devcontainer.json` (root)
///
/// Returns None if neither exists.
pub fn find_devcontainer_json(workspace_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        workspace_dir.join(".devcontainer").join("devcontainer.json"),
        workspace_dir.join(".devcontainer.json"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Parse a devcontainer.json string into DevcontainerJson.
pub fn parse_devcontainer(json_str: &str) -> Result<DevcontainerJson, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Load and parse devcontainer.json from a workspace directory.
/// Returns None if no devcontainer.json exists.
pub fn load_devcontainer(workspace_dir: &Path) -> Option<DevcontainerJson> {
    let path = find_devcontainer_json(workspace_dir)?;
    let contents = std::fs::read_to_string(&path).ok()?;
    parse_devcontainer(&contents).ok()
}

/// Convert DevcontainerJson fields to RepoHints.
///
/// Maps:
/// - `image` → `hints.image`
/// - `build` → `hints.dockerfile`
/// - `forwardPorts` → `hints.ports`
/// - `containerEnv` → `hints.env`
/// - `postCreateCommand` → `hints.setup`
/// - `mounts` (volume type, /workspace/* target) → `hints.anonymous_volumes`
pub fn to_repo_hints(dc: &DevcontainerJson, devcontainer_dir: &Path) -> RepoHints {
    let mut hints = RepoHints::default();

    // Image (mutually exclusive with build)
    if let Some(ref image) = dc.image {
        hints.image = image.clone();
    }

    // Build → DockerfileBuild
    if let Some(ref build) = dc.build {
        if build.dockerfile.is_some() {
            hints.dockerfile = Some(crate::config::DockerfileBuild {
                path: build.dockerfile.clone().unwrap_or_else(|| "Dockerfile".to_string()),
                context: build.context.clone().unwrap_or_else(|| ".".to_string()),
                args: build.args.clone().unwrap_or_default(),
                target: build.target.clone(),
                // Resolve paths relative to devcontainer.json location
                base_dir: Some(devcontainer_dir.to_path_buf()),
            });
        }
    }

    // Ports
    if let Some(ref ports) = dc.forward_ports {
        hints.ports = ports.iter().filter_map(|p| p.to_port()).collect();
    }

    // Environment variables
    if let Some(ref env) = dc.container_env {
        hints.env = env.clone();
    }

    // Setup command
    if let Some(ref cmd) = dc.post_create_command {
        let shell_cmd = cmd.to_shell_command();
        if !shell_cmd.is_empty() {
            hints.setup = Some(shell_cmd);
        }
    }

    // Anonymous volumes from mounts
    if let Some(ref mounts) = dc.mounts {
        let extra_volumes: Vec<String> = mounts
            .iter()
            .filter_map(|m| m.as_anonymous_volume())
            .collect();
        if !extra_volumes.is_empty() {
            // Merge with defaults, dedup
            for vol in extra_volumes {
                if !hints.anonymous_volumes.contains(&vol) {
                    hints.anonymous_volumes.push(vol);
                }
            }
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_image() {
        let json = r#"{"image": "node:20"}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.image.as_deref(), Some("node:20"));
        assert!(dc.build.is_none());
    }

    #[test]
    fn parse_with_build() {
        let json = r#"{
            "build": {
                "dockerfile": "Dockerfile",
                "context": "..",
                "args": {"NODE_VERSION": "20"},
                "target": "development"
            }
        }"#;
        let dc = parse_devcontainer(json).unwrap();
        assert!(dc.image.is_none());
        let build = dc.build.unwrap();
        assert_eq!(build.dockerfile.as_deref(), Some("Dockerfile"));
        assert_eq!(build.context.as_deref(), Some(".."));
        assert_eq!(build.args.unwrap().get("NODE_VERSION").unwrap(), "20");
        assert_eq!(build.target.as_deref(), Some("development"));
    }

    #[test]
    fn parse_forward_ports_integers() {
        let json = r#"{"forwardPorts": [3000, 8080]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc.forward_ports.unwrap().iter().filter_map(|p| p.to_port()).collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_strings() {
        let json = r#"{"forwardPorts": ["3000", "8080"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc.forward_ports.unwrap().iter().filter_map(|p| p.to_port()).collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_mixed() {
        let json = r#"{"forwardPorts": [3000, "8080"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc.forward_ports.unwrap().iter().filter_map(|p| p.to_port()).collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_invalid_string_skipped() {
        let json = r#"{"forwardPorts": [3000, "not-a-port"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc.forward_ports.unwrap().iter().filter_map(|p| p.to_port()).collect();
        assert_eq!(ports, vec![3000]);
    }

    #[test]
    fn parse_container_env() {
        let json = r#"{"containerEnv": {"NODE_ENV": "development", "DEBUG": "true"}}"#;
        let dc = parse_devcontainer(json).unwrap();
        let env = dc.container_env.unwrap();
        assert_eq!(env.get("NODE_ENV").unwrap(), "development");
        assert_eq!(env.get("DEBUG").unwrap(), "true");
    }

    #[test]
    fn parse_post_create_command_string() {
        let json = r#"{"postCreateCommand": "pnpm install"}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.post_create_command.unwrap().to_shell_command(), "pnpm install");
    }

    #[test]
    fn parse_post_create_command_array() {
        let json = r#"{"postCreateCommand": ["pnpm", "install"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.post_create_command.unwrap().to_shell_command(), "pnpm install");
    }

    #[test]
    fn parse_post_create_command_object() {
        let json = r#"{"postCreateCommand": {"install": "pnpm install", "build": "pnpm build"}}"#;
        let dc = parse_devcontainer(json).unwrap();
        let cmd = dc.post_create_command.unwrap().to_shell_command();
        // Object order is non-deterministic, but both commands should be present
        assert!(cmd.contains("pnpm install"));
        assert!(cmd.contains("pnpm build"));
        assert!(cmd.contains(" && "));
    }

    #[test]
    fn parse_post_create_command_object_with_array_value() {
        let json = r#"{"postCreateCommand": {"install": ["pnpm", "install"]}}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.post_create_command.unwrap().to_shell_command(), "pnpm install");
    }

    #[test]
    fn parse_mount_object_volume() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: None,
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), Some("node_modules".to_string()));
    }

    #[test]
    fn parse_mount_object_with_source_not_anonymous() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: Some("my-vol".to_string()),
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_mount_object_bind_not_anonymous() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("bind".to_string()),
            source: None,
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_mount_object_non_workspace_target() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: None,
            target: "/data/cache".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_unknown_fields_ignored() {
        let json = r#"{
            "image": "node:20",
            "customizations": {"vscode": {"extensions": ["ms-python.python"]}},
            "remoteUser": "vscode",
            "features": {"ghcr.io/devcontainers/features/node:1": {}}
        }"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.image.as_deref(), Some("node:20"));
    }

    #[test]
    fn parse_empty_object() {
        let json = r#"{}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert!(dc.image.is_none());
        assert!(dc.build.is_none());
        assert!(dc.forward_ports.is_none());
    }

    #[test]
    fn to_repo_hints_image_only() {
        let dc = DevcontainerJson {
            image: Some("python:3.12".to_string()),
            build: None,
            forward_ports: None,
            container_env: None,
            post_create_command: None,
            mounts: None,
        };
        let hints = to_repo_hints(&dc, Path::new("."));
        assert_eq!(hints.image, "python:3.12");
        assert!(hints.dockerfile.is_none());
    }

    #[test]
    fn to_repo_hints_full() {
        let dc = DevcontainerJson {
            image: Some("node:20".to_string()),
            build: None,
            forward_ports: Some(vec![PortSpec::Number(3000), PortSpec::String("8080".to_string())]),
            container_env: Some(HashMap::from([("NODE_ENV".to_string(), "dev".to_string())])),
            post_create_command: Some(CommandSpec::String("pnpm install".to_string())),
            mounts: None,
        };
        let hints = to_repo_hints(&dc, Path::new("."));
        assert_eq!(hints.image, "node:20");
        assert_eq!(hints.ports, vec![3000, 8080]);
        assert_eq!(hints.env.get("NODE_ENV").unwrap(), "dev");
        assert_eq!(hints.setup.as_deref(), Some("pnpm install"));
    }

    #[test]
    fn find_devcontainer_json_in_subdir() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-find");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "node:20"}"#,
        ).unwrap();

        let found = find_devcontainer_json(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".devcontainer/devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_json_at_root() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-root");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".devcontainer.json"), r#"{"image": "node:20"}"#).unwrap();

        let found = find_devcontainer_json(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_json_missing() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(find_devcontainer_json(&dir).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_prefers_subdir_over_root() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-prefer");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "python:3.12"}"#,
        ).unwrap();
        std::fs::write(dir.join(".devcontainer.json"), r#"{"image": "node:20"}"#).unwrap();

        let found = find_devcontainer_json(&dir).unwrap();
        assert!(found.ends_with(".devcontainer/devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles with new `serde_json` dependency
- [x] `cargo test` passes all new tests in `devcontainer.rs`
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: Config Integration

### Overview
Add `DockerfileBuild` struct to `RepoHints`, modify `load_hints()` to fall back to devcontainer.json, and wire up the conversion.

### Changes Required:

#### 1. Add `DockerfileBuild` to config types
**File**: `src/config.rs`
**Changes**: Add new struct and field to `RepoHints`

After `SharedConfig` (line 15), add:

```rust
/// Dockerfile build configuration for building images from source.
/// Used when devcontainer.json specifies `build.dockerfile` or when
/// configured directly in .dual.toml.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DockerfileBuild {
    /// Path to the Dockerfile (relative to base_dir or workspace root)
    pub path: String,

    /// Build context directory (relative to base_dir or workspace root)
    #[serde(default = "default_build_context")]
    pub context: String,

    /// Docker build arguments (--build-arg)
    #[serde(default)]
    pub args: HashMap<String, String>,

    /// Target build stage for multi-stage builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Base directory for resolving relative paths (set by devcontainer loader).
    /// When loaded from .dual.toml, this is None and paths are relative to workspace root.
    #[serde(skip)]
    pub base_dir: Option<PathBuf>,
}

fn default_build_context() -> String {
    ".".to_string()
}
```

Add field to `RepoHints` struct (after `shared`, line 45):

```rust
    /// Dockerfile build config — if set, build image instead of pulling.
    /// Can be set via .dual.toml [dockerfile] section or from devcontainer.json build field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<DockerfileBuild>,
```

Update `Default` impl (after line 65):

```rust
            dockerfile: None,
```

#### 2. Modify `load_hints()` to add devcontainer fallback
**File**: `src/config.rs`
**Changes**: Insert devcontainer.json fallback between "file doesn't exist" and "return default"

Replace `load_hints()` (lines 77-89):

```rust
/// Load RepoHints from a workspace directory.
///
/// Priority order:
/// 1. `.dual.toml` — Dual-native config (always takes priority)
/// 2. `.devcontainer/devcontainer.json` or `.devcontainer.json` — fallback
/// 3. Default hints (node:20, no ports, etc.)
pub fn load_hints(workspace_dir: &Path) -> Result<RepoHints, HintsError> {
    let path = workspace_dir.join(HINTS_FILENAME);

    // 1. .dual.toml takes priority
    if path.exists() {
        let contents =
            std::fs::read_to_string(&path).map_err(|e| HintsError::ReadError(path.clone(), e))?;
        let hints: RepoHints =
            toml::from_str(&contents).map_err(|e| HintsError::ParseError(path, e))?;
        return Ok(hints);
    }

    // 2. Fall back to devcontainer.json
    if let Some(hints) = crate::devcontainer::load_devcontainer_as_hints(workspace_dir) {
        return Ok(hints);
    }

    // 3. Default
    Ok(RepoHints::default())
}
```

#### 3. Add convenience function to devcontainer module
**File**: `src/devcontainer.rs`
**Changes**: Add `load_devcontainer_as_hints()` that combines load + convert

```rust
/// Load devcontainer.json from a workspace and convert to RepoHints.
/// Returns None if no devcontainer.json exists or it fails to parse.
pub fn load_devcontainer_as_hints(workspace_dir: &Path) -> Option<RepoHints> {
    let path = find_devcontainer_json(workspace_dir)?;
    let devcontainer_dir = path.parent().unwrap_or(workspace_dir);
    let contents = std::fs::read_to_string(&path).ok()?;
    let dc = parse_devcontainer(&contents).ok()?;
    Some(to_repo_hints(&dc, devcontainer_dir))
}
```

#### 4. Update test fixture helper
**File**: `tests/fixtures/mod.rs`
**Changes**: Add `dockerfile` field to `create_fixture_hints`

```rust
pub fn create_fixture_hints(repo_dir: &Path, ports: &[u16]) {
    let hints = dual::config::RepoHints {
        image: "node:20".to_string(),
        ports: ports.to_vec(),
        setup: None,
        env: std::collections::HashMap::new(),
        extra_commands: Vec::new(),
        anonymous_volumes: vec!["node_modules".to_string()],
        shared: None,
        dockerfile: None,  // new field
    };
    dual::config::write_hints(repo_dir, &hints).expect("failed to write fixture hints");
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles
- [x] `cargo test` — all existing config tests still pass
- [x] `cargo test` — new integration tests pass:
  - `load_hints` returns devcontainer config when no `.dual.toml` exists
  - `load_hints` prefers `.dual.toml` over devcontainer.json when both exist
  - `load_hints` returns defaults when neither exists
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean

#### Manual Verification:
- [x] Create a test directory with `.devcontainer/devcontainer.json` containing `{"image": "python:3.12", "forwardPorts": [8000]}` and verify `load_hints()` returns the correct image and ports

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Docker Build Support

### Overview
Add `build_image()` function to `container.rs` and modify `cmd_launch()` in `main.rs` to build from Dockerfile before creating the container.

### Changes Required:

#### 1. Add `build_image()` to container module
**File**: `src/container.rs`
**Changes**: New function and arg builder

After `exec_setup()` (line 179), add:

```rust
/// Build a Docker image from a Dockerfile.
///
/// Returns the image tag on success.
pub fn build_image(
    tag: &str,
    workspace_dir: &Path,
    build: &crate::config::DockerfileBuild,
) -> Result<String, ContainerError> {
    let args = build_image_args(tag, workspace_dir, build);
    let output = Command::new("docker")
        .args(&args)
        .output()
        .map_err(|e| ContainerError::DockerNotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ContainerError::Failed {
            operation: "build".to_string(),
            name: tag.to_string(),
            stderr,
        });
    }

    Ok(tag.to_string())
}

/// Build docker build arguments (public for testing).
pub fn build_image_args(
    tag: &str,
    workspace_dir: &Path,
    build: &crate::config::DockerfileBuild,
) -> Vec<String> {
    // Resolve paths relative to base_dir (devcontainer.json location) or workspace root
    let base = build.base_dir.as_deref().unwrap_or(workspace_dir);

    let dockerfile_path = base.join(&build.path);
    let context_path = base.join(&build.context);

    let mut args = vec![
        "build".to_string(),
        "-t".to_string(),
        tag.to_string(),
        "-f".to_string(),
        dockerfile_path.display().to_string(),
    ];

    // Build arguments
    for (key, value) in &build.args {
        args.push("--build-arg".to_string());
        args.push(format!("{key}={value}"));
    }

    // Target stage
    if let Some(ref target) = build.target {
        args.push("--target".to_string());
        args.push(target.clone());
    }

    // Build context (last argument)
    args.push(context_path.display().to_string());

    args
}
```

#### 2. Modify `cmd_launch()` to build before create
**File**: `src/main.rs`
**Changes**: Insert build step between hints loading and container creation

At `main.rs:399-409` (container creation), modify to:

```rust
container::ContainerStatus::Missing => {
    // Build image from Dockerfile if configured
    let effective_image = if let Some(ref build) = hints.dockerfile {
        let image_tag = format!("dual-build-{container_name}");
        info!("Building image from Dockerfile...");
        match container::build_image(&image_tag, &workspace_dir, build) {
            Ok(tag) => tag,
            Err(e) => {
                error!("docker build failed: {e}");
                return 1;
            }
        }
    } else {
        hints.image.clone()
    };

    info!("Creating container {container_name}...");
    if let Err(e) = container::create(
        &container_name,
        &workspace_dir,
        &effective_image,
        &hints.env,
        &hints.anonymous_volumes,
    ) {
        error!("container create failed: {e}");
        return 1;
    }
    if let Err(e) = container::start(&container_name) {
        error!("container start failed: {e}");
        return 1;
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles
- [x] `cargo test` — new `build_image_args` tests pass:
  - Correct `-t`, `-f`, context path ordering
  - Build args passed as `--build-arg KEY=VALUE`
  - Target stage passed as `--target`
  - Paths resolved relative to `base_dir` when set
  - Paths resolved relative to workspace root when `base_dir` is None
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean

#### Manual Verification:
- [x] Create a workspace with a Dockerfile, add `devcontainer.json` with `{"build": {"dockerfile": "Dockerfile"}}`, run `dual launch`, verify image is built and container starts
- [x] Verify a workspace with `{"image": "node:20"}` still works (no build step)
- [x] Verify a workspace with `.dual.toml` ignores devcontainer.json entirely

**Implementation Note**: After completing this phase and all verification passes, the feature is complete.

---

## Testing Strategy

### Unit Tests:
- **devcontainer.rs**: All format variants for each field (covered in Phase 1 inline tests)
- **config.rs**: `load_hints()` fallback chain — `.dual.toml` priority, devcontainer fallback, defaults
- **container.rs**: `build_image_args()` construction correctness

### Integration Tests:
- Create temp directory with `.devcontainer/devcontainer.json` → verify `load_hints()` returns correct hints
- Create temp directory with both `.dual.toml` and devcontainer.json → verify `.dual.toml` wins
- `build_image_args()` with various `DockerfileBuild` configurations

### Edge Cases to Test:
- Empty devcontainer.json `{}`
- devcontainer.json with only unsupported fields (should return defaults)
- Invalid JSON (should fall through to defaults, not error)
- Invalid port strings in `forwardPorts` (should be silently skipped)
- `postCreateCommand` with empty object `{}`
- `build` without `dockerfile` field (should not set `hints.dockerfile`)

## Performance Considerations

- JSON parse adds ~1ms to `load_hints()` in the fallback path — negligible
- `docker build` can take 30s-5min depending on Dockerfile complexity — acceptable since it only runs on first container creation (same as pulling large images)
- No new runtime dependencies beyond `serde_json`

## References

- Research: `thoughts/shared/research/2026-02-16-web-analysis-devcontainer-adoption.md`
- Dev Container JSON Reference: https://containers.dev/implementors/json_reference/
- Current config module: `src/config.rs`
- Container lifecycle: `src/container.rs`
- Launch flow: `src/main.rs:364-432`
