use std::{collections::HashSet, hash::BuildHasher};

use crate::types::transaction::Transaction;
use ckb_types::{
    core::cell::{ResolveOptions, ResolvedTransaction},
    packed::OutPoint,
};

use super::provider::RpcProvider;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionResolverError {
    #[error("Failed to resolve transaction: dead outpoint")]
    DeadOutPoint(OutPoint),
    #[error("Failed to resolve transaction: unknown outpoint")]
    UnknownOutPoint(OutPoint),
}

pub trait TransactionResolver {
    fn resolve_tx(&self, tx: Transaction) -> Result<ResolvedTransaction, TransactionResolverError>;
}
