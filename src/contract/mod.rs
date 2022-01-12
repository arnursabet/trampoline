use ckb_hash::blake2b_256;
use ckb_jsonrpc_types::{CellDep, DepType, JsonBytes, OutPoint, Script};
use ckb_types::core::{TransactionView};
use ckb_types::packed::{CellOutput, CellOutputBuilder, Uint64};
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use generator::{GeneratorMiddleware};

use std::fs;
use std::path::PathBuf;

use self::chain::CellOutputWithData;
use self::generator::CellQuery;
pub mod chain;
pub mod generator;
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
    Capacity,
}
pub enum ContractCellField<A, D> {
    Args(A),
    Data(D),
    LockScript(ckb_types::packed::Script),
    TypeScript(ckb_types::packed::Script),
    Capacity(Uint64),
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
    #[allow(clippy::type_complexity)]
    pub output_rules: Vec<(
        ContractCellFieldSelector,
        Box<dyn Fn(ContractCellField<A, D>) -> ContractCellField<A, D>>,
    )>,
    pub input_rules: Vec<Box<dyn Fn(TransactionView) -> CellQuery>>,
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

    // The lock script of the cell containing contract code
    pub fn lock(mut self, lock: Script) -> Self {
        self.lock = Some(lock);
        self
    }

    // The type script of the cell containing contract code
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
                    .args(
                        self.args
                            .as_ref()
                            .unwrap_or(&JsonBytes::from_vec(vec![]))
                            .clone()
                            .into_bytes()
                            .pack(),
                    )
                    .code_hash(data_hash.pack())
                    .hash_type(ckb_types::core::ScriptHashType::Data1.into())
                    .build(),
            )
        })
    }

    // Return a CellOutputWithData which is the code cell storing this contract's logic
    pub fn as_code_cell(&self) -> CellOutputWithData {
        let data: Bytes = self.code.clone().unwrap_or_default().into_bytes();
        let type_script = self.type_.clone().unwrap_or_default();
        let type_script = {
            if self.type_.is_some() {
                Some(ckb_types::packed::Script::from(type_script))
            } else {
                None
            }
        };

        let cell_output = CellOutputBuilder::default()
            .capacity((data.len() as u64).pack())
            .lock(self.lock.clone().unwrap_or_default().into())
            .type_(type_script.pack())
            .build();
        (cell_output, data)
    }

    pub fn script_hash(&self) -> Option<ckb_jsonrpc_types::Byte32> {
        let script: ckb_types::packed::Script = self.as_script().unwrap().into();
        Some(script.calc_script_hash().into())
    }

    pub fn as_cell_dep(&self, out_point: OutPoint) -> CellDep {
        CellDep {
            out_point,
            dep_type: DepType::Code,
        }
    }

    // Set data of a cell that will *reference* (i.e., use) this contract
    pub fn set_raw_data(&mut self, data: impl Into<JsonBytes>) {
        self.data = Some(data.into());
    }

    pub fn set_data(&mut self, data: D) {
        self.data = Some(self.data_schema.pack(data).into());
    }

    // Set args of a cell that will *reference* (i.e., use) this contract
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

    pub fn add_input_rule<F>(&mut self, query_func: F)
    where
        F: Fn(TransactionView) -> CellQuery + 'static,
    {
        self.input_rules.push(Box::new(query_func))
    }
}

