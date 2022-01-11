use crate::contract::{Contract};
use ckb_types::core::TransactionView;
pub trait TransactionProvider {
    fn send_tx(&self) -> ();

    fn build_tx(&self) -> ();

    fn sign_tx(&self) -> ();

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
    pub fn set_pipeline(&mut self, pipes: Vec<&'a dyn GeneratorMiddleware>) {
        self.middleware = pipes;
    }

    pub fn set_chain_service(&mut self, chain_service: &'b dyn TransactionProvider) {
        self.chain_service = Some(chain_service);
    }

    pub fn set_query_service(&mut self, query_service: &'b dyn QueryProvider) {
        self.query_service = Some(query_service);
    }


}