
use ckb_types::core::{TransactionView, TransactionBuilder};
use ckb_types::H256;
pub trait TransactionProvider {
    fn send_tx(&self) -> Option<H256>;

    fn verify_tx(&self) -> bool;

}

pub trait GeneratorMiddleware {
    fn pipe(&self, tx: TransactionView) -> TransactionView;
}

pub trait QueryProvider {
    fn get_live_cell(&self) -> ();

    fn get_live_cells(&self) -> ();
}

#[derive(Default)]
pub struct Generator<'a, 'b> {
    middleware: Vec<&'a dyn GeneratorMiddleware>,
    chain_service: Option<&'b dyn TransactionProvider>,
    query_service: Option<&'b dyn QueryProvider>,
    tx: Option<TransactionView>
}

impl<'a, 'b> Generator<'a, 'b> {
    pub fn new() -> Self {
        Generator {
            middleware: vec![],
            chain_service: None,
            query_service: None,
            tx: Some(TransactionBuilder::default().build())
        }
    }

    pub fn set_pipeline(&mut self, pipes: Vec<&'a dyn GeneratorMiddleware>) {
        self.middleware = pipes;
    }

    pub fn set_chain_service(&mut self, chain_service: &'b dyn TransactionProvider) {
        self.chain_service = Some(chain_service);
    }

    pub fn set_query_service(&mut self, query_service: &'b dyn QueryProvider) {
        self.query_service = Some(query_service);
    }

    pub fn generate(&self) -> TransactionView {
        self.middleware.iter().fold(self.tx.as_ref().unwrap().clone(), |tx, middleware| {
            middleware.pipe(tx)
        })
    }
}