use std::{collections::HashMap, path::Path};

use serde::Deserialize;
use tracing::instrument;

pub mod webhook;

#[derive(Debug)]
pub struct Config {
    pub endpoints: HashMap<String, HashMap<String, Task>>,
}

impl Config {
    pub async fn load(path: &Path) -> Result<Self, ()> {
        let content = tokio::fs::read(path).await.map_err(|_| ())?;

        let endpoints: HashMap<String, HashMap<String, Task>> =
            serde_json::from_slice(&content).map_err(|_| ())?;

        Ok(Self { endpoints })
    }
}

#[derive(Debug, Deserialize)]
pub enum Task {
    RestartJob { id: String },
}

#[derive(Debug)]
pub struct NomadConfig {
    endpoint: reqwest::Url,
}

impl NomadConfig {
    pub fn new<U>(url: U) -> Self
    where
        U: reqwest::IntoUrl,
    {
        Self {
            endpoint: url.into_url().unwrap(),
        }
    }
}

impl Task {
    #[instrument(skip(req_client))]
    pub async fn perform(&self, config: &NomadConfig, req_client: &reqwest::Client) {
        match self {
            Self::RestartJob { id } => {
                let job_allocs = match nomad::list_job_allocations(id, config, req_client).await {
                    Ok(ja) => ja,
                    Err(e) => {
                        tracing::error!("Listing Allocations: {:?}", e);
                        return;
                    }
                };

                let allocs = job_allocs
                    .allocs
                    .into_iter()
                    .filter(|alloc| alloc.client_status == "running");

                for alloc in allocs {
                    if let Err(e) = nomad::restart_allocation(&alloc.id, config, req_client).await {
                        tracing::error!("Restarting Allocation ({}): {:?}", alloc.id, e);
                    }
                }
            }
        };
    }
}

mod nomad {
    use serde_json::json;

    use crate::NomadConfig;

    #[derive(Debug, serde::Deserialize)]
    pub struct Allocation {
        #[serde(rename = "ID")]
        pub id: String,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "ClientStatus")]
        pub client_status: String,
    }

    #[derive(Debug)]
    pub struct JobAllocations {
        pub allocs: Vec<Allocation>,
    }

    #[derive(Debug)]
    pub enum ListJobAllocsError {
        CreateUrl,
        SendRequest(reqwest::Error),
        RequestFailed(reqwest::Response),
        LoadingBody,
    }

    pub async fn list_job_allocations(
        id: &str,
        config: &NomadConfig,
        client: &reqwest::Client,
    ) -> Result<JobAllocations, ListJobAllocsError> {
        let url = config
            .endpoint
            .join(&format!("/v1/job/{id}/allocations"))
            .map_err(|_| ListJobAllocsError::CreateUrl)?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(ListJobAllocsError::SendRequest)?;

        if !resp.status().is_success() {
            return Err(ListJobAllocsError::RequestFailed(resp));
        }

        let body = resp
            .bytes()
            .await
            .map_err(|_| ListJobAllocsError::LoadingBody)?;
        let result: Vec<Allocation> = serde_json::from_slice(&body).unwrap();

        Ok(JobAllocations { allocs: result })
    }

    pub async fn restart_allocation(
        id: &str,
        config: &NomadConfig,
        client: &reqwest::Client,
    ) -> Result<(), ()> {
        let target_url = config
            .endpoint
            .join(&format!("/v1/client/allocation/{id}/restart"))
            .map_err(|_| ())?;

        let body = json!({ "AllTasks": true });

        let resp = client
            .post(target_url)
            .body(serde_json::to_string(&body).unwrap())
            .send()
            .await
            .map_err(|_| ())?;

        if !resp.status().is_success() {
            return Err(());
        }

        Ok(())
    }
}
