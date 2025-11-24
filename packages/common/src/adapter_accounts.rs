use anchor_lang::prelude::{sysvar::SysvarId, AccountMeta, Clock, Pubkey};

use crate::{
    pda,
    wormhole_adapter::{
        self,
        constants::{
            CORE_BRIDGE_CONFIG, CORE_BRIDGE_FEE_COLLECTOR, CORE_BRIDGE_PROGRAM_ID, EMITTER_SEED,
            GLOBAL_SEED, SEQUENCE_SEED,
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
    pub wormhole_post_message_shi: Pubkey,
}

impl Default for WormholeRemainingAccounts {
    fn default() -> Self {
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
                &[b"__event_authority"],
                &wormhole_post_message_shim::ID
            ),
            wormhole_post_message_shi: wormhole_post_message_shim::ID,
        }
    }
}

impl WormholeRemainingAccounts {
    pub fn account_metas() -> Vec<AccountMeta> {
        Self::default().to_account_metas()
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
            AccountMeta::new_readonly(self.wormhole_post_message_shi, false),
        ]
    }
}
