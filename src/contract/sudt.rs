use ckb_types::{
    bytes::Bytes,
    packed::{self, Byte32, Uint128},
    prelude::*,
};


use crate::contract::*;
pub struct SudtDataSchema {}
pub struct SudtArgsSchema {}

impl ContractSchema for SudtDataSchema {
    type Output = Uint128;

    fn pack(&self, input: Self::Output) -> packed::Bytes {
        input.as_bytes().pack()
    }

    fn unpack(&self, bytes: Bytes) -> Self::Output {
        let raw_bytes = bytes.to_vec();
        Uint128::from_slice(&raw_bytes).unwrap()
    }
}

impl ContractSchema for SudtArgsSchema {
    type Output = Byte32;

    fn pack(&self, input: Self::Output) -> packed::Bytes {
        input.as_bytes().pack()
    }

    fn unpack(&self, bytes: Bytes) -> Self::Output {
        let raw_bytes = bytes.to_vec();
        Byte32::from_slice(&raw_bytes).unwrap()
    }
}

pub type SudtContract = Contract<Byte32, Uint128>;
