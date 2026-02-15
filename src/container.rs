use std::path::Path;
use std::process::Command;

const WORKSPACE_MOUNT: &str = "/workspace";

/// Container status.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Missing,
}

/// Create a new Docker container for a workspace.
///
/// - Bind mounts workspace dir to /workspace
/// - Anonymous volume for node_modules isolation
/// - Sets working directory to /workspace
/// - Uses bridge network (default) for network namespace isolation
pub fn create(name: &str, workspace_dir: &Path, image: &str) -> Result<String, ContainerError> {
    let output = Command::new("docker")
        .args(build_create_args(name, workspace_dir, image))
        .output()
        .map_err(|e| ContainerError::DockerNotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ContainerError::Failed {
            operation: "create".to_string(),
            name: name.to_string(),
            stderr,
        });
    }

    Ok(name.to_string())
}

/// Start an existing container.
pub fn start(name: &str) -> Result<(), ContainerError> {
    docker_simple("start", name)
}

/// Stop a running container.
pub fn stop(name: &str) -> Result<(), ContainerError> {
    docker_simple("stop", name)
}

/// Remove a container (must be stopped first).
pub fn destroy(name: &str) -> Result<(), ContainerError> {
    docker_simple("rm", name)
}

/// Execute a command inside a running container.
///
/// Returns the exit code of the command.
pub fn exec(name: &str, cmd: &[&str], tty: bool) -> Result<i32, ContainerError> {
    let mut args = vec!["exec".to_string()];

    if tty {
        args.push("-t".to_string());
    }

    args.push("-w".to_string());
    args.push(WORKSPACE_MOUNT.to_string());
    args.push(name.to_string());
    args.extend(cmd.iter().map(|s| s.to_string()));

    let status = Command::new("docker")
        .args(&args)
        .status()
        .map_err(|e| ContainerError::DockerNotFound(e.to_string()))?;

    Ok(status.code().unwrap_or(1))
}

/// Check the status of a container.
pub fn status(name: &str) -> ContainerStatus {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Running}}", name])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let running = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if running == "true" {
                ContainerStatus::Running
            } else {
                ContainerStatus::Stopped
            }
        }
        _ => ContainerStatus::Missing,
    }
}

/// Get the IP address of a running container on the Docker bridge network.
pub fn get_ip(name: &str) -> Option<String> {
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
            name,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ip.is_empty() { None } else { Some(ip) }
}

/// List all dual-managed containers (name and running status).
pub fn list_all() -> Vec<(String, bool)> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "name=dual-",
            "--format",
            "{{.Names}}\t{{.State}}",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let mut parts = line.splitn(2, '\t');
                    let name = parts.next()?.to_string();
                    let state = parts.next().unwrap_or("");
                    Some((name, state == "running"))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Build the docker create arguments (for testing).
pub fn build_create_args(name: &str, workspace_dir: &Path, image: &str) -> Vec<String> {
    vec![
        "create".to_string(),
        "--name".to_string(),
        name.to_string(),
        // Bind mount workspace
        "-v".to_string(),
        format!("{}:{WORKSPACE_MOUNT}", workspace_dir.display()),
        // Anonymous volume for node_modules isolation
        "-v".to_string(),
        format!("{WORKSPACE_MOUNT}/node_modules"),
        // Working directory
        "-w".to_string(),
        WORKSPACE_MOUNT.to_string(),
        // Image
        image.to_string(),
        // Keep container running for docker exec
        "sleep".to_string(),
        "infinity".to_string(),
    ]
}

/// Build docker exec arguments (for testing).
pub fn build_exec_args(name: &str, cmd: &[&str], tty: bool) -> Vec<String> {
    let mut args = vec!["exec".to_string()];
    if tty {
        args.push("-t".to_string());
    }
    args.push("-w".to_string());
    args.push(WORKSPACE_MOUNT.to_string());
    args.push(name.to_string());
    args.extend(cmd.iter().map(|s| s.to_string()));
    args
}

fn docker_simple(operation: &str, name: &str) -> Result<(), ContainerError> {
    let output = Command::new("docker")
        .args([operation, name])
        .output()
        .map_err(|e| ContainerError::DockerNotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ContainerError::Failed {
            operation: operation.to_string(),
            name: name.to_string(),
            stderr,
        });
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("docker not found: {0}")]
    DockerNotFound(String),

    #[error("docker {operation} failed for {name}: {stderr}")]
    Failed {
        operation: String,
        name: String,
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_args_correct() {
        let args = build_create_args(
            "dual-lightfast-main",
            Path::new("/home/user/dual-workspaces/lightfast/main"),
            "node:20",
        );
        assert_eq!(args[0], "create");
        assert_eq!(args[1], "--name");
        assert_eq!(args[2], "dual-lightfast-main");
        assert_eq!(args[3], "-v");
        assert!(args[4].contains("/home/user/dual-workspaces/lightfast/main:/workspace"));
        assert_eq!(args[5], "-v");
        assert_eq!(args[6], "/workspace/node_modules");
        assert_eq!(args[7], "-w");
        assert_eq!(args[8], "/workspace");
        assert_eq!(args[9], "node:20");
        assert_eq!(args[10], "sleep");
        assert_eq!(args[11], "infinity");
    }

    #[test]
    fn exec_args_without_tty() {
        let args = build_exec_args("dual-lightfast-main", &["pnpm", "dev"], false);
        assert_eq!(
            args,
            vec![
                "exec",
                "-w",
                "/workspace",
                "dual-lightfast-main",
                "pnpm",
                "dev"
            ]
        );
    }

    #[test]
    fn exec_args_with_tty() {
        let args = build_exec_args("dual-lightfast-main", &["bash"], true);
        assert_eq!(
            args,
            vec![
                "exec",
                "-t",
                "-w",
                "/workspace",
                "dual-lightfast-main",
                "bash"
            ]
        );
    }

    #[test]
    fn container_status_variants() {
        // Just ensure the enum works
        assert_eq!(ContainerStatus::Running, ContainerStatus::Running);
        assert_ne!(ContainerStatus::Running, ContainerStatus::Stopped);
        assert_ne!(ContainerStatus::Stopped, ContainerStatus::Missing);
    }
}
