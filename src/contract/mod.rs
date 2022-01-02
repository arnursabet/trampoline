use crate::schema::Schema;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::{JsonBytes, OutPoint, Script, ScriptHashType};
use ckb_types::{bytes::Bytes, core, packed, prelude::*, H256};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::PathBuf;
pub mod sudt;
pub trait ContractSchema {
    type Output;

    fn pack(&self, input: Self::Output) -> packed::Bytes;
    fn unpack(&self, bytes: Bytes) -> Self::Output;
}

#[derive(Debug, Clone)]
pub enum ContractSource {
    LocalPath(PathBuf),
    Immediate(Bytes),
}

impl ContractSource {
    pub fn load_from_path(path: PathBuf) -> std::io::Result<Bytes> {
        let file = fs::read(path)?;
        Ok(Bytes::from(file))
    }
}

pub struct Contract<A, D> {
    pub source: Option<ContractSource>,
    args_schema: Box<dyn ContractSchema<Output = A>>,
    data_schema: Box<dyn ContractSchema<Output = D>>,
    pub data: Option<JsonBytes>,
    pub args: Option<JsonBytes>,
    pub lock: Option<Script>,
    pub type_: Option<Script>,
    pub code: Option<JsonBytes>,
}

impl<A, D> From<ContractSource> for Contract<A, D> {
    fn from(_other: ContractSource) -> Contract<A, D> {
        todo!()
    }
}

impl<A, D> Contract<A, D> {
    pub fn args_schema(mut self, schema: Box<dyn ContractSchema<Output = A>>) -> Self {
        self.args_schema = schema;
        self
    }

    pub fn data_schema(mut self, schema: Box<dyn ContractSchema<Output = D>>) -> Self {
        self.data_schema = schema;
        self
    }

    pub fn lock(mut self, lock: Script) -> Self {
        self.lock = Some(lock);
        self
    }

    pub fn type_(mut self, type_: Script) -> Self {
        self.type_ = Some(type_);
        self
    }

    pub fn data_hash(&self) -> Option<H256> {
        if let Some(data) = &self.code {
            let byte_slice = data.as_bytes();

            let raw_hash = blake2b_256(&byte_slice);
            H256::from_slice(&raw_hash).ok()
        } else {
            None
        }
    }

    // Returns a script structure which can be used as a lock or type script on other cells.
    // This is an easy way to let other cells use this contract
    pub fn as_script(&self) -> Option<ckb_jsonrpc_types::Script> {
        if let Some(data_hash) = self.data_hash() {
            Some(Script::from(
                packed::ScriptBuilder::default()
                    .args(self.args.as_ref().unwrap().clone().into_bytes().pack())
                    .code_hash(data_hash.0.pack())
                    .hash_type(ckb_types::core::ScriptHashType::Data1.into())
                    .build(),
            ))
        } else {
            None
        }
    }

    pub fn set_raw_data(&mut self, data: impl Into<JsonBytes>) {
        self.data = Some(data.into());
    }

    pub fn set_data(&mut self, data: D) {
        self.data = Some(self.data_schema.pack(data).into());
    }

    pub fn set_raw_args(&mut self, args: impl Into<JsonBytes>) {
        self.args = Some(args.into());
    }

    pub fn set_args(&mut self, args: A) {
        self.args = Some(self.args_schema.pack(args).into());
    }

    pub fn read_data(&self) -> D {
        self.data_schema
            .unpack(self.data.as_ref().unwrap().clone().into_bytes())
    }

    pub fn read_args(&self) -> A {
        self.args_schema
            .unpack(self.args.as_ref().unwrap().clone().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::sudt::*;
    use super::*;
    use std::path::Path;

    use ckb_jsonrpc_types::JsonBytes;
    use ckb_types::packed::{Byte32, Uint128};

    // Generated from ckb-cli util blake2b --binary-path /path/to/builtins/bins/simple_udt
    const expected_sudt_hash: &'static str =
        "0xe1e354d6d643ad42724d40967e334984534e0367405c5ae42a9d7d63d77df419";

    fn gen_sudt_contract() -> SudtContract {
        let path_to_sudt_bin = "builtins/bins/simple_udt";

        let path_to_sudt_bin = Path::new(path_to_sudt_bin).canonicalize().unwrap();
        let sudt_src = ContractSource::load_from_path(path_to_sudt_bin).unwrap();
        let arg_schema_ptr =
            Box::new(SudtArgsSchema {}) as Box<dyn ContractSchema<Output = Byte32>>;
        let data_schema_ptr =
            Box::new(SudtDataSchema {}) as Box<dyn ContractSchema<Output = Uint128>>;
        SudtContract {
            args: None,
            data: None,
            source: Some(ContractSource::Immediate(sudt_src.clone())),
            args_schema: arg_schema_ptr,
            data_schema: data_schema_ptr,
            lock: None,
            type_: None,
            code: Some(JsonBytes::from_bytes(sudt_src)),
        }
    }
    #[test]
    fn test_contract_pack_and_unpack_data() {
        let mut sudt_contract = gen_sudt_contract();

        sudt_contract.set_args(Byte32::default());
        sudt_contract.set_data(1200_u128.pack());

        let uint128_data: u128 = sudt_contract.read_data().unpack();
        assert_eq!(uint128_data, 1200_u128);
    }

    #[test]
    fn test_sudt_data_hash_gen_json() {
        let sudt_contract = gen_sudt_contract();

        let json_code_hash =
            ckb_jsonrpc_types::Byte32::from(sudt_contract.data_hash().unwrap().pack());

        let as_json_hex_str = serde_json::to_string(&json_code_hash).unwrap();

        assert_eq!(
            &format!("\"{}\"", expected_sudt_hash),
            as_json_hex_str.as_str()
        );
    }

    #[test]
    fn test_sudt_data_hash_gen() {
        let sudt_contract = gen_sudt_contract();

        let code_hash = sudt_contract.data_hash().unwrap().pack();
        let hash_hex_str = format!("0x{}", hex::encode(&code_hash.raw_data().to_vec()));
        assert_eq!(expected_sudt_hash, hash_hex_str.as_str());
    }
}
