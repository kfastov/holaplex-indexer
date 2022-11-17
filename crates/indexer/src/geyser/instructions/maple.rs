use indexer_core::{
    // db::{tables::metadatas, update},
    prelude::*,
};

use super::Client;
use crate::prelude::*;



pub(crate) async fn process_withdrawal_instruction(
    _client: &Client,
    _accounts: &[Pubkey],
    _slot: u64,
) -> Result<()> {
    /*
    "accounts": [
        {
          "name": "lender", // 1
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "lenderOwner", // 2
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "pool", // 3
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "globals", // 4
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "sharesMint", // 5
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "lenderShareAccount", // 6
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "withdrawalRequest", // 7
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "withdrawalRequestLocker", // 8
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram", // 9
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram", // 10
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",  // 11
          "isMut": false,
          "isSigner": false
        }
      ],
    "args": [
        {
          "name": "nonce",
          "type": {
            "defined": "Nonce"
          }
        },
        {
          "name": "shares",
          "type": "u64"
        }
      ]


     */

    // let lender_owner = accounts[1].to_string();
    // let pool = accounts[2].to_string();
    // let withdrawal_request = accounts[0].to_string();

    // What does this instruction (https://explorer.solana.com/tx/5ndP6W4XSyrrDt54BYgzrbH93xPMnB2RpXmGnEQrF8irxmAwrLdw1LEPdBhTESsGpQ5tq7nBkES6SfYjLUbYp5jd) do?
    // 1. Creates new Withdrawal Request account (2e9otkD6z4hCyxnfL2gY5PAjnBg4S7i8fYSPgkZrdGAP)
    // 2. Creates new Withdrawal Request locker token account (835rgAVagDpntYUVuYr8e6M16HVQb1khrLaXcoxpXMAg), owned by the Pool (TamdAwg85s9aZ6mwSeAHoczzAV53rFokL5FVKzaF1Tb)
    // 3. Transfers 19,888.549577 tokens from the Lender Share account (HtktPfqFxrVojnEaq9pP415DcdFdRhXUFUjikSKHMpe6) to Withdrawal Request locker account



    // client
    //     .db()
    //     .run(move |db| {
    //         update(metadatas::table.filter(metadatas::mint_address.eq(mint)))
    //             .set((
    //                 metadatas::burned_at.eq(Some(Local::now().naive_utc())),
    //                 metadatas::slot.eq(slot),
    //             ))
    //             .execute(db)
    //     })
    //     .await
    //     .context("failed to update metadata")?;

    Ok(())
}
