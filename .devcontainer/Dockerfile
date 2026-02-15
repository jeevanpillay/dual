FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Dual requires tmux for terminal multiplexing
RUN apt-get update \
    && apt-get install -y --no-install-recommends tmux \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
