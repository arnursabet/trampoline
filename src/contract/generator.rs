
use ckb_jsonrpc_types::{
    Byte32, Capacity, OutPoint, Script, TransactionView as JsonTransaction,
};

use ckb_types::{
    core::{TransactionBuilder, TransactionView},
    packed::CellInputBuilder,
    prelude::*,
};

use std::sync::{Arc, Mutex};

// Note: Uses ckb_jsonrpc_types
pub trait TransactionProvider {
    fn send_tx(&self, tx: JsonTransaction) -> Option<Byte32>;

    fn verify_tx(&self, tx: JsonTransaction) -> bool;
}

// Note: Uses ckb_types::core::TransactionView; not ckb_jsonrpc_types::TransactionView
pub trait GeneratorMiddleware {
    fn pipe(
        &self,
        tx: TransactionView,
        query_register: Arc<Mutex<Vec<CellQuery>>>,
    ) -> TransactionView;

}

// TODO: implement from for CellQueryAttribute on json_types and packed types
#[derive(Debug, Clone)]
pub enum CellQueryAttribute {
    LockHash(Byte32),
    LockScript(Script),
    TypeScript(Script),
    MinCapacity(Capacity),
    MaxCapacity(Capacity),
}

#[derive(Debug, Clone)]
pub enum QueryStatement {
    Single(CellQueryAttribute),
    FilterFrom(CellQueryAttribute, CellQueryAttribute),
    Any(Vec<CellQueryAttribute>),
    All(Vec<CellQueryAttribute>),
}

#[derive(Debug, Clone)]
pub struct CellQuery {
    pub _query: QueryStatement,
    pub _limit: u64,
}

pub trait QueryProvider {
    fn query(&self, query: CellQuery) -> Option<Vec<OutPoint>>;
}

#[derive(Default)]
pub struct Generator<'a, 'b> {
    middleware: Vec<&'a dyn GeneratorMiddleware>,
    chain_service: Option<&'b dyn TransactionProvider>,
    query_service: Option<&'b dyn QueryProvider>,
    tx: Option<TransactionView>,
    query_queue: Arc<Mutex<Vec<CellQuery>>>,
}

impl<'a, 'b> Generator<'a, 'b> {
    pub fn new() -> Self {
        Generator {
            middleware: vec![],
            chain_service: None,
            query_service: None,
            tx: Some(TransactionBuilder::default().build()),
            query_queue: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn pipeline(mut self, pipes: Vec<&'a dyn GeneratorMiddleware>) -> Self {
        self.middleware = pipes;
        self
    }

    pub fn chain_service(mut self, chain_service: &'b dyn TransactionProvider) -> Self {
        self.chain_service = Some(chain_service);
        self
    }

    pub fn query_service(mut self, query_service: &'b dyn QueryProvider) -> Self {
        self.query_service = Some(query_service);
        self
    }

    pub fn query(&self, query: CellQuery) -> Option<Vec<OutPoint>> {
        let res = self.query_service.unwrap().query(query.clone());
        println!(
            "Res in generator.query for cell_query {:?} is {:?}",
            query, res
        );
        res
    }
  
    pub fn generate(&self) -> TransactionView {
        self.pipe(self.tx.as_ref().unwrap().clone(), self.query_queue.clone())
    }
}

impl GeneratorMiddleware for Generator<'_, '_> {
    fn pipe(
        &self,
        tx: TransactionView,
        query_register: Arc<Mutex<Vec<CellQuery>>>,
    ) -> TransactionView {
        let res = self.middleware.iter().fold(tx, |tx, middleware| {
            middleware.pipe(tx, query_register.clone())
        });
       
        let inputs = query_register
            .lock()
            .unwrap()
            .iter()
            .map(|query| self.query(query.to_owned()).unwrap())
            .flatten()
            .map(|outp| {
                CellInputBuilder::default()
                    .previous_output(outp.into())
                    .build()
            })
            .collect::<Vec<_>>();

        println!("GENERATED INPUTS: {:?}", inputs);
        res.as_advanced_builder().set_inputs(inputs).build()
    }
}
