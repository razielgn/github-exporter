use crate::types::{Organisation, MACOS, UBUNTU, WINDOWS};
use lazy_static::lazy_static;
use prometheus::{register_gauge_vec, GaugeVec};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::{sync::Arc, time::Duration};
use tokio::time;
use tracing::{error, info};

pub async fn poll_orgs_billing(orgs: Arc<Vec<Organisation>>, sleep: Duration) {
    loop {
        for org in orgs.iter() {
            if let Err(err) = poll_org_billing(org).await {
                error!("failed to poll org billing for org `{}`: {}", org, err);
            }
        }

        time::sleep(sleep).await;
    }
}

async fn poll_org_billing(org: &str) -> anyhow::Result<()> {
    let octocrab = octocrab::instance();

    let actions_billing_fut = octocrab.get::<ActionsBilling, _, _>(
        octocrab
            .absolute_url(format!("orgs/{}/settings/billing/actions", org))
            .expect("failed to generate absolute API url"),
        None::<&()>,
    );

    let packages_billing_fut = octocrab.get::<PackagesBilling, _, _>(
        octocrab
            .absolute_url(format!("orgs/{}/settings/billing/packages", org))
            .expect("failed to generate absolute API url"),
        None::<&()>,
    );

    let shared_storage_billing_fut = octocrab.get::<SharedStorageBilling, _, _>(
        octocrab
            .absolute_url(format!("orgs/{}/settings/billing/shared-storage", org))
            .expect("failed to generate absolute API url"),
        None::<&()>,
    );

    let (actions_billing_res, packages_billing_res, shared_storage_billing_res) = tokio::join!(
        actions_billing_fut,
        packages_billing_fut,
        shared_storage_billing_fut
    );

    set_metrics_actions_billing(org, &actions_billing_res?);
    set_metrics_packages_billing(org, &packages_billing_res?);
    set_metrics_shared_storage_billing(org, &shared_storage_billing_res?);

    info!("polled org billing for `{}`", org);

    Ok(())
}

fn set_metrics_actions_billing(org: &str, actions_billing: &ActionsBilling) {
    ORG_BILLING_ACTIONS_TOTAL_MINUTES_USED
        .with_label_values(&[org])
        .set(actions_billing.total_minutes_used);
    ORG_BILLING_ACTIONS_TOTAL_PAID_MINUTES_USED
        .with_label_values(&[org])
        .set(actions_billing.total_paid_minutes_used);
    ORG_BILLING_ACTIONS_INCLUDED_MINUTES
        .with_label_values(&[org])
        .set(actions_billing.included_minutes);

    if let Some(m) = actions_billing.minutes_used_breakdown.ubuntu {
        ORG_BILLING_ACTIONS_MINUTES_USED_BREAKDOWN
            .with_label_values(&[org, UBUNTU])
            .set(m);
    }

    if let Some(m) = actions_billing.minutes_used_breakdown.macos {
        ORG_BILLING_ACTIONS_MINUTES_USED_BREAKDOWN
            .with_label_values(&[org, MACOS])
            .set(m);
    }

    if let Some(m) = actions_billing.minutes_used_breakdown.windows {
        ORG_BILLING_ACTIONS_MINUTES_USED_BREAKDOWN
            .with_label_values(&[org, WINDOWS])
            .set(m);
    }
}

fn set_metrics_packages_billing(org: &str, packages_billing: &PackagesBilling) {
    ORG_BILLING_PACKAGES_INCLUDED_GIGABYTES_BANDWIDTH
        .with_label_values(&[org])
        .set(packages_billing.included_gigabytes_bandwidth);
    ORG_BILLING_PACKAGES_TOTAL_GIGABYTES_BANDWIDTH_USED
        .with_label_values(&[org])
        .set(packages_billing.total_gigabytes_bandwidth_used);
    ORG_BILLING_PACKAGES_TOTAL_PAID_GIGABYTES_BANDWIDTH_USED
        .with_label_values(&[org])
        .set(packages_billing.total_paid_gigabytes_bandwidth_used);
}

