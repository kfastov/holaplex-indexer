use indexer_core::{
    // db::{insert_into, models::CurrentMetadataOwner, tables::current_metadata_owners, update},
    prelude::*,
};
use syrup_cpi::{Globals, Lender, Loan, OpenTermLoan, Pool, WithdrawalRequest};

use super::Client;
use crate::prelude::*;

pub async fn process_globals(
    _client: &Client,
    key: Pubkey,
    _globals: Globals,
    _slot: u64,
) -> Result<()> {
    debug!("processing globals account {}", key);
    Ok(())
}

pub async fn process_lender(_client: &Client, key: Pubkey, _lender: Lender, _slot: u64) -> Result<()> {
    debug!("processing lender account {}", key);
    Ok(())
}

pub async fn process_loan(_client: &Client, key: Pubkey, _loan: Loan, _slot: u64) -> Result<()> {
    debug!("processing loan account {}", key);
    Ok(())
}

pub async fn process_open_term_loan(
    _client: &Client,
    key: Pubkey,
    _loan: OpenTermLoan,
    _slot: u64,
) -> Result<()> {
    debug!("processing open term loan account {}", key);
    Ok(())
}

pub async fn process_pool(_client: &Client, key: Pubkey, _pool: Pool, _slot: u64) -> Result<()> {
    debug!("processing pool account {}", key);
    Ok(())
}

pub async fn process_withdrawal_request(
    _client: &Client,
    key: Pubkey,
    _request: WithdrawalRequest,
    _slot: u64,
) -> Result<()> {
    debug!("processing withdrawal request account {}", key);
    Ok(())
}
