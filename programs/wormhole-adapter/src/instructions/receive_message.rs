use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::spl_associated_token_account::solana_program::keccak,
    token_interface::{Mint, TokenInterface},
};
use common::{
    earn::{self, accounts::EarnGlobal, program::Earn},
    portal,
    wormhole_verify_vaa_shim::{self, cpi::accounts::VerifyHash, program::WormholeVerifyVaaShim},
};

use crate::{
    consts::{AUTHORITY_SEED, CORE_BRIDGE_PROGRAM_ID, GUARDIAN_SET_SEED},
    instructions::VaaBody,
    state::{WormholeGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
#[instruction(_guardian_set_index: u32)]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        constraint = !wormhole_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

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
    pub messenger_authority: AccountInfo<'info>,

    #[account(
        seeds = [GUARDIAN_SET_SEED, &_guardian_set_index.to_be_bytes()],
        seeds::program = CORE_BRIDGE_PROGRAM_ID,
        bump
    )]
    /// CHECK: Guardian set used for signature verification by shim (checked by the shim)
    pub guardian_set: UncheckedAccount<'info>,

    /// CHECK: Stored guardian signatures to be verified by shim (ownership ownership and discriminator is checked by the shim)
    pub guardian_signatures: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = m_global.bump,
        has_one = m_mint,
    )]
    pub m_global: Account<'info, EarnGlobal>,

    #[account(mut)]
    pub m_mint: InterfaceAccount<'info, Mint>,

    pub wormhole_verify_vaa_shim: Program<'info, WormholeVerifyVaaShim>,

    pub earn_program: Program<'info, Earn>,

    pub token_program: Interface<'info, TokenInterface>,

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
        _guardian_set_index: u32,
        vaa_body: Vec<u8>,
    ) -> Result<()> {
        portal::cpi::receive_message(
            CpiContext::new(
                ctx.accounts.wormhole_verify_vaa_shim.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    relayer: ctx.accounts.relayer.to_account_info(),
                    adapter_authority: ctx.accounts.wormhole_adapter_authority.to_account_info(),
                    messenger_authority: ctx.accounts.messenger_authority.to_account_info(),
                    m_global: ctx.accounts.m_global.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    earn_program: ctx.accounts.earn_program.to_account_info(),
                    m_token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
            VaaBody::from_bytes(&vaa_body)?.payload.encode(),
        )
    }
}
