use indexer_core::{
    db::{insert_into, models::CurrentMetadataOwner, tables::current_metadata_owners, update},
    prelude::*,
};
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};

use super::Client;
use crate::prelude::*;

pub async fn process(
    client: &Client,
    key: Pubkey,
    token_account: TokenAccount,
    slot: u64,
) -> Result<()> {
    let pubkey = key.to_string();

    if token_account.amount > 1 {
        client
            .dispatch_fungible_token(
                token_account.owner,
                key,
                token_account.mint,
                token_account.amount,
            )
            .await?;
        return Ok(());
    }

    let owner = token_account.owner.to_string();
    let mint_address = token_account.mint.to_string();
    let incoming_slot: i64 = slot.try_into()?;

    let values = CurrentMetadataOwner {
        mint_address: Owned(mint_address),
        owner_address: Owned(owner),
        token_account_address: Owned(pubkey),
        slot: incoming_slot,
    };

    client
        .db()
        .run(move |db| {
            let rows = current_metadata_owners::table
                .select((
                    current_metadata_owners::mint_address,
                    current_metadata_owners::owner_address,
                    current_metadata_owners::token_account_address,
                    current_metadata_owners::slot,
                ))
                .filter(current_metadata_owners::mint_address.eq(token_account.mint.to_string()))
                .load::<CurrentMetadataOwner>(db)
                .context("failed to load metadata owner!")?;

            match rows.get(0) {
                Some(r) if incoming_slot > r.slot => {
                    db.build_transaction().read_write().run(|| {
                        update(
                            current_metadata_owners::table
                                .filter(current_metadata_owners::mint_address.eq(values.clone().mint_address)),
                        )
                        .set(&values)
                        .execute(db)
                        .context("transaction failed! unable to update metadata_owners when incoming slot > indexed slot")
                        .map(|_| ())
                    })
                },
                Some(_) => Ok(()),
                None => {
                    db.build_transaction()
                        .read_write()
                        .run(|| {
                            insert_into(current_metadata_owners::table)
                                .values(&values)
                                .on_conflict(current_metadata_owners::mint_address)
                                .do_update()
                                .set(&values)
                                .execute(db)
                                .map(|_| ())
                        })
                        .context("transaction failed! unable to insert metadata owner")?;

                    Ok(())
                },
            }
        })
        .await
        .context("failed to insert token metadata owner!")?;
    Ok(())
}

pub async fn process_mint(
    client: &Client,
    key: Pubkey,
    mint_account: MintAccount,
    _slot: u64,
) -> Result<()> {
    if !mint_account.is_initialized || mint_account.supply == 1 {
        // Looks like NFT or uninitialized account, skip it
        return Ok(());
    }

    let supply = mint_account.supply;
    let decimals = mint_account.decimals;
    let mint_authority = mint_account.mint_authority;

    client
        .dispatch_fungible_token_mint(mint_authority.into(), key, decimals, supply)
        .await?;
    Ok(())
}