impl<A, D> GeneratorMiddleware for Contract<A, D>
where
    D: Clone,
{
    fn pipe(&self, tx: TransactionView) -> TransactionView {
        type OutputWithData = (CellOutput, Bytes);
        let mut idx = 0;
        let outputs = tx
            .clone()
            .outputs()
            .into_iter()
            .filter_map(|output| {
                let self_script_hash: ckb_types::packed::Byte32 =
                    self.script_hash().unwrap().into();

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
            });

        let outputs = outputs
            .into_iter()
            .map(|output| {
                let processed =
                    self.output_rules
                        .iter()
                        .fold(output, |output, rule| match rule.0 {
                            ContractCellFieldSelector::Data => {
                                let data = self.read_raw_data(output.1.clone());
                                println!(
                                    "Data before update {:?}",
                                    self.data_schema.pack(data.clone())
                                );
                                let updated_field = rule.1(ContractCellField::Data(data));
                                if let ContractCellField::Data(new_data) = updated_field {
                                    println!(
                                        "Data after update {:?}",
                                        self.data_schema.pack(new_data.clone())
                                    );

                                    (
                                        output.0,
                                        self.data_schema.pack(new_data).unpack(),
                                    )
                                } else {
                                    output
                                }
                            }
                            ContractCellFieldSelector::LockScript => todo!(),
                            ContractCellFieldSelector::TypeScript => todo!(),
                            ContractCellFieldSelector::Capacity => todo!(),
                            ContractCellFieldSelector::Args => todo!(),
                        });
                println!(
                    "Output bytes of processed output: {:?}",
                    processed.1.pack()
                );
                processed
            })
            .collect::<Vec<OutputWithData>>();

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
            .set_outputs(
                outputs
                    .iter()
                    .map(|out| out.0.clone())
                    .collect::<Vec<CellOutput>>(),
            )
            .set_outputs_data(
                outputs
                    .iter()
                    .map(|out| out.1.clone().pack())
                    .collect::<Vec<ckb_types::packed::Bytes>>(),
            )
            .build()
    }
}
#[cfg(test)]
mod tests {
    use super::sudt::*;
    use super::*;
    use chain::{
        MockChain,
        MockChainTxProvider as ChainRpc,
    };
    use ckb_always_success_script;
    use ckb_jsonrpc_types::JsonBytes;
    use ckb_types::{
        core::TransactionBuilder,
        packed::{Byte32, CellInputBuilder, Uint128},
    };
    use generator::*;
    use std::path::Path;

    // Generated from ckb-cli util blake2b --binary-path /path/to/builtins/bins/simple_udt
    const EXPECTED_SUDT_HASH: &str =
        "0xe1e354d6d643ad42724d40967e334984534e0367405c5ae42a9d7d63d77df419";

    fn gen_sudt_contract(
        minter_lock: Option<ckb_types::packed::Script>,
        initial_supply: Option<u128>,
    ) -> SudtContract {
        let path_to_sudt_bin = "builtins/bins/simple_udt";

        let lock = {
            if let Some(lock_script) = minter_lock {
                Some(JsonBytes::from_bytes(
                    lock_script.calc_script_hash().as_bytes(),
                ))
            } else {
                Some(JsonBytes::from_bytes(Byte32::default().as_bytes()))
            }
        };

        let init_supply = {
            if let Some(supply) = initial_supply {
                let supply = supply.to_le_bytes();
                let mut bytes_buf = [0u8; 16];
                bytes_buf.copy_from_slice(&supply);
                Some(JsonBytes::from_vec(bytes_buf.to_vec()))
            } else {
                let supply = 0_u128.to_le_bytes();
                let mut bytes_buf = [0u8; 16];
                bytes_buf.copy_from_slice(&supply);
                Some(JsonBytes::from_vec(bytes_buf.to_vec()))
            }
        };
        

        
        let path_to_sudt_bin = Path::new(path_to_sudt_bin).canonicalize().unwrap();
        let sudt_src = ContractSource::load_from_path(path_to_sudt_bin).unwrap();
        let arg_schema_ptr =
            Box::new(SudtArgsSchema {}) as Box<dyn ContractSchema<Output = Byte32>>;
        let data_schema_ptr =
            Box::new(SudtDataSchema {}) as Box<dyn ContractSchema<Output = Uint128>>;
        SudtContract {
            args: lock,
            data: init_supply,
            source: Some(ContractSource::Immediate(sudt_src.clone())),
            args_schema: arg_schema_ptr,
            data_schema: data_schema_ptr,
            lock: None,
            type_: None,
            code: Some(JsonBytes::from_bytes(sudt_src)),
            output_rules: vec![],
            input_rules: vec![],
        }
    }

