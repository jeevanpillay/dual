# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# devcontainer.json as Primary Config Source

## Context

The current implementation treats devcontainer.json as a fallback when `.dual.toml` is missing. The user wants the opposite model: devcontainer.json is the **primary source** for container configuration, and `.dual.toml` contains only Dual-specific orchestration fields. This aligns with the devcontainer ecosystem — users define their container in devcontainer.json, and `.dual.toml` handles only what devcon...

### Prompt 2

commit this

