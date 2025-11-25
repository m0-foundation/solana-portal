use anchor_lang::prelude::{sysvar::SysvarId, AccountMeta, Clock, Pubkey};

use crate::{
    hyperlane_adapter::{
        self,
        constants::{
            DASH_SEED, DISPATCHED_MESSAGE_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2, HYPERLANE_SEED,
            MAILBOX_PROGRAM_ID, OUTBOX_SEED, SPL_NOOP, UNIQUE_MESSAGE_SEED,
        },
    },
    pda,
    wormhole_adapter::{
        self,
        constants::{
            CORE_BRIDGE_CONFIG, CORE_BRIDGE_FEE_COLLECTOR, CORE_BRIDGE_PROGRAM_ID, EMITTER_SEED,
            EVENT_AUTHORITY_SEED, GLOBAL_SEED, SEQUENCE_SEED,
        },
    },
    wormhole_post_message_shim,
};

pub struct WormholeRemainingAccounts {
    pub wormhole_global: Pubkey,
    pub bridge: Pubkey,
    pub message_account: Pubkey,
    pub emitter: Pubkey,
    pub sequence: Pubkey,
    pub fee_collector: Pubkey,
    pub clock: Pubkey,
    pub wormhole_program: Pubkey,
    pub wormhole_post_message_shim_ea: Pubkey,
    pub wormhole_post_message_shim: Pubkey,
}

impl WormholeRemainingAccounts {
    pub fn new() -> Self {
        let emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID);

        Self {
            wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
            bridge: CORE_BRIDGE_CONFIG,
            message_account: pda!(&[&emitter.to_bytes()], &wormhole_post_message_shim::ID),
            emitter,
            sequence: pda!(
                &[SEQUENCE_SEED, &emitter.to_bytes()],
                &CORE_BRIDGE_PROGRAM_ID
            ),
            fee_collector: CORE_BRIDGE_FEE_COLLECTOR,
            clock: Clock::id(),
            wormhole_program: CORE_BRIDGE_PROGRAM_ID,
            wormhole_post_message_shim_ea: pda!(
                &[EVENT_AUTHORITY_SEED],
                &wormhole_post_message_shim::ID
            ),
            wormhole_post_message_shim: wormhole_post_message_shim::ID,
        }
    }

    pub fn account_metas() -> Vec<AccountMeta> {
        Self::new().to_account_metas()
    }

    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.wormhole_global, false),
            AccountMeta::new(self.bridge, false),
            AccountMeta::new(self.message_account, false),
            AccountMeta::new_readonly(self.emitter, false),
            AccountMeta::new(self.sequence, false),
            AccountMeta::new(self.fee_collector, false),
            AccountMeta::new_readonly(self.clock, false),
            AccountMeta::new_readonly(self.wormhole_program, false),
            AccountMeta::new_readonly(self.wormhole_post_message_shim_ea, false),
            AccountMeta::new_readonly(self.wormhole_post_message_shim, false),
        ]
    }
}

pub struct HyperlaneRemainingAccounts {
    pub hyperlane_global: Pubkey,
    pub mailbox_outbox: Pubkey,
    pub dispatch_authority: Pubkey,
    pub unique_message: Pubkey,
    pub dispatched_message: Pubkey,
    pub mailbox_program: Pubkey,
    pub spl_noop_program: Pubkey,
}

impl HyperlaneRemainingAccounts {
    pub fn new(global_nonce: u64) -> Self {
        let unique_message = pda!(
            &[UNIQUE_MESSAGE_SEED, global_nonce.to_le_bytes().as_ref()],
            &hyperlane_adapter::ID
        );

        Self {
            hyperlane_global: pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID),
            mailbox_outbox: pda!(
                &[HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
                &MAILBOX_PROGRAM_ID
            ),
            dispatch_authority: pda!(
                &[DISPATCH_SEED_1, DASH_SEED, DISPATCH_SEED_2],
                &hyperlane_adapter::ID
            ),
            unique_message,
            dispatched_message: pda!(
                &[
                    HYPERLANE_SEED,
                    DASH_SEED,
                    DISPATCHED_MESSAGE_SEED,
                    DASH_SEED,
                    unique_message.as_ref(),
                ],
                &MAILBOX_PROGRAM_ID
            ),
            mailbox_program: MAILBOX_PROGRAM_ID,
            spl_noop_program: SPL_NOOP,
        }
    }

    pub fn account_metas(global_nonce: u64) -> Vec<AccountMeta> {
        Self::new(global_nonce).to_account_metas()
    }

    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.hyperlane_global, false),
            AccountMeta::new(self.mailbox_outbox, false),
            AccountMeta::new_readonly(self.dispatch_authority, false),
            AccountMeta::new_readonly(self.unique_message, false),
            AccountMeta::new(self.dispatched_message, false),
            AccountMeta::new_readonly(self.mailbox_program, false),
            AccountMeta::new_readonly(self.spl_noop_program, false),
        ]
    }
}