    fn generate_always_success_lock(
        args: Option<ckb_types::packed::Bytes>,
    ) -> ckb_types::packed::Script {
        let data: Bytes = ckb_always_success_script::ALWAYS_SUCCESS.to_vec().into();
        let data_hash = H256::from(blake2b_256(data.to_vec().as_slice()));
        ckb_types::packed::Script::default()
            .as_builder()
            .args(args.unwrap_or([0u8].pack()))
            .code_hash(data_hash.pack())
            .hash_type(ckb_types::core::ScriptHashType::Data1.into())
            .build()
    }
    fn generate_simple_udt_cell(sudt_contract: &SudtContract) -> CellOutput {
        let lock = sudt_contract
            .lock
            .clone()
            .unwrap_or(generate_always_success_lock(None).into());
        // let mut capacity_buf = [0u8; 16];
        // capacity_buf.copy_from_slice(sudt_contract.read_data().as_slice());
        // let capacity = u128::from_le_bytes(capacity_buf);
        CellOutput::new_builder()
            .capacity(100_u64.pack())
            .type_(
                Some(ckb_types::packed::Script::from(
                    sudt_contract.as_script().unwrap(),
                ))
                .pack(),
            )
            .lock(lock.into())
            .build()
    }

    fn generate_mock_tx(
        outputs: Vec<CellOutput>,
        outputs_data: Vec<ckb_types::packed::Bytes>,
    ) -> TransactionView {
        TransactionBuilder::default()
            .outputs(outputs)
            .outputs_data(outputs_data)
            .build()
    }

    #[test]
    fn test_sudt_issuance_tx_with_contract_pipeline() {
        let mut chain = MockChain::default();

        // Create always success lock cell and add to chain
        let minter_lock_code_cell_data: Bytes =
            ckb_always_success_script::ALWAYS_SUCCESS.to_vec().into();
        let minter_lock_cell = chain.deploy_cell_with_data(minter_lock_code_cell_data);
        let minter_lock_script = chain.build_script(&minter_lock_cell, vec![1_u8].into());
        let non_minter_lock = chain.build_script(&minter_lock_cell, vec![200_u8].into());

        // Create two cells locked with always success but w/ different script args
        let minter_owned_cell = chain.create_cell(
            CellOutput::new_builder()
                .capacity(2000_u64.pack())
                .lock(minter_lock_script.clone().unwrap())
                .build(),
            Default::default(),
        );

        let non_minter_owned_cell = chain.create_cell(
            CellOutput::new_builder()
                .capacity(2000_u64.pack())
                .lock(non_minter_lock.unwrap())
                .build(),
            Default::default(),
        );

        // Deploy SUDT to chain
        let mut sudt_contract = gen_sudt_contract(minter_lock_script.clone(), Some(1500));
        let sudt_code_cell = sudt_contract.as_code_cell();
        let sudt_code_cell_outpoint = chain.create_cell(sudt_code_cell.0, sudt_code_cell.1);

        // Create Cell Inputs
        let minter_owned_cell_input = CellInputBuilder::default()
            .previous_output(minter_owned_cell)
            .build();

        let non_minter_owned_cell_input = CellInputBuilder::default()
            .previous_output(non_minter_owned_cell)
            .build();

        // Create Mint SUDT transaction, using as input a cell locked with the minter's lock script
        let tx = TransactionBuilder::default()
            .input(minter_owned_cell_input)
            .cell_dep(
                sudt_contract
                    .as_cell_dep(sudt_code_cell_outpoint.clone().into())
                    .into(),
            )
            .cell_dep(chain.find_cell_dep_for_script(&minter_lock_script.clone().unwrap()))
            .output(generate_simple_udt_cell(&sudt_contract))
            .outputs_data(vec![0_u128.to_le_bytes().pack()])
            .build();

        // Create Mint SUDT transaction, using as input a cell locked with a different user's lock script
        // Should fail because the user does not have mint permissions
        let fail_tx = TransactionBuilder::default()
            .input(non_minter_owned_cell_input)
            .cell_dep(
                sudt_contract
                    .as_cell_dep(sudt_code_cell_outpoint.into())
                    .into(),
            )
            .cell_dep(chain.find_cell_dep_for_script(&minter_lock_script.unwrap()))
            .output(generate_simple_udt_cell(&sudt_contract))
            .outputs_data(vec![0_u128.to_le_bytes().pack()])
            .build();

        // Add rule to sudt output generation to increase the amount field.
        sudt_contract.add_output_rule(
            ContractCellFieldSelector::Data,
            |amount: ContractCellField<Byte32, Uint128>| -> ContractCellField<Byte32, Uint128> {
                if let ContractCellField::Data(amount) = amount {
                    let mut amt_bytes = [0u8; 16];
                    amt_bytes.copy_from_slice(amount.as_slice());
                    let amt = u128::from_le_bytes(amt_bytes) + 2000;
                    ContractCellField::Data(amt.pack())
                } else {
                    amount
                }
            },
        );

        // Instantiate chain rpc and tx generator
        let chain_rpc = ChainRpc::new(chain);
        let generator = Generator::new()
            .chain_service(&chain_rpc)
            .pipeline(vec![&sudt_contract]);

        // Generate two transactions
        let new_tx = generator.pipe(tx);
        let new_fail_tx = generator.pipe(fail_tx);

        // Test that success transaction succeeded & has correct sudt amount minted
        let new_tx_amt = new_tx.output_with_data(0).unwrap().1;
        let new_tx_amt: u128 = sudt_contract.read_raw_data(new_tx_amt).unpack();
        assert_eq!(new_tx_amt, 2000_u128);

        let is_valid = chain_rpc.verify_tx(new_tx.into());
        assert!(is_valid);

        // Test that failure transaction failed
        let is_valid = chain_rpc.verify_tx(new_fail_tx.into());
        assert!(!is_valid);
    }

