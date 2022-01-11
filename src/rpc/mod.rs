use reqwest::{Client, Request, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use ckb_jsonrpc_types::{
    TransactionView, CellInfo,
    Script, ScriptHashType,
    TransactionWithStatus, BlockView, OutPoint, CellWithStatus, BlockNumber, Transaction
};
use ckb_types::{core::cell::{
    CellMeta,
    CellStatus
}, H256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    JsonRPC(#[from] jsonrpc_core::Error),
}

pub type RpcResult<T> = std::result::Result<T, RpcError>;

#[derive(Clone, Debug)]
pub struct RpcClient {
    pub client: reqwest::blocking::Client,
    id: u64,
}

impl RpcClient {

    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            id: 0
        }
    }


    pub fn req<T: for <'de>Deserialize<'de>, P: Serialize>(&mut self, url: impl reqwest::IntoUrl, method: impl Into<String>, payload: Vec<P>) 
    -> RpcResult<T> {
        let payload = serde_json::to_value(payload).expect("Serialize payload");
        let req_body = self.generate_json_rpc_req(method.into().as_str(), payload)?;
        let response = self.client.post(url.into_url()?).json(&req_body).send()?;
        let req_output = response.json::<jsonrpc_core::response::Output>()?;
        match req_output {
            jsonrpc_core::response::Output::Success(success) => {
                serde_json::from_value(success.result).map_err(Into::into)
            },
            jsonrpc_core::response::Output::Failure(failure) => {
                Err(failure.error.into())
            }
        }
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

    pub fn get_transaction(&mut self, hash: H256, url: impl reqwest::IntoUrl) -> RpcResult<Option<TransactionWithStatus>> {
        self.req(url, "get_transaction", vec![hash])
    }

    pub fn get_block(&mut self, hash: H256, url: impl reqwest::IntoUrl) -> RpcResult<Option<BlockView>> {
        self.req(url, "get_block", vec![hash])
    }

    pub fn get_live_cell(&mut self, out_point: OutPoint, with_data: bool, url: impl reqwest::IntoUrl) -> RpcResult<CellWithStatus> {
        self.req(url, "get_live_cell", vec![serde_json::to_string(&out_point)?, serde_json::to_string(&with_data)?])
    }

    pub fn get_block_by_number(&mut self, number: BlockNumber, url: impl reqwest::IntoUrl) -> RpcResult<Option<BlockView>> {
        self.req(url, "get_block_by_number", vec![number])
    }

    pub fn send_transaction(&mut self, tx: Transaction, url: impl reqwest::IntoUrl) -> RpcResult<H256> {
        self.req(url, "send_transaction", vec![tx])
    }
}