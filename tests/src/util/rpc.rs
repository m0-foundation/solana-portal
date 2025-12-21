use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct SetAccountParams {
    pub data: String,
    pub executable: bool,
    pub lamports: u64,
    pub owner: String,
    #[serde(rename = "rentEpoch")]
    pub rent_epoch: u64,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}