    #[test]
    fn test_update_sudt_with_rule_pipeline() {
        // Load SUDT contract
        let mut sudt_contract = gen_sudt_contract(None, None);
        // Create SUDT Cell Output
        let sudt_cell = generate_simple_udt_cell(&sudt_contract);
        // Mock Transaction with a single output
        let transaction = generate_mock_tx(vec![sudt_cell], vec![2000_u128.to_le_bytes().pack()]);

        // Add output rule to sudt contract to increase balance by 17
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
            },
        );

        // Add output rule to sudt contract to increase balance by 20
        sudt_contract.add_output_rule(
            ContractCellFieldSelector::Data,
            |amount: ContractCellField<Byte32, Uint128>| -> ContractCellField<Byte32, Uint128> {
                if let ContractCellField::Data(amount) = amount {
                    let mut amt_bytes = [0u8; 16];
                    amt_bytes.copy_from_slice(amount.as_slice());
                    let amt = u128::from_le_bytes(amt_bytes) + 20;
                    ContractCellField::Data(amt.pack())
                } else {
                    amount
                }
            },
        );

        // Pipe transaction into sudt contract
        let new_tx = sudt_contract.pipe(transaction);

        // Check that sudt contract updated correctly with a total balance increase of 37 (17 + 20)
        let new_tx_amt = new_tx.output_with_data(0).unwrap().1;
        println!("New tx amt as bytes: {:?}", new_tx_amt.pack());
        let new_tx_amt: u128 = sudt_contract.read_raw_data(new_tx_amt).unpack();
        assert_eq!(new_tx_amt, 2037_u128);
    }
    #[test]
    fn test_add_output_rule() {
        let mut sudt_contract = gen_sudt_contract(None, None);

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
            },
        );
    }
    #[test]
    fn test_contract_pack_and_unpack_data() {
        let mut sudt_contract = gen_sudt_contract(None, None);

        sudt_contract.set_args(Byte32::default());
        sudt_contract.set_data(1200_u128.pack());

        let uint128_data: u128 = sudt_contract.read_data().unpack();
        assert_eq!(uint128_data, 1200_u128);
    }

    #[test]
    fn test_sudt_data_hash_gen_json() {
        let sudt_contract = gen_sudt_contract(None, None);

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
        let sudt_contract = gen_sudt_contract(None, None);

        let code_hash = sudt_contract.data_hash().unwrap().pack();
        let hash_hex_str = format!("0x{}", hex::encode(&code_hash.raw_data().to_vec()));
        assert_eq!(EXPECTED_SUDT_HASH, hash_hex_str.as_str());
    }
}
