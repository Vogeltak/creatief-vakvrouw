use serde::{Deserialize, Serialize};

use std::iter::zip;

#[derive(Clone, Debug, Deserialize)]
pub struct FactuurForm {
    pub factuur_nummer: usize,
    pub client_name: String,
    pub client_address: String,
    pub client_zip: String,
    #[serde(rename = "task")]
    pub tasks: Vec<String>,
    #[serde(rename = "price")]
    pub prices: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Factuur {
    pub nummer: usize,
    pub client: Client,
    pub work_items: Vec<WorkItem>,
}

impl From<FactuurForm> for Factuur {
    fn from(value: FactuurForm) -> Self {
        Factuur {
            nummer: value.factuur_nummer,
            client: Client {
                name: value.client_name,
                address: value.client_address,
                zip: value.client_zip,
            },
            work_items: zip(value.tasks, value.prices)
                .map(|(desc, euro)| WorkItem { desc, euro })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Client {
    pub name: String,
    pub address: String,
    pub zip: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkItem {
    pub desc: String,
    pub euro: f32,
}
