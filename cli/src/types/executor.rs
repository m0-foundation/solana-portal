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

pub type ExecutorTransactions = Vec<ExecutorTransaction>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorTransaction {
    pub chain_id: i64,
    pub id: String,
    pub failure_cause: String,
    pub failure_message: String,
    pub status: String,
    pub tx_hash: String,
    pub txs: Vec<Tx>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    pub tx_hash: String,
    pub chain_id: i64,
    pub block_number: String,
    pub block_time: String,
    pub cost: String,
}
