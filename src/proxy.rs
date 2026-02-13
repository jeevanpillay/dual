use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::config::{self, DualConfig};
use crate::container;

/// Routing entry: subdomain → container IP.
type RouteMap = HashMap<String, String>;

/// Full routing state: port → (subdomain → container_ip).
pub struct ProxyState {
    /// Map of port → RouteMap (subdomain → container_ip).
    pub routes: HashMap<u16, RouteMap>,
}

impl ProxyState {
    /// Build routing table from config and running containers.
    pub fn from_config(config: &DualConfig) -> Self {
        let mut routes: HashMap<u16, RouteMap> = HashMap::new();

        for (repo, branch) in config.all_workspaces() {
            let container_name = DualConfig::container_name(&repo.name, branch);

            // Only route to running containers
            if container::status(&container_name) != container::ContainerStatus::Running {
                continue;
            }

            let ip = match container::get_ip(&container_name) {
                Some(ip) => ip,
                None => continue,
            };

            let subdomain = format!("{}-{}", repo.name, config::encode_branch(branch));

            for &port in &repo.ports {
                routes
                    .entry(port)
                    .or_default()
                    .insert(subdomain.clone(), ip.clone());
            }
        }

        ProxyState { routes }
    }

    /// Get the container IP for a given port and subdomain.
    pub fn resolve(&self, port: u16, subdomain: &str) -> Option<&str> {
        self.routes
            .get(&port)
            .and_then(|m| m.get(subdomain))
            .map(|s| s.as_str())
    }

    /// Get all unique ports that need listeners.
    pub fn ports(&self) -> Vec<u16> {
        self.routes.keys().copied().collect()
    }
}

/// Start the reverse proxy, listening on all configured ports.
pub async fn start(config: &DualConfig) -> Result<(), Box<dyn std::error::Error>> {
    let state = ProxyState::from_config(config);
    let ports = state.ports();

    if ports.is_empty() {
        println!("No ports configured for proxy. Add 'ports' to repo config in dual.toml.");
        println!("Example:");
        println!("  [[repos]]");
        println!("  name = \"my-app\"");
        println!("  url = \"...\"");
        println!("  branches = [\"main\"]");
        println!("  ports = [3000, 3001]");
        return Ok(());
    }

    let state = Arc::new(state);

    println!("Starting reverse proxy...\n");
    println!("Routes:");
    for (&port, routes) in &state.routes {
        for (subdomain, ip) in routes {
            println!("  {subdomain}.localhost:{port} → {ip}:{port}");
        }
    }
    println!();

    let mut handles = Vec::new();

    for port in ports {
        let state = Arc::clone(&state);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {addr}");

        handles.push(tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("accept error on port {port}: {e}");
                        continue;
                    }
                };

                let state = Arc::clone(&state);
                let io = TokioIo::new(stream);

                tokio::spawn(async move {
                    let service = service_fn(move |req| {
                        let state = Arc::clone(&state);
                        handle_request(state, port, req)
                    });

                    if let Err(e) = http1::Builder::new()
                        .preserve_header_case(true)
                        .serve_connection(io, service)
                        .with_upgrades()
                        .await
                        && !e.to_string().contains("connection closed")
                    {
                        eprintln!("connection error: {e}");
                    }
                });
            }
        }));
    }

    println!("\nProxy running. Press Ctrl+C to stop.");

    // Wait for all listeners (runs forever until Ctrl+C)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

