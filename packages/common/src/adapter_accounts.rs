use std::str::FromStr;

use anchor_lang::prelude::{AccountMeta, Clock, Pubkey};
use anchor_lang::solana_program::sysvar::SysvarId;

use crate::consts::{
    HYPERLANE_MAILBOX_PROGRAM_ID, HYPERLANE_MAILBOX_PROGRAM_ID_TESTNET, WORMHOLE_BRIDGE_CONFIG,
    WORMHOLE_BRIDGE_CONFIG_DEVNET, WORMHOLE_BRIDGE_FEE_COLLECTOR,
    WORMHOLE_BRIDGE_FEE_COLLECTOR_DEVNET, WORMHOLE_BRIDGE_PROGRAM_ID,
    WORMHOLE_BRIDGE_PROGRAM_ID_DEVNET,
};
use crate::{
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::{
            DASH_SEED, DISPATCHED_MESSAGE_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2, GAS_PAYMENT_SEED,
            HYPERLANE_IGP_SEED, HYPERLANE_SEED, OUTBOX_SEED, PROGRAM_DATA_SEED,
            UNIQUE_MESSAGE_SEED,
        },
    },
    pda,
    wormhole_adapter::{
        self,
        constants::{EMITTER_SEED, EVENT_AUTHORITY_SEED, GLOBAL_SEED, SEQUENCE_SEED},
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
    pub fn new(devnet: bool) -> Self {
        let emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID);

        let (bridge_config, bridge_program_id, fee_collector) = if devnet {
            (
                WORMHOLE_BRIDGE_CONFIG_DEVNET,
                WORMHOLE_BRIDGE_PROGRAM_ID_DEVNET,
                WORMHOLE_BRIDGE_FEE_COLLECTOR_DEVNET,
            )
        } else {
            (
                WORMHOLE_BRIDGE_CONFIG,
                WORMHOLE_BRIDGE_PROGRAM_ID,
                WORMHOLE_BRIDGE_FEE_COLLECTOR,
            )
        };

        Self {
            wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
            bridge: bridge_config,
            message_account: pda!(&[&emitter.to_bytes()], &wormhole_post_message_shim::ID),
            emitter,
            sequence: pda!(&[SEQUENCE_SEED, &emitter.to_bytes()], &bridge_program_id),
            fee_collector: fee_collector,
            clock: Clock::id(),
            wormhole_program: bridge_program_id,
            wormhole_post_message_shim_ea: pda!(
                &[EVENT_AUTHORITY_SEED],
                &wormhole_post_message_shim::ID
            ),
            wormhole_post_message_shim: wormhole_post_message_shim::ID,
        }
    }

    pub fn account_metas(devnet: bool) -> Vec<AccountMeta> {
        Self::new(devnet).to_account_metas()
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
    pub hyperlane_user_global: Pubkey,
    pub unique_message: Pubkey,
    pub dispatched_message: Pubkey,
    pub igp_program_id: Pubkey,
    pub igp_program_data: Pubkey,
    pub igp_gas_payment: Pubkey,
    pub igp_account: Pubkey,
    pub igp_overhead_account: Option<Pubkey>,
    pub mailbox_program: Pubkey,
    pub spl_noop_program: Pubkey,
}

impl HyperlaneRemainingAccounts {
    pub fn new(
        payer: &Pubkey,
        global: &HyperlaneGlobal,
        user_global: Option<&HyperlaneUserGlobal>,
        testnet: bool,
    ) -> Self {
        let mailbox_program_id = if testnet {
            HYPERLANE_MAILBOX_PROGRAM_ID_TESTNET
        } else {
            HYPERLANE_MAILBOX_PROGRAM_ID
        };

        let hyperlane_user_global = pda!(
            &[GLOBAL_SEED, DASH_SEED, payer.as_ref()],
            &hyperlane_adapter::ID
        );

        let unique_message = pda!(
            &[
                UNIQUE_MESSAGE_SEED,
                hyperlane_user_global.as_ref(),
                &user_global
                    .map(|g| g.nonce)
                    .unwrap_or_default()
                    .to_le_bytes()
            ],
            &hyperlane_adapter::ID
        );

        Self {
            hyperlane_global: pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID),
            mailbox_outbox: pda!(
                &[HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
                &mailbox_program_id
            ),
            dispatch_authority: pda!(
                &[DISPATCH_SEED_1, DASH_SEED, DISPATCH_SEED_2],
                &hyperlane_adapter::ID
            ),
            hyperlane_user_global,
            unique_message,
            dispatched_message: pda!(
                &[
                    HYPERLANE_SEED,
                    DASH_SEED,
                    DISPATCHED_MESSAGE_SEED,
                    DASH_SEED,
                    unique_message.as_ref(),
                ],
                &mailbox_program_id
            ),
            igp_program_id: global.igp_program_id,
            igp_program_data: pda!(
                &[HYPERLANE_IGP_SEED, DASH_SEED, PROGRAM_DATA_SEED],
                &global.igp_program_id
            ),
            igp_gas_payment: pda!(
                &[
                    HYPERLANE_IGP_SEED,
                    DASH_SEED,
                    GAS_PAYMENT_SEED,
                    DASH_SEED,
                    unique_message.as_ref()
                ],
                &global.igp_program_id
            ),
            igp_account: global.igp_account,
            igp_overhead_account: global.igp_overhead_account,
            mailbox_program: mailbox_program_id,
            spl_noop_program: Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV")
                .unwrap(),
        }
    }

    pub fn account_metas(
        payer: &Pubkey,
        global: &HyperlaneGlobal,
        user_global: Option<&HyperlaneUserGlobal>,
        testnet: bool,
    ) -> Vec<AccountMeta> {
        Self::new(payer, global, user_global, testnet).to_account_metas()
    }

    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut accounts = vec![
            AccountMeta::new(self.hyperlane_global, false),
            AccountMeta::new(self.mailbox_outbox, false),
            AccountMeta::new_readonly(self.dispatch_authority, false),
            AccountMeta::new(self.hyperlane_user_global, false),
            AccountMeta::new_readonly(self.unique_message, false),
            AccountMeta::new(self.dispatched_message, false),
            AccountMeta::new_readonly(self.igp_program_id, false),
            AccountMeta::new(self.igp_program_data, false),
            AccountMeta::new(self.igp_gas_payment, false),
            AccountMeta::new(self.igp_account, false),
            AccountMeta::new_readonly(self.mailbox_program, false),
            AccountMeta::new_readonly(self.spl_noop_program, false),
        ];

        if let Some(igp_overhead_account) = self.igp_overhead_account {
            accounts.push(AccountMeta::new_readonly(igp_overhead_account, false));
        }

        accounts
    }
}
