use lazy_static::lazy_static;
use prometheus::{register_gauge_vec, GaugeVec};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time};
use tracing::{error, info};

use crate::types::{Repository, Workflow, MACOS, UBUNTU, WINDOWS};

pub async fn poll_billable_ms(
    github_workflows: Arc<HashMap<Repository, RwLock<Vec<Workflow>>>>,
    sleep: Duration,
) {
    loop {
        for (repo, workflows) in github_workflows.iter() {
            for workflow in workflows.read().await.iter() {
                if let Err(err) = poll_billable_ms_for_workflow(repo, workflow).await {
                    error!(
                        "failed to poll billable time for workflow {:?} in repo {}: {}",
                        workflow, repo, err
                    );
                } else {
                    info!("polled usage for {}:{}", repo, workflow.name);
                }
            }
        }

        time::sleep(sleep).await;
    }
}

async fn poll_billable_ms_for_workflow(
    repo: &Repository,
    workflow: &Workflow,
) -> anyhow::Result<()> {
    let octocrab = octocrab::instance();

    let usage = octocrab
        .get::<Usage, _, _>(
            octocrab
                .absolute_url(format!(
                    "repos/{owner}/{repo}/actions/workflows/{workflow_id}/timing",
                    owner = repo.owner,
                    repo = repo.name,
                    workflow_id = workflow.id,
                ))
                .expect("failed to generate absolute API url"),
            None::<&()>,
        )
        .await?;

    if let Some(BillableTime { total_ms, .. }) = usage.billable.ubuntu {
        ACTIONS_BILLABLE_MS
            .with_label_values(&[&repo.owner, &repo.name, &workflow.name, UBUNTU])
            .set(total_ms);
    }

    if let Some(BillableTime { total_ms, .. }) = usage.billable.macos {
        ACTIONS_BILLABLE_MS
            .with_label_values(&[&repo.owner, &repo.name, &workflow.name, MACOS])
            .set(total_ms);
    }

    if let Some(BillableTime { total_ms, .. }) = usage.billable.windows {
        ACTIONS_BILLABLE_MS
            .with_label_values(&[&repo.owner, &repo.name, &workflow.name, WINDOWS])
            .set(total_ms);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub billable: Billable,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct Billable {
    #[serde(rename = "UBUNTU")]
    pub ubuntu: Option<BillableTime>,
    #[serde(rename = "MACOS")]
    pub macos: Option<BillableTime>,
    #[serde(rename = "WINDOWS")]
    pub windows: Option<BillableTime>,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct BillableTime {
    pub total_ms: f64,
}

lazy_static! {
    pub static ref ACTIONS_BILLABLE_MS: GaugeVec = register_gauge_vec!(
        "github_actions_billable_ms",
        "Github Actions billable milliseconds",
        &["owner", "repository", "workflow", "os"]
    )
    .unwrap();
}
