use std::path::Path;

use anyhow::Result;

use crate::logging::service_log_path;
use takd::agent::{AgentConfig, TransportState, read_transport_health};

pub(super) fn print_status(config: &AgentConfig, state_root: &Path) -> Result<()> {
    let log_path = service_log_path(state_root);
    let health = read_transport_health(state_root)?;
    let transport_state = health
        .as_ref()
        .map(|value| value.transport_state)
        .unwrap_or_else(|| {
            if config.transport != "tor" && config.base_url.is_some() {
                TransportState::Ready
            } else {
                TransportState::Pending
            }
        });
    let advertised_base_url = health
        .as_ref()
        .and_then(|value| value.base_url.clone())
        .or_else(|| config.base_url.clone());

    println!("node_id: {}", config.node_id);
    println!("transport: {}", config.transport);
    println!(
        "readiness: {}",
        if advertised_base_url.is_some() {
            "advertised"
        } else {
            "pending"
        }
    );
    println!("transport_state: {}", transport_state.as_str());
    if transport_state == TransportState::Ready {
        println!("reachability: verified");
    } else {
        println!("reachability: unverified");
    }
    if let Some(base_url) = advertised_base_url {
        println!("base_url: {base_url}");
    }
    if let Some(detail) = transport_detail(health.as_ref()) {
        println!("transport_detail: {detail}");
    }
    println!("log_path: {}", log_path.display());
    println!(
        "log_state: {}",
        if log_path.exists() {
            "present"
        } else {
            "missing"
        }
    );
    Ok(())
}

fn transport_detail(health: Option<&takd::agent::TransportHealth>) -> Option<String> {
    if let Some(detail) = health
        .and_then(|value| value.detail.as_deref())
        .filter(|detail| !detail.trim().is_empty())
    {
        return Some(detail.to_string());
    }
    if health.is_some_and(|value| value.transport_state == TransportState::Pending) {
        return Some("waiting for takd Tor startup probe to report a readiness detail".to_string());
    }
    None
}
