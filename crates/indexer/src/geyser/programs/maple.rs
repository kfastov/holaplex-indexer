use anchor_lang_v0_24::AccountDeserialize;
// use solana_program::program_pack::Pack;
use syrup_cpi::{Globals, Lender, Loan, OpenTermLoan, Pool, WithdrawalRequest};

use crate::prelude::*;

const GLOBALS_SIZE: usize = 1226;
const LENDER_SIZE: usize = 240;
const LOAN_SIZE: usize = 376;
const OPEN_TERM_LOAN_SIZE: usize = 432;
const POOL_SIZE: usize = 397;
const WITHDRAWAL_REQUEST_SIZE: usize = 216;

// instruction ids
const WITHDRAWAL_REQUEST_INITIALIZE: u8 = 21; // or not?

use super::{accounts::maple, instructions::maple as maple_instruction, AccountUpdate, Client};
// use crate::prelude::*;

async fn process_globals(client: &Client, update: AccountUpdate) -> Result<()> {
    let globals = Globals::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize globals account!")?;

    maple::process_globals(client, update.key, globals, update.slot).await
}

async fn process_lender(client: &Client, update: AccountUpdate) -> Result<()> {
    let lender = Lender::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize lender account!")?;

    maple::process_lender(client, update.key, lender, update.slot).await
}

async fn process_loan(client: &Client, update: AccountUpdate) -> Result<()> {
    let loan = Loan::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize loan account!")?;

    maple::process_loan(client, update.key, loan, update.slot).await
}

async fn process_open_term_loan(client: &Client, update: AccountUpdate) -> Result<()> {
    let loan = OpenTermLoan::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize loan account!")?;

    maple::process_open_term_loan(client, update.key, loan, update.slot).await
}

async fn process_pool(client: &Client, update: AccountUpdate) -> Result<()> {
    let pool = Pool::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize pool account!")?;

    maple::process_pool(client, update.key, pool, update.slot).await
}

async fn process_withdrawal_request(client: &Client, update: AccountUpdate) -> Result<()> {
    let request = WithdrawalRequest::try_deserialize_unchecked(&mut update.data.as_slice())
        .context("failed to deserialize withdrawal request account!")?;

    maple::process_withdrawal_request(client, update.key, request, update.slot).await
}

// TODO use anchor discriminator instead of relying on account length
pub(crate) async fn process(client: &Client, update: AccountUpdate) -> Result<()> {
    match update.data.len() {
        GLOBALS_SIZE => process_globals(client, update).await,
        LENDER_SIZE => process_lender(client, update).await,
        LOAN_SIZE => process_loan(client, update).await,
        OPEN_TERM_LOAN_SIZE => process_open_term_loan(client, update).await,
        POOL_SIZE => process_pool(client, update).await,
        WITHDRAWAL_REQUEST_SIZE => process_withdrawal_request(client, update).await,
        _ => Ok(()),
    }
}

pub(crate) async fn process_instruction(
    client: &Client,
    data: &[u8],
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    let (&discriminator, _) = data
        .split_first()
        .context("invalid spl token instruction")?;
    debug!("Maple ix discriminator: {}", discriminator);
    match discriminator {
        WITHDRAWAL_REQUEST_INITIALIZE => {
            maple_instruction::process_withdrawal_instruction(client, accounts, slot).await
        },
        _ => Ok(()),
    }
}
