use std::path::{Path, PathBuf};

use dialoguer::{Confirm, Input, Select};

use crate::config::{DualConfig, HintsError};
use crate::devcontainer;

/// Result of the init wizard — everything needed to write config files.
pub struct InitResult {
    /// Path to devcontainer.json (relative to repo root)
    pub devcontainer_path: String,
    /// Whether we need to create a new devcontainer.json
    pub create_devcontainer: bool,
    /// Docker image (only used if create_devcontainer is true)
    pub image: String,
    /// Ports (only used if create_devcontainer is true)
    pub ports: Vec<u16>,
    /// Setup command (only used if create_devcontainer is true)
    pub setup: Option<String>,
}

/// Base image presets for the wizard.
const IMAGE_PRESETS: &[(&str, &str)] = &[
    ("node:20", "Node.js"),
    ("python:3.12", "Python"),
    ("rust:latest", "Rust"),
];

/// Run the interactive init wizard.
///
/// Walks through 5 steps:
/// 1. Detect existing devcontainer.json
/// 2. Select base image
/// 3. Configure ports
/// 4. Configure setup command
/// 5. Summary & confirm
pub fn run_wizard(repo_root: &Path) -> Result<InitResult, HintsError> {
    // Step 1: Detect existing devcontainer
    if let Some(dc_path) = devcontainer::find_devcontainer_json(repo_root) {
        let rel_path = dc_path
            .strip_prefix(repo_root)
            .unwrap_or(&dc_path)
            .to_string_lossy()
            .to_string();

        println!("Detected {rel_path}");
        let use_existing = Confirm::new()
            .with_prompt("Use this as your container config?")
            .default(true)
            .interact()
            .unwrap_or(true);

        if use_existing {
            return Ok(InitResult {
                devcontainer_path: rel_path,
                create_devcontainer: false,
                image: String::new(),
                ports: Vec::new(),
                setup: None,
            });
        }
    }

    // Step 2: Select base image
    let mut options: Vec<String> = IMAGE_PRESETS
        .iter()
        .map(|(img, label)| format!("{img} ({label})"))
        .collect();
    options.push("Custom (enter image name)".to_string());

    let selection = Select::new()
        .with_prompt("Select base image")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(0);

    let image = if selection < IMAGE_PRESETS.len() {
        IMAGE_PRESETS[selection].0.to_string()
    } else {
        Input::<String>::new()
            .with_prompt("Image name")
            .default("node:20".to_string())
            .interact_text()
            .unwrap_or_else(|_| "node:20".to_string())
    };

    // Step 3: Ports
    let ports_input: String = Input::new()
        .with_prompt("Which ports does your dev server use? (comma-separated, or empty for none)")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();

    let ports: Vec<u16> = ports_input
        .split(',')
        .filter_map(|s| s.trim().parse::<u16>().ok())
        .collect();

    // Step 4: Setup command
    let setup_input: String = Input::new()
        .with_prompt("Setup command after container creation? (e.g., pnpm install, or empty)")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();

    let setup = if setup_input.trim().is_empty() {
        None
    } else {
        Some(setup_input.trim().to_string())
    };

    // Step 5: Summary & confirm
    println!();
    println!("Configuration:");
    println!("  Image:    {image}");
    if ports.is_empty() {
        println!("  Ports:    (none)");
    } else {
        let port_strs: Vec<String> = ports.iter().map(|p| p.to_string()).collect();
        println!("  Ports:    {}", port_strs.join(", "));
    }
    match &setup {
        Some(cmd) => println!("  Setup:    {cmd}"),
        None => println!("  Setup:    (none)"),
    }
    println!();
    println!("  .dual/settings.json              (Dual config)");
    println!("  .devcontainer/devcontainer.json   (container config)");
    println!();

    let confirmed = Confirm::new()
        .with_prompt("Create these files?")
        .default(true)
        .interact()
        .unwrap_or(false);

    if !confirmed {
        return Err(HintsError::WriteError(
            repo_root.to_path_buf(),
            std::io::Error::other("cancelled by user"),
        ));
    }

    Ok(InitResult {
        devcontainer_path: ".devcontainer/devcontainer.json".to_string(),
        create_devcontainer: true,
        image,
        ports,
        setup,
    })
}

/// Apply non-interactive defaults (for --yes flag).
///
/// If an existing devcontainer.json is found, uses it.
/// Otherwise creates defaults: node:20, no ports, no setup.
pub fn apply_defaults(repo_root: &Path) -> InitResult {
    if let Some(dc_path) = devcontainer::find_devcontainer_json(repo_root) {
        let rel_path = dc_path
            .strip_prefix(repo_root)
            .unwrap_or(&dc_path)
            .to_string_lossy()
            .to_string();
        return InitResult {
            devcontainer_path: rel_path,
            create_devcontainer: false,
            image: String::new(),
            ports: Vec::new(),
            setup: None,
        };
    }

    InitResult {
        devcontainer_path: ".devcontainer/devcontainer.json".to_string(),
        create_devcontainer: true,
        image: "node:20".to_string(),
        ports: Vec::new(),
        setup: None,
    }
}

