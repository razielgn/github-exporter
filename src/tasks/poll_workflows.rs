use crate::types::{Repository, Workflow};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time};
use tracing::{error, info};

pub async fn poll_workflows(
    github_workflows: Arc<HashMap<Repository, RwLock<Vec<Workflow>>>>,
    sleep: Duration,
) {
    loop {
        for (repo, workflows) in github_workflows.iter() {
            if let Err(err) = poll_workflow(repo, workflows).await {
                error!("failed to fetch workflows for repo {}: {}", repo, err);
            }
        }

        time::sleep(sleep).await;
    }
}

async fn poll_workflow(repo: &Repository, workflows: &RwLock<Vec<Workflow>>) -> anyhow::Result<()> {
    let octocrab = octocrab::instance();

    let page = octocrab
        .workflows(&repo.owner, &repo.name)
        .list()
        .per_page(100)
        .send()
        .await?;

    let updated_workflows = page
        .into_iter()
        .map(|w| Workflow {
            id: w.id,
            name: w.name,
        })
        .collect::<Vec<_>>();

    info!(
        "found workflows for repo `{}`: {:?}",
        repo, updated_workflows
    );

    {
        let mut w = workflows.write().await;
        *w = updated_workflows;
    }

    Ok(())
}
