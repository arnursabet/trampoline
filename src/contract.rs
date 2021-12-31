use crate::schema::Schema;
use ckb_jsonrpc_types::{JsonBytes, OutPoint, Script, ScriptHashType};
use ckb_types::{
    bytes::Bytes,
    core, // core::cell::CellMeta
    packed,
    prelude::*,
    H256,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ContractSource {
    LocalPath(PathBuf),
    OutPoint(OutPoint),
    Cell(core::cell::CellMeta),
}

#[derive(Debug, Default, Clone)]
pub struct Contract {
    pub source: Option<ContractSource>,
    pub args_schema: Schema,
    pub data_schema: Schema,
    pub data: Option<JsonBytes>,
    pub lock: Option<Box<Contract>>, // can call contract.to_script() to get the script for it
    pub type_: Option<Box<Contract>>,
}

impl From<ContractSource> for Contract {
    fn from(other: ContractSource) -> Contract {
        todo!()
    }
}

impl Contract {
    pub fn arg_schema(mut self, schema: Schema) -> Self {
        todo!()
    }

    pub fn data_schema(mut self, schema: Schema) -> Self {
        todo!()
    }

    pub fn lock(self, lock: Contract) -> Self {
        todo!()
    }

    pub fn type_(self, type_: Contract) -> Self {
        todo!()
    }

    pub fn data_hash(&self) -> H256 {
        todo!()
    }

    // Returns a script structure which can be used as a lock or type script on other cells
    // enabling other cells to use this contract
    pub fn script(&self, args: Bytes) -> Script {
        todo!()
    }
}