/// Write devcontainer.json from wizard results.
/// Creates .devcontainer/ directory if needed.
pub fn write_devcontainer(repo_root: &Path, result: &InitResult) -> Result<PathBuf, HintsError> {
    let dc_dir = repo_root.join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).map_err(|e| HintsError::WriteError(dc_dir.clone(), e))?;

    let dc_path = dc_dir.join("devcontainer.json");

    let mut dc = serde_json::Map::new();
    dc.insert(
        "image".to_string(),
        serde_json::Value::String(result.image.clone()),
    );

    if !result.ports.is_empty() {
        let ports: Vec<serde_json::Value> = result
            .ports
            .iter()
            .map(|p| serde_json::Value::Number((*p).into()))
            .collect();
        dc.insert("forwardPorts".to_string(), serde_json::Value::Array(ports));
    }

    if let Some(ref setup) = result.setup {
        dc.insert(
            "postCreateCommand".to_string(),
            serde_json::Value::String(setup.clone()),
        );
    }

    let content = serde_json::to_string_pretty(&dc).map_err(HintsError::JsonSerializeError)?;
    std::fs::write(&dc_path, format!("{content}\n"))
        .map_err(|e| HintsError::WriteError(dc_path.clone(), e))?;

    Ok(dc_path)
}

/// Write .dual/settings.json from wizard results.
/// Creates .dual/ directory if needed.
pub fn write_settings(repo_root: &Path, devcontainer_path: &str) -> Result<PathBuf, HintsError> {
    let dual_dir = repo_root.join(".dual");
    std::fs::create_dir_all(&dual_dir).map_err(|e| HintsError::WriteError(dual_dir.clone(), e))?;

    let config = DualConfig {
        devcontainer: devcontainer_path.to_string(),
        ..Default::default()
    };

    let path = dual_dir.join("settings.json");
    let contents = serde_json::to_string_pretty(&config).map_err(HintsError::JsonSerializeError)?;
    std::fs::write(&path, contents).map_err(|e| HintsError::WriteError(path.clone(), e))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_defaults_without_existing_devcontainer() {
        let dir = std::env::temp_dir().join("dual-test-init-defaults");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = apply_defaults(&dir);
        assert!(result.create_devcontainer);
        assert_eq!(result.image, "node:20");
        assert!(result.ports.is_empty());
        assert!(result.setup.is_none());
        assert_eq!(result.devcontainer_path, ".devcontainer/devcontainer.json");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_defaults_with_existing_devcontainer() {
        let dir = std::env::temp_dir().join("dual-test-init-existing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "python:3.12"}"#,
        )
        .unwrap();

        let result = apply_defaults(&dir);
        assert!(!result.create_devcontainer);
        assert_eq!(result.devcontainer_path, ".devcontainer/devcontainer.json");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_devcontainer_creates_valid_json() {
        let dir = std::env::temp_dir().join("dual-test-init-write-dc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = InitResult {
            devcontainer_path: ".devcontainer/devcontainer.json".to_string(),
            create_devcontainer: true,
            image: "node:20".to_string(),
            ports: vec![3000, 8080],
            setup: Some("pnpm install".to_string()),
        };

        write_devcontainer(&dir, &result).unwrap();

        let dc_path = dir.join(".devcontainer").join("devcontainer.json");
        assert!(dc_path.exists());

        let content = std::fs::read_to_string(&dc_path).unwrap();
        let dc: crate::devcontainer::DevcontainerJson = serde_json::from_str(&content).unwrap();
        assert_eq!(dc.image.as_deref(), Some("node:20"));
        let ports: Vec<u16> = dc
            .forward_ports
            .unwrap()
            .iter()
            .filter_map(|p| p.to_port())
            .collect();
        assert_eq!(ports, vec![3000, 8080]);
        assert_eq!(
            dc.post_create_command.unwrap().to_shell_command(),
            "pnpm install"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_devcontainer_no_ports_no_setup() {
        let dir = std::env::temp_dir().join("dual-test-init-write-dc-minimal");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = InitResult {
            devcontainer_path: ".devcontainer/devcontainer.json".to_string(),
            create_devcontainer: true,
            image: "rust:latest".to_string(),
            ports: Vec::new(),
            setup: None,
        };

        write_devcontainer(&dir, &result).unwrap();

        let content =
            std::fs::read_to_string(dir.join(".devcontainer").join("devcontainer.json")).unwrap();
        assert!(!content.contains("forwardPorts"));
        assert!(!content.contains("postCreateCommand"));
        assert!(content.contains("rust:latest"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_settings_creates_valid_json() {
        let dir = std::env::temp_dir().join("dual-test-init-write-settings");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        write_settings(&dir, ".devcontainer/devcontainer.json").unwrap();

        let path = dir.join(".dual").join("settings.json");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let config: crate::config::DualConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.devcontainer, ".devcontainer/devcontainer.json");
        assert!(config.extra_commands.is_empty());
        assert_eq!(config.anonymous_volumes, vec!["node_modules".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_settings_custom_devcontainer_path() {
        let dir = std::env::temp_dir().join("dual-test-init-custom-dc-path");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        write_settings(&dir, "custom/devcontainer.json").unwrap();

        let content = std::fs::read_to_string(dir.join(".dual").join("settings.json")).unwrap();
        let config: crate::config::DualConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.devcontainer, "custom/devcontainer.json");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