/// Handle a single HTTP request by proxying to the correct container.
async fn handle_request(
    state: Arc<ProxyState>,
    port: u16,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Extract subdomain from Host header
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let subdomain = extract_subdomain(host);

    let container_ip = match subdomain.and_then(|s| state.resolve(port, s)) {
        Some(ip) => ip.to_string(),
        None => {
            let body = format!(
                "No route for host: {host}\n\nAvailable routes on port {port}:\n{}",
                state
                    .routes
                    .get(&port)
                    .map(|m| m
                        .keys()
                        .map(|s| format!("  {s}.localhost:{port}"))
                        .collect::<Vec<_>>()
                        .join("\n"))
                    .unwrap_or_default()
            );
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    // Forward the request to the container
    let target_uri = format!(
        "http://{}:{}{}",
        container_ip,
        port,
        req.uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
    );

    // Use a TCP connection to the container
    let stream = match tokio::net::TcpStream::connect(format!("{container_ip}:{port}")).await {
        Ok(s) => s,
        Err(e) => {
            let body = format!("Cannot connect to {container_ip}:{port}: {e}");
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    let io = TokioIo::new(stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            let body = format!("Handshake failed with {container_ip}:{port}: {e}");
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    // Drive the connection in the background
    tokio::spawn(async move {
        if let Err(e) = conn.await
            && !e.to_string().contains("connection closed")
        {
            eprintln!("backend connection error: {e}");
        }
    });

    // Forward request
    let _ = target_uri; // URI used for logging context
    match sender.send_request(req).await {
        Ok(resp) => {
            // Collect the response body
            let (parts, body) = resp.into_parts();
            match body.collect().await {
                Ok(collected) => Ok(Response::from_parts(parts, Full::new(collected.to_bytes()))),
                Err(e) => {
                    let body = format!("Error reading response: {e}");
                    Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
            }
        }
        Err(e) => {
            let body = format!("Proxy error: {e}");
            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
    }
}

/// Extract subdomain from a Host header value.
/// "lightfast-main.localhost:3000" → Some("lightfast-main")
/// "localhost:3000" → None
/// "lightfast-main.localhost" → Some("lightfast-main")
fn extract_subdomain(host: &str) -> Option<&str> {
    // Strip port if present
    let host_part = host.split(':').next().unwrap_or(host);

    // Check for .localhost suffix
    if let Some(subdomain) = host_part.strip_suffix(".localhost")
        && !subdomain.is_empty()
    {
        return Some(subdomain);
    }

    None
}

/// Get all configured URLs for workspaces.
pub fn workspace_urls(config: &DualConfig) -> Vec<(String, Vec<String>)> {
    let mut result = Vec::new();

    for (repo, branch) in config.all_workspaces() {
        let container_name = DualConfig::container_name(&repo.name, branch);
        let workspace_id = format!("{}-{}", repo.name, config::encode_branch(branch));
        let is_running = container::status(&container_name) == container::ContainerStatus::Running;

        let mut urls = Vec::new();
        for &port in &repo.ports {
            let status = if is_running { "\u{25cf}" } else { "\u{25cb}" };
            urls.push(format!("  {status} {workspace_id}.localhost:{port}"));
        }

        if !urls.is_empty() {
            result.push((workspace_id, urls));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_subdomain_with_port() {
        assert_eq!(
            extract_subdomain("lightfast-main.localhost:3000"),
            Some("lightfast-main")
        );
    }

    #[test]
    fn extract_subdomain_without_port() {
        assert_eq!(
            extract_subdomain("lightfast-main.localhost"),
            Some("lightfast-main")
        );
    }

    #[test]
    fn extract_subdomain_bare_localhost() {
        assert_eq!(extract_subdomain("localhost:3000"), None);
        assert_eq!(extract_subdomain("localhost"), None);
    }

    #[test]
    fn extract_subdomain_nested() {
        assert_eq!(
            extract_subdomain("lightfast-feat__auth.localhost:3001"),
            Some("lightfast-feat__auth")
        );
    }

    #[test]
    fn proxy_state_resolve() {
        let mut routes = HashMap::new();
        let mut route_map = RouteMap::new();
        route_map.insert("lightfast-main".to_string(), "172.17.0.2".to_string());
        routes.insert(3000, route_map);

        let state = ProxyState { routes };
        assert_eq!(state.resolve(3000, "lightfast-main"), Some("172.17.0.2"));
        assert_eq!(state.resolve(3000, "unknown"), None);
        assert_eq!(state.resolve(4000, "lightfast-main"), None);
    }

    #[test]
    fn proxy_state_ports() {
        let mut routes = HashMap::new();
        routes.insert(3000, RouteMap::new());
        routes.insert(3001, RouteMap::new());

        let state = ProxyState { routes };
        let mut ports = state.ports();
        ports.sort();
        assert_eq!(ports, vec![3000, 3001]);
    }
}
