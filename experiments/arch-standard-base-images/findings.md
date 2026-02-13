---
date_run: 2026-02-06T12:51:50Z
experiment_design: inline (standard-base-images)
status: complete
tests_run: 4 of 4
duration: ~20s
---

# Experiment Findings: Standard Base Images

## Test Results

### Test 7.1: node:20-alpine as Dev Base
**Node version**: v20.20.0
**npm version**: 10.8.2
**Base size**: 193MB
**Missing from alpine**: git, curl, bash, openssh (need `apk add`)
**After install**: All dev tools work (git 2.52.0, curl, bash, openssh-client)
**Total after tools**: ~32MB additional = ~225MB total

### Test 7.2: npm Project Initialization
**npm init**: Works
**npm install express**: Works, 0 vulnerabilities
**Verdict**: PASS — standard package management works

### Test 7.3: Node HTTP Server
**Result**: HTTP server starts and listens on port
**Verdict**: PASS

### Test 7.4: Conceptual Dockerfile
```dockerfile
FROM node:20-alpine
RUN apk add --no-cache git curl bash openssh-client
# RUN npm install -g @anthropic-ai/claude-code
```
- 3-line Dockerfile covers the entire base image need
- No project-specific analysis or generation required
- Same image works for any Node.js project

## Summary

| Aspect | Result |
|--------|--------|
| Base image viability | PASS — node:20-alpine works |
| Dev tools installable | PASS — git, curl, bash, ssh in one `apk add` |
| npm/node work | PASS |
| Custom Dockerfile needed? | NO — standard image + fixed tools |
| Image size | ~225MB (acceptable) |

The standard base image approach works. A single pre-built image per runtime (node:18, node:20, node:22, python) with git/curl/bash/ssh added covers all common dev needs. No auto-generation of Dockerfiles required.
