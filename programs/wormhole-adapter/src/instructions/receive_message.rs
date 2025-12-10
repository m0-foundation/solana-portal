use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account::solana_program::keccak;
use common::{
    portal::{self, accounts::PortalGlobal, program::Portal},
    wormhole_verify_vaa_shim::{self, cpi::accounts::VerifyHash, program::WormholeVerifyVaaShim}
};

use crate::{
    consts::{AUTHORITY_SEED, CORE_BRIDGE_PROGRAM_ID, GUARDIAN_SET_SEED},
    instructions::VaaBody,
    state::{WormholeGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
#[instruction(guardian_set_index: u32)]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = portal::ID,
        bump = portal_global.bump,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump
    )]
    /// CHECK: Account does not hold data
    pub wormhole_adapter_authority: AccountInfo<'info>,

    #[account(
        seeds = [AUTHORITY_SEED],
        seeds::program = portal::ID,
        bump
    )]
    /// CHECK: Account does not hold data
    pub portal_authority: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Initialized and verified in CPI to Portal
    pub message_account: AccountInfo<'info>,

    #[account(
        seeds = [GUARDIAN_SET_SEED, &guardian_set_index.to_be_bytes()],
        seeds::program = CORE_BRIDGE_PROGRAM_ID,
        bump
    )]
    /// CHECK: Guardian set used for signature verification by shim
    pub guardian_set: UncheckedAccount<'info>,

    /// CHECK: Stored guardian signatures to be verified by shim
    pub guardian_signatures: UncheckedAccount<'info>,

    pub wormhole_verify_vaa_shim: Program<'info, WormholeVerifyVaaShim>,

    pub portal_program: Program<'info, Portal>,

    pub system_program: Program<'info, System>,
}

impl ReceiveMessage<'_> {
    fn validate(&self, guardian_set_bump: u8, vaa_body: &Vec<u8>) -> Result<()> {
        // Compute the message hash.
        let message_hash = &keccak::hashv(&[&vaa_body]).to_bytes();
        let digest = keccak::hash(message_hash.as_slice()).to_bytes();

        // Verify the hash against the signatures.
        wormhole_verify_vaa_shim::cpi::verify_hash(
            CpiContext::new(
                self.wormhole_verify_vaa_shim.to_account_info(),
                VerifyHash {
                    guardian_set: self.guardian_set.to_account_info(),
                    guardian_signatures: self.guardian_signatures.to_account_info(),
                },
            ),
            guardian_set_bump,
            digest,
        )?;

        // Parse and verify vaa
        let vaa = VaaBody::from_bytes(vaa_body)?;
        self.wormhole_global.validate(&vaa)?;

        Ok(())
    }

    #[access_control(ctx.accounts.validate(ctx.bumps.guardian_set, &vaa_body))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        #[allow(unused_variables)] guardian_set_index: u32,
        vaa_body: Vec<u8>,
    ) -> Result<()> {
        let vm = VaaBody::from_bytes(&vaa_body)?;
        let payload = vm.payload;

        portal::cpi::receive_message(
            CpiContext::new_with_signer(
                ctx.accounts.portal_program.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    payer: ctx.accounts.relayer.to_account_info(),
                    portal_global: ctx.accounts.portal_global.to_account_info(),
                    adapter_authority: ctx.accounts.wormhole_adapter_authority.to_account_info(),
                    message_account: ctx.accounts.message_account.to_account_info(),
                    portal_authority: ctx.accounts.portal_authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.wormhole_adapter_authority]]],
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
            vm.emitter_chain.into(),
            payload.message_id(),
            payload.encode(),
        )
    }
}
