---
date_run: 2026-02-06T12:48:00Z
experiment_design: inline (subdomain-proxy-http + websocket + sse)
status: complete
tests_run: 5 of 5
duration: ~120s
---

# Experiment Findings: Reverse Proxy Subdomain Routing (HTTP + WebSocket + SSE)

## Setup

- Two Node.js backends (A and B) each binding :3000 on a Docker custom network
- nginx reverse proxy routing by Host header (subdomain)
- Backend A: `lightfast-main.localhost`
- Backend B: `lightfast-feat-auth.localhost`
- Both serve HTTP, WebSocket (upgrade handler), and SSE (/sse endpoint)

## Test Results

### Test 11.1: HTTP Subdomain Routing
**Backend A via `Host: lightfast-main.localhost`**: `backend-a` — CORRECT
**Backend B via `Host: lightfast-feat-auth.localhost`**: `backend-b` — CORRECT
**Unknown host**: `No workspace found for this hostname` — CORRECT (404 default)
**Verdict**: PASS

### Test 12.1: WebSocket Upgrade Through Proxy
**Response**: `HTTP/1.1 101 Switching Protocols` with `Upgrade: websocket`
**Verdict**: PASS — nginx forwards WebSocket upgrade correctly

### Test 12.2: Full WebSocket Data Exchange
**Sent**: `hello-from-client` through proxy to Backend A
**Received**: `ws-backend-a:hello-from-client`
**Verdict**: PASS — full bidirectional WebSocket works through proxy

### Test 14.1: SSE Through Proxy (Backend A)
**Received**: 3 events (`backend-a-event-1`, `backend-a-event-2`, `backend-a-event-3`)
**Verdict**: PASS

### Test 14.2: SSE Through Proxy (Backend B)
**Received**: 3 events (`backend-b-event-1`, `backend-b-event-2`, `backend-b-event-3`)
**Verdict**: PASS

## Summary

| Feature | Status | Notes |
|---------|--------|-------|
| HTTP routing by subdomain | PASS | Host header → correct backend |
| WebSocket upgrade | PASS | 101 Switching Protocols |
| WebSocket data exchange | PASS | Bidirectional through proxy |
| SSE streaming | PASS | Events stream through proxy |
| Default routing (unknown host) | PASS | Returns 404 |

## Implementation Notes

- nginx `proxy_http_version 1.1` + `Upgrade`/`Connection` headers required for WebSocket
- Custom Docker network enables DNS resolution between proxy and backends
- Dynamic registration (not tested here) would require nginx config reload or a programmable proxy like Caddy/Traefik
