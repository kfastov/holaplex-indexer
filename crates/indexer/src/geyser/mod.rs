//! Support features for the Geyser indexer

mod accounts;
mod client;
mod instructions;
mod programs;

use std::{collections::HashSet, fmt, sync::Arc};

pub use client::{Args as ClientArgs, Client};
use indexer_core::pubkeys;
pub(self) use indexer_rabbitmq::geyser::AccountUpdate;
use indexer_rabbitmq::geyser::Message;

use crate::prelude::*;

/// A value indicating a specific topic to ignore
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum IgnoreType {
    /// Ignore the metadata program
    Metadata,
    /// Ignore the Metaplex candy machine program
    CandyMachine,
    /// Ignore the SPL token program
    Tokens,
}

/// Message identifier
#[derive(Debug, Clone, Copy)]
pub enum MessageId {
    /// An update of an account with the given key
    AccountUpdate(Pubkey),
    /// An instruction from the program with the given key
    Instruction(Pubkey),
    /// A status update of the slot with the given ID
    SlotStatus(u64),
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AccountUpdate(k) => write!(f, "account update for {k}"),
            Self::Instruction(p) => write!(f, "instruction from program {p}"),
            &Self::SlotStatus(s) => write!(f, "status update for slot {s}"),
        }
    }
}

/// Process a message from a Geyser RabbitMQ queue
///
/// # Errors
/// This function fails if an error occurs processing the message body.
#[allow(clippy::too_many_lines)]
pub async fn process_message<H: std::hash::BuildHasher>(
    msg: Message,
    client: &Client,
    ignore_on_startup: Arc<HashSet<IgnoreType, H>>,
) -> MessageResult<MessageId> {
    let check_ignore =
        |ty, update: &AccountUpdate| !(update.is_startup && ignore_on_startup.contains(&ty));

    let id = match msg {
        Message::AccountUpdate(ref u) => MessageId::AccountUpdate(u.key),
        Message::InstructionNotify(ref i) => MessageId::Instruction(i.program),
        Message::SlotStatusUpdate(ref s) => MessageId::SlotStatus(s.slot),
    };

    match msg {
        // Accounts
        Message::AccountUpdate(update)
            if update.owner == pubkeys::METADATA && check_ignore(IgnoreType::Metadata, &update) =>
        {
            programs::metadata::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::REWARD_CENTER => {
            programs::reward_center::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::MAPLE => {
            programs::maple::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::AUCTION => {
            programs::auction::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::METAPLEX => {
            programs::metaplex::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::AUCTION_HOUSE => {
            programs::auction_house::process(client, update).await
        },
        Message::AccountUpdate(update)
            if update.owner == pubkeys::TOKEN && check_ignore(IgnoreType::Tokens, &update) =>
        {
            programs::token::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::GRAPH_PROGRAM => {
            programs::graph::process(client, update).await
        },
        Message::AccountUpdate(update)
            if update.owner == pubkeys::CANDY_MACHINE
                && check_ignore(IgnoreType::CandyMachine, &update) =>
        {
            programs::candy_machine::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::NAME_SERVICE => {
            programs::name_service::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::CARDINAL_TOKEN_MANAGER => {
            programs::cardinal_token_manager::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::CARDINAL_TIME_INVALIDATOR => {
            programs::cardinal_time_invalidator::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::CARDINAL_USE_INVALIDATOR => {
            programs::cardinal_use_invalidator::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::CARDINAL_PAID_CLAIM_APPROVER => {
            programs::cardinal_paid_claim_approver::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::GOKI_SMART_WALLET => {
            programs::goki_smart_wallet::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::TRIBECA_LOCKED_VOTER => {
            programs::tribeca_locked_voter::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::TRIBECA_GOVERN => {
            programs::tribeca_govern::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::NAMESPACES => {
            programs::namespaces::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == pubkeys::TOKEN_BONDING => {
            programs::token_bonding::process(client, update).await
        },
        Message::AccountUpdate(update) if pubkeys::SPL_GOVERNANCE.contains(&update.owner) => {
            programs::spl_governance::process(client, update).await
        },
        Message::AccountUpdate(update) if update.owner == genostub::ID => {
            programs::genopets::process(client, update).await
        },

        // Instructions
        Message::InstructionNotify(ins) if ins.program == pubkeys::AUCTION_HOUSE => {
            programs::auction_house::process_instruction(client, &ins.data, &ins.accounts, ins.slot)
                .await
        },
        Message::InstructionNotify(ins) if ins.program == pubkeys::REWARD_CENTER => {
            programs::reward_center::process_instruction(client, &ins.data, &ins.accounts, ins.slot)
                .await
        },
        Message::InstructionNotify(ins) if ins.program == pubkeys::ME_HAUS => {
            programs::magic_eden_haus::process_instruction(
                client,
                &ins.data,
                &ins.accounts,
                ins.slot,
            )
            .await
        },
        Message::InstructionNotify(ins) if ins.program == pubkeys::TOKEN => {
            programs::token::process_instruction(client, &ins.data, &ins.accounts, ins.slot).await
        },
        Message::InstructionNotify(ins) if ins.program == pubkeys::MAPLE => {
            programs::maple::process_instruction(client, &ins.data, &ins.accounts, ins.slot).await
        },

        // Other
        Message::SlotStatusUpdate(slot) => {
            debug!("Slot status update: {:?}", slot);
            Ok(())
        },

        // Fallbacks
        Message::AccountUpdate(update) => {
            debug!(
                "Unhandled account update for program {}",
                bs58::encode(update.owner).into_string()
            );
            Ok(())
        },
        Message::InstructionNotify { .. } => Ok(()),
    }
    .map_err(|e| MessageError::new(e, id))
}
