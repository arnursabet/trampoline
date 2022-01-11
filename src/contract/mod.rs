use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::{JsonBytes, Script, CellDep, DepType, OutPoint};
use ckb_types::core::{TransactionView, TransactionBuilder};
use ckb_types::packed::{CellOutput, Uint64};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use generator::{QueryProvider, TransactionProvider, GeneratorMiddleware};

use std::fs;
use std::path::PathBuf;
pub mod sudt;
pub mod generator;


pub trait ContractSchema {
    type Output;

    fn pack(&self, input: Self::Output) -> packed::Bytes;
    fn unpack(&self, bytes: Bytes) -> Self::Output;
}

#[derive(Debug, Clone)]
pub enum ContractSource {
    LocalPath(PathBuf),
    Immediate(Bytes),
    Chain(OutPoint),
}

impl ContractSource {
    pub fn load_from_path(path: PathBuf) -> std::io::Result<Bytes> {
        let file = fs::read(path)?;
        Ok(Bytes::from(file))
    }
}

pub enum ContractCellFieldSelector {
    Args,
    Data,
    LockScript,
    TypeScript,
    Capacity
}
pub enum ContractCellField<A,D> {
    Args(A),
    Data(D),
    LockScript(ckb_types::packed::Script),
    TypeScript(ckb_types::packed::Script),
    Capacity(Uint64)
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
    pub output_rules: Vec<(ContractCellFieldSelector, Box<dyn Fn(ContractCellField<A, D>) -> ContractCellField<A, D>>)>
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
        self.data_hash().map(|data_hash| {
            Script::from(
                packed::ScriptBuilder::default()
                    .args(self.args.as_ref().unwrap_or(&JsonBytes::from_vec(vec![])).clone().into_bytes().pack())
                    .code_hash(data_hash.0.pack())
                    .hash_type(ckb_types::core::ScriptHashType::Data1.into())
                    .build(),
            )
        })
    }

    pub fn script_hash(&self) -> Option<ckb_jsonrpc_types::Byte32> {
       let script: ckb_types::packed::Script = self.as_script().unwrap().into();
       Some(script.calc_script_hash().into())
    }

    pub fn as_cell_dep(out_point: OutPoint) -> CellDep {
        CellDep { out_point, dep_type: DepType::Code }
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

    pub fn read_raw_data(&self, data: Bytes) -> D {
        self.data_schema.unpack(data)
    }

    pub fn read_raw_args(&self, args: Bytes) -> A {
        self.args_schema.unpack(args)
    }

    pub fn add_output_rule<F>(&mut self, field: ContractCellFieldSelector, transform_func: F)
    where
        F: Fn(ContractCellField<A, D>) -> ContractCellField<A, D> + 'static,
    {
        self.output_rules.push((field, Box::new(transform_func)));
    }

}

impl<A, D> GeneratorMiddleware for Contract<A, D> 
where
    D: Clone

{
    fn pipe(&self, tx: TransactionView) -> TransactionView {
        type OutputWithData = (CellOutput, Bytes);
        let mut idx = 0;
        let outputs = tx.clone().outputs().into_iter().filter_map(|output| {
            let self_script_hash: ckb_types::packed::Byte32 = self.script_hash().unwrap().into();
            
            if let Some(type_) = output.type_().to_opt() {
                if type_.calc_script_hash() == self_script_hash {
                    return tx.output_with_data(idx);
                }
            }
          
            if output.lock().calc_script_hash() == self_script_hash {
                return tx.output_with_data(idx);
            }
        
            idx += 1;
            None
            
        }).collect::<Vec<OutputWithData>>();

        let outputs = outputs.into_iter().map(|output| {
           let processed =  self.output_rules.iter().fold(output, |output, rule| {
                match rule.0 {
                    ContractCellFieldSelector::Data => {
                        let data = self.read_raw_data(output.1.clone());
                        println!("Data before update {:?}", self.data_schema.pack(data.clone()));
                        let updated_field = rule.1(ContractCellField::Data(data));
                        if let ContractCellField::Data(new_data) = updated_field {
                            println!("Data after update {:?}", self.data_schema.pack(new_data.clone()));

                            return (output.0.clone(), self.data_schema.pack(new_data).unpack());
                        } else {
                            return output;
                        }
                    },
                    ContractCellFieldSelector::LockScript => todo!(),
                    ContractCellFieldSelector::TypeScript => todo!(),
                    ContractCellFieldSelector::Capacity => todo!(),
                    ContractCellFieldSelector::Args => todo!(),
                }
            });
            println!("Output bytes of processed output: {:?}", processed.1.clone().pack());
            processed
        }).collect::<Vec<OutputWithData>>();

        // let new_tx = TransactionBuilder::default()
        //     .outputs(outputs.iter().map(|out| out.0.clone()).collect::<Vec<CellOutput>>().pack())
        //     .outputs_data(outputs.iter().map(|out| out.1.clone()).collect::<Vec<Bytes>>().pack())
        //     .cell_deps(tx.cell_deps())
        //     .header_deps(tx.header_deps())
        //     .inputs(tx.inputs())
        //     .witnesses(tx.witnesses())
        //     .build();
        // new_tx
        // let mut new_tx = tx.clone();
        tx.as_advanced_builder()
            .set_outputs(outputs.iter().map(|out| out.0.clone()).collect::<Vec<CellOutput>>())
            .set_outputs_data(outputs.iter().map(|out| out.1.clone().pack()).collect::<Vec<ckb_types::packed::Bytes>>())
            .build()
        
    }
}
#[cfg(test)]
mod tests {
    use super::sudt::*;
    use super::*;
    use std::path::Path;
    use ckb_always_success_script;

