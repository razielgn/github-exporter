use crate::types::{Organisation, Repository, Workflow};
use anyhow::Result;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, values_t, App, Arg,
};
use octocrab::Octocrab;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{info, Level};

mod http;
mod tasks;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("bind")
                .help("bind to address")
                .long("bind")
                .short("b")
                .env("GH_EXPORTER_BIND")
                .validator(|s: String| {
                    SocketAddr::from_str(&s)
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .default_value("0.0.0.0:8000"),
        )
        .arg(
            Arg::with_name("github_token")
                .help("GitHub token")
                .long("github-token")
                .short("t")
                .env("GH_TOKEN")
                .required(true),
        )
        .arg(
            Arg::with_name("github_orgs")
                .help("GitHub organisations, delimited by `,`")
                .long("github-orgs")
                .short("o")
                .multiple(true)
                .use_delimiter(true)
                .env("GH_ORGS")
                .default_value("")
        )
        .arg(
            Arg::with_name("github_repos")
                .help("GitHub repos list, formatted as owner/repo, delimited by `,`")
                .long("github-repos")
                .short("r")
                .multiple(true)
                .use_delimiter(true)
                .env("GH_REPOS")
                .default_value("")
        )
        .arg(
            Arg::with_name("github_api_baseurl")
                .help("GitHub API base url")
                .long("github-base-url")
                .short("u")
                .env("GH_API_BASEURL")
        )
        .arg(
            Arg::with_name("github_workflows_refresh")
                .help("interval when to refresh workflows cache for each GitHub repository (in seconds)")
                .long("github-workflows-refresh")
                .short("wp")
                .env("GH_WORKFLOWS_REFRESH")
                .default_value("1800"),
        )
        .arg(
            Arg::with_name("github_poll_interval")
                .help("poll interval from GitHub API (in seconds)")
                .long("github-poll-interval")
                .short("p")
                .env("GH_POLL_INTERVAL")
                .default_value("300"),
        )
        .get_matches();

    let bind_to = value_t!(matches, "bind", SocketAddr)?;
    let github_base_url = matches.value_of("github_base_url");
    let github_token = value_t!(matches, "github_token", String)?;
    let github_repos = if matches.occurrences_of("github_repos") > 0 {
        values_t!(matches, "github_repos", Repository)?
    } else {
        Default::default()
    };
    let github_orgs = if matches.occurrences_of("github_orgs") > 0 {
        Arc::new(values_t!(matches, "github_orgs", Organisation)?)
    } else {
        Default::default()
    };
    let poll_interval = Duration::from_secs(value_t!(matches, "github_poll_interval", u64)?);
    let workflows_refresh_interval =
        Duration::from_secs(value_t!(matches, "github_workflows_refresh", u64)?);

    tracing_subscriber::fmt()
        .json()
        .with_max_level(Level::INFO)
        .with_current_span(false)
        .init();

    {
        let mut builder = Octocrab::builder().personal_token(github_token);

        if let Some(s) = github_base_url {
            builder = builder.base_url(s)?;
        }

        octocrab::initialise(builder)?;
    }

    info!("configured repos: {:?}", github_repos);
    info!("configured organisations: {:?}", github_orgs);

    let github_workflows = Arc::new(
        github_repos
            .into_iter()
            .map(|r| (r, RwLock::new(Vec::<Workflow>::new())))
            .collect::<HashMap<_, _>>(),
    );

    let _ = tokio::spawn(tasks::poll_workflows(
        github_workflows.clone(),
        workflows_refresh_interval,
    ));

    let _ = tokio::spawn(tasks::poll_billable_ms(
        github_workflows.clone(),
        poll_interval,
    ));

    let _ = tokio::spawn(tasks::poll_orgs_billing(github_orgs, poll_interval));

    http::listen(&bind_to).await?;

    Ok(())
}
