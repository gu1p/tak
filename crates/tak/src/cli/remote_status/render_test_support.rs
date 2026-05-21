use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use tak_proto::{
    ActiveJob, ContainerResourceLimits, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse,
    StorageUsage, SubmittedNeed,
};

use super::dashboard::buffer_to_plain_text;
use super::render_dashboard;
use crate::cli::remote_inventory::RemoteRecord;
use crate::cli::remote_status::RemoteStatusResult;
use crate::cli::remote_status::view::RemoteStatusView;

pub(super) fn render_dashboard_text(view: &RemoteStatusView, color_enabled: bool) -> String {
    buffer_to_plain_text(&render_dashboard_buffer(view, color_enabled))
}

pub(super) fn render_dashboard_buffer(view: &RemoteStatusView, color_enabled: bool) -> Buffer {
    let backend = TestBackend::new(118, 34);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_dashboard(frame, view, color_enabled))
        .expect("draw dashboard");
    terminal.backend().buffer().clone()
}

pub(super) fn style_for_text(buffer: &Buffer, needle: &str) -> ratatui::style::Style {
    let area = buffer.area;
    for y in area.y..(area.y + area.height) {
        let mut row = String::with_capacity(area.width as usize);
        for x in area.x..(area.x + area.width) {
            row.push_str(buffer[(x, y)].symbol());
        }
        if let Some(column) = row.find(needle) {
            let x = area.x + u16::try_from(column).expect("needle column fits in u16");
            return buffer[(x, y)].style();
        }
    }
    panic!("missing {needle:?} in dashboard buffer");
}

pub(super) fn remote(node_id: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.example"),
        bearer_token: "secret".to_string(),
        pools: vec!["default".to_string()],
        tags: vec!["builder".to_string()],
        capabilities: vec!["linux".to_string()],
        transport: "direct".to_string(),
        enabled: true,
    }
}

pub(super) fn ok_result(node_id: &str, with_job: bool) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: Some(status(node_id, "ready", with_job)),
        error: None,
    }
}

pub(super) fn warning_result(node_id: &str) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: Some(status(node_id, "recovering", false)),
        error: None,
    }
}

pub(super) fn error_result(node_id: &str) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: None,
        error: Some("node status failed with HTTP 401".to_string()),
    }
}

fn status(node_id: &str, transport_state: &str, with_job: bool) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.to_string(),
            display_name: node_id.to_string(),
            base_url: format!("http://{node_id}.example"),
            healthy: true,
            pools: vec!["default".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: "direct".to_string(),
            transport_state: transport_state.to_string(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
        }),
        storage: Some(StorageUsage {
            path: "/tmp/takd-remote-exec".to_string(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs: active_jobs(with_job),
        image_cache: None,
        queued_jobs: vec![],
    }
}

fn active_jobs(with_job: bool) -> Vec<ActiveJob> {
    if !with_job {
        return Vec::new();
    }
    vec![ActiveJob {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        task_label: "//apps/web:build".to_string(),
        started_at_ms: 1_734_000_000_000,
        needs: vec![SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 2.0,
        }],
        execution_root_bytes: 256,
        runtime: Some("containerized".to_string()),
        origin: Some("task".to_string()),
        runtime_source: Some("image:alpine:3.20".to_string()),
        command: Some("make build".to_string()),
        resource_limits: Some(ContainerResourceLimits {
            cpu_cores: 2.0,
            memory_mb: 1024,
        }),
    }]
}