    use ckb_jsonrpc_types::JsonBytes;
    use ckb_types::{packed::{Byte32, Uint128}, core::TransactionBuilder};

    // Generated from ckb-cli util blake2b --binary-path /path/to/builtins/bins/simple_udt
    const EXPECTED_SUDT_HASH: &str =
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
            output_rules: vec![]
        }
    }

    #[test]
    fn test_update_sudt_with_rule() {
       
        let mut sudt_contract = gen_sudt_contract();
        let sudt_cell = CellOutput::new_builder()
            .capacity(100_u64.pack())
            .type_(Some(ckb_types::packed::Script::from(sudt_contract.as_script().unwrap())).pack())
            .lock(sudt_contract.as_script().unwrap().into())
            .build();
        let transaction = TransactionBuilder::default()
            .output(sudt_cell)
            .outputs_data(vec![2000_u128.to_le_bytes().pack()])
            .build();
            sudt_contract.add_output_rule(
                ContractCellFieldSelector::Data, 
        |amount: ContractCellField<Byte32, Uint128>| -> ContractCellField<Byte32, Uint128> {
                        if let ContractCellField::Data(amount) = amount {
                            let mut amt_bytes = [0u8; 16];
                            amt_bytes.copy_from_slice(amount.as_slice());
                            let amt = u128::from_le_bytes(amt_bytes) + 17;
                            ContractCellField::Data(amt.pack())
                        } else {
                            amount
                        }
                      }
            );

            let new_tx = sudt_contract.pipe(transaction);
            let new_tx_amt = new_tx.output_with_data(0).unwrap().1.clone();
            println!("New tx amt as bytes: {:?}", new_tx_amt.pack());
            let new_tx_amt: u128 = sudt_contract.read_raw_data(new_tx_amt).unpack();
            assert_eq!(new_tx_amt, 2017_u128);
    }
    #[test]
    fn test_add_output_rule() {
        let mut sudt_contract = gen_sudt_contract();

        sudt_contract.add_output_rule(
            ContractCellFieldSelector::Data, 
    |amount: ContractCellField<Byte32, Uint128>| -> ContractCellField<Byte32, Uint128> {
                    if let ContractCellField::Data(amount) = amount {
                        let mut amt_bytes = [0u8; 16];
                        amt_bytes.copy_from_slice(amount.as_slice());
                        let amt = u128::from_le_bytes(amt_bytes) + 17;
                        ContractCellField::Data(amt.pack())
                    } else {
                        amount
                    }
                  }
        );
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
            &format!("\"{}\"", EXPECTED_SUDT_HASH),
            as_json_hex_str.as_str()
        );
    }

    #[test]
    fn test_sudt_data_hash_gen() {
        let sudt_contract = gen_sudt_contract();

        let code_hash = sudt_contract.data_hash().unwrap().pack();
        let hash_hex_str = format!("0x{}", hex::encode(&code_hash.raw_data().to_vec()));
        assert_eq!(EXPECTED_SUDT_HASH, hash_hex_str.as_str());
    }
}
