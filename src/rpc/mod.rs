use reqwest::{Client, Request, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use ckb_jsonrpc_types::{
    TransactionView, CellInfo,
    Script, ScriptHashType,
};
use ckb_types::core::cell::{
    CellMeta,
    CellStatus
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub type RpcResult<T> = std::result::Result<T, RpcError>;

#[derive(Clone, Debug)]
pub struct RpcClient {
    pub url: reqwest::Url,
    id: u64,
}

impl RpcClient {

    pub fn new(url: impl reqwest::IntoUrl) -> Self {
        Self {
            url: url.into_url().expect("Invalid url supplied to rpc client constructor"),
            id: 0
        }
    }


    pub fn req(endpoint: impl reqwest::IntoUrl, method: impl Into<String>, payload: Vec<impl Serialize>) -> RpcResult<()> {
        let payload = serde_json::to_value(payload)?;
        Ok(())
    }

    fn generate_json_rpc_req(&mut self, method: &str, payload: serde_json::Value) -> 
    RpcResult<serde_json::Map<String, serde_json::Value>> {
        self.id += 1;
        let mut map = serde_json::Map::new();
        map.insert("id".to_owned(), serde_json::json!(self.id));
        map.insert("jsonrpc".to_owned(), serde_json::json!("2.0"));
        map.insert("method".to_owned(), serde_json::json!(method));
        map.insert("params".to_owned(),payload);

        Ok(map)
    }



}