use std::{net::IpAddr, net::SocketAddr};

use axum::http::HeaderMap;

/// Number of reverse proxies we control, read once from env (e.g. "1" for a single
/// nginx in front). When unset or 0 we trust NO forwarded headers and use the socket
/// peer, which is the safe default for ban enforcement and logging.
fn trusted_proxy_hops() -> usize {
    std::env::var("TRUSTED_PROXY_HOPS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

/// Resolve the real client IP, establishing a trusted-proxy boundary instead of
/// trusting the leftmost (attacker-controlled) `X-Forwarded-For` hop.
///
/// We select the rightmost UNTRUSTED address from the hop chain, gated on the
/// configured trusted-hop count. When no trusted proxy boundary is configured we
/// ignore client-supplied headers entirely and use the socket peer; this is the
/// fail-safe default for ban enforcement and visitor logging.
pub fn extract_client_ip(headers: &HeaderMap, fallback: SocketAddr) -> Option<IpAddr> {
    let hops = trusted_proxy_hops();
    if hops == 0 {
        // No trusted proxy boundary configured: ignore client-supplied headers.
        return Some(fallback.ip());
    }

    // Build the hop chain as the proxy sees it: [client, proxy1, ..., proxyN].
    // The direct peer (fallback.ip()) is the last hop; XFF entries precede it left-to-right.
    let mut chain: Vec<IpAddr> = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .map(|raw| {
            raw.split(',')
                .filter_map(|part| part.trim().parse::<IpAddr>().ok())
                .collect()
        })
        .unwrap_or_default();
    chain.push(fallback.ip());

    // Strip the `hops` rightmost trusted-proxy addresses; the next one is the real client.
    // If the chain is shorter than the trusted-hop count, the request did not pass through
    // our proxies as expected, so fall back to the direct peer rather than trusting input.
    if chain.len() <= hops {
        return Some(fallback.ip());
    }
    let idx = chain.len() - hops - 1;
    match chain.get(idx) {
        Some(ip) => Some(*ip),
        None => Some(fallback.ip()),
    }
}
