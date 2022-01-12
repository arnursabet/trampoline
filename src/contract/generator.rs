use std::rc::Rc;

use ckb_jsonrpc_types::{Byte32, Capacity, OutPoint, Script, TransactionView as JsonTransaction};

use ckb_types::core::{TransactionBuilder, TransactionView};

// Note: Uses ckb_jsonrpc_types
pub trait TransactionProvider {
    fn send_tx(&self, tx: JsonTransaction) -> Option<Byte32>;

    fn verify_tx(&self, tx: JsonTransaction) -> bool;
}

// Note: Uses ckb_types::core::TransactionView; not ckb_jsonrpc_types::TransactionView
pub trait GeneratorMiddleware {
    fn pipe(&self, tx: TransactionView) -> TransactionView;
}

// TODO: implement from for CellQueryAttribute on json_types and packed types
#[derive(Debug, Clone)]
pub enum CellQueryAttribute {
    LockHash(Byte32),
    LockScript(Script),
    TypeScript(Script),
    OutPoint(OutPoint),
    Capacity(Capacity),
}

#[derive(Debug, Clone)]
pub enum QueryStatement {
    Single(CellQueryAttribute),
    And(Rc<QueryStatement>, Rc<QueryStatement>),
    Or(Rc<QueryStatement>, Rc<QueryStatement>),
    // First statement is primary query, second statements filters from query matches
    FilterFrom(Rc<QueryStatement>, Rc<QueryStatement>),
}

#[derive(Debug, Clone)]
pub struct CellQuery {
    _query: QueryStatement,
    _limit: u64,
}

pub trait QueryProvider {
    fn query(&self, query: CellQuery) -> Vec<OutPoint>;
}

#[derive(Default)]
pub struct Generator<'a, 'b> {
    middleware: Vec<&'a dyn GeneratorMiddleware>,
    chain_service: Option<&'b dyn TransactionProvider>,
    query_service: Option<&'b dyn QueryProvider>,
    tx: Option<TransactionView>,
}

impl<'a, 'b> Generator<'a, 'b> {
    pub fn new() -> Self {
        Generator {
            middleware: vec![],
            chain_service: None,
            query_service: None,
            tx: Some(TransactionBuilder::default().build()),
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
        self.query_service
            .map(|query_service| query_service.query(query))
    }
    pub fn generate(&self) -> TransactionView {
        self.pipe(self.tx.as_ref().unwrap().clone())
    }
}

impl GeneratorMiddleware for Generator<'_, '_> {
    fn pipe(&self, tx: TransactionView) -> TransactionView {
        self.middleware
            .iter()
            .fold(tx, |tx, middleware| middleware.pipe(tx))
    }
}