fn set_metrics_shared_storage_billing(org: &str, shared_storage_billing: &SharedStorageBilling) {
    ORG_BILLING_SHARED_STORAGE_DAYS_LEFT_IN_BILLING_CYCLE
        .with_label_values(&[org])
        .set(shared_storage_billing.days_left_in_billing_cycle);
    ORG_BILLING_SHARED_STORAGE_ESTIMATED_PAID_STORAGE_FOR_MONTH
        .with_label_values(&[org])
        .set(shared_storage_billing.estimated_paid_storage_for_month);
    ORG_BILLING_SHARED_STORAGE_ESTIMATED_STORAGE_FOR_MONTH
        .with_label_values(&[org])
        .set(shared_storage_billing.estimated_storage_for_month);
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct ActionsBilling {
    pub total_minutes_used: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_paid_minutes_used: f64,
    pub included_minutes: f64,
    pub minutes_used_breakdown: MinutesUsedBreakdown,
}

#[derive(Debug, Deserialize)]
pub struct MinutesUsedBreakdown {
    #[serde(rename = "UBUNTU")]
    pub ubuntu: Option<f64>,
    #[serde(rename = "MACOS")]
    pub macos: Option<f64>,
    #[serde(rename = "WINDOWS")]
    pub windows: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PackagesBilling {
    pub total_gigabytes_bandwidth_used: f64,
    pub total_paid_gigabytes_bandwidth_used: f64,
    pub included_gigabytes_bandwidth: f64,
}

#[derive(Debug, Deserialize)]
pub struct SharedStorageBilling {
    pub days_left_in_billing_cycle: f64,
    pub estimated_paid_storage_for_month: f64,
    pub estimated_storage_for_month: f64,
}

lazy_static! {
    pub static ref ORG_BILLING_ACTIONS_TOTAL_MINUTES_USED: GaugeVec = register_gauge_vec!(
        "github_org_billing_actions_total_minutes_used",
        "Github Actions organisation billing total minutes used",
        &["organisation"]
    )
    .unwrap();
    pub static ref ORG_BILLING_ACTIONS_TOTAL_PAID_MINUTES_USED: GaugeVec = register_gauge_vec!(
        "github_org_billing_actions_total_paid_minutes_used",
        "Github Actions organisation billing total paid minutes used",
        &["organisation"]
    )
    .unwrap();
    pub static ref ORG_BILLING_ACTIONS_INCLUDED_MINUTES: GaugeVec = register_gauge_vec!(
        "github_org_billing_actions_included_minutes",
        "Github Actions organisation billing included minutes",
        &["organisation"]
    )
    .unwrap();
    pub static ref ORG_BILLING_ACTIONS_MINUTES_USED_BREAKDOWN: GaugeVec = register_gauge_vec!(
        "github_org_billing_actions_minutes_used_breakdown",
        "Github Actions organisation billing minutes breakdown",
        &["organisation", "os"]
    )
    .unwrap();
    pub static ref ORG_BILLING_PACKAGES_TOTAL_GIGABYTES_BANDWIDTH_USED: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_packages_total_gigabytes_bandwidth_used",
            "Github Packages organisation billing total gigabytes bandwidth used",
            &["organisation"]
        )
        .unwrap();
    pub static ref ORG_BILLING_PACKAGES_TOTAL_PAID_GIGABYTES_BANDWIDTH_USED: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_packages_total_paid_gigabytes_bandwidth_used",
            "Github Packages organisation billing total paid gigabytes bandwidth used",
            &["organisation"]
        )
        .unwrap();
    pub static ref ORG_BILLING_PACKAGES_INCLUDED_GIGABYTES_BANDWIDTH: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_packages_included_gigabytes_bandwidth",
            "Github Packages organisation billing included gigabytes bandwidth",
            &["organisation"]
        )
        .unwrap();
    pub static ref ORG_BILLING_SHARED_STORAGE_DAYS_LEFT_IN_BILLING_CYCLE: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_shared_storage_days_left_in_billing_cycle",
            "Github Shared Storage organisation billing days left in billing cycle",
            &["organisation"]
        )
        .unwrap();
    pub static ref ORG_BILLING_SHARED_STORAGE_ESTIMATED_PAID_STORAGE_FOR_MONTH: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_shared_storage_estimated_paid_storage_for_month",
            "Github Shared Storage organisation billing estimated paid storage for month",
            &["organisation"]
        )
        .unwrap();
    pub static ref ORG_BILLING_SHARED_STORAGE_ESTIMATED_STORAGE_FOR_MONTH: GaugeVec =
        register_gauge_vec!(
            "github_org_billing_shared_storage_estimated_storage_for_month",
            "Github Shared Storage organisation billing estimated storage for month",
            &["organisation"]
        )
        .unwrap();
}
