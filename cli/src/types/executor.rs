use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WormholeResponse {
    pub operations: Vec<Operation>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub emitter_chain: i64,
    pub sequence: String,
    pub vaa: Vaa,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vaa {
    pub raw: String,
    pub guardian_set_index: i64,
    pub is_duplicated: bool,
}
