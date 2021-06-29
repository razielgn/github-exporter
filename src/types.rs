use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use octocrab::models::WorkflowId;

pub static UBUNTU: &str = "ubuntu";
pub static MACOS: &str = "macos";
pub static WINDOWS: &str = "windows";

pub type Organisation = String;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Repository {
    pub owner: Organisation,
    pub name: String,
}

impl FromStr for Repository {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (owner, name) = s
            .split_once('/')
            .ok_or("repo must be in format {owner}/{name}!")?;

        Ok(Repository {
            owner: owner.into(),
            name: name.into(),
        })
    }
}

impl Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

#[derive(Debug)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
}

impl Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) {}", self.id, self.name)
    }
}
