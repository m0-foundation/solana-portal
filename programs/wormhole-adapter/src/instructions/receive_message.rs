use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::spl_associated_token_account::solana_program::keccak,
    token_interface::{Mint, TokenInterface},
};

use crate::{
    consts::CORE_BRIDGE_PROGRAM_ID,
    instructions::{
        earn::{self, accounts::EarnGlobal, program::Earn},
        portal,
        wormhole_verify_vaa_shim::{
            self, cpi::accounts::VerifyHash, program::WormholeVerifyVaaShim,
        },
        VaaBody,
    },
    state::{WormholeGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
    #[account(
        constraint = !wormhole_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    #[account(
        seeds = [b"authority"], 
        seeds::program = portal::ID,
        bump
    )]
    /// CHECK: Account does not hold data
    pub messenger_authority: AccountInfo<'info>,

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
    fn validate(&self, guardian_set_index: u32, vaa_body: &Vec<u8>) -> Result<()> {
        let (guardian_set_key, guardian_set_bump) = Pubkey::find_program_address(
            &[b"GuardianSet", &guardian_set_index.to_be_bytes()],
            &CORE_BRIDGE_PROGRAM_ID,
        );

        if guardian_set_key != self.guardian_set.key() {
            return Err(ProgramError::InvalidArgument.into());
        }

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

    #[access_control(ctx.accounts.validate(guardian_set_index, &vaa_body))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        guardian_set_index: u32,
        vaa_body: Vec<u8>,
    ) -> Result<()> {
        let vaa = VaaBody::from_bytes(&vaa_body)?;

        portal::cpi::receive_message(
            CpiContext::new(
                ctx.accounts.wormhole_verify_vaa_shim.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    messenger_authority: ctx.accounts.messenger_authority.to_account_info(),
                    m_global: ctx.accounts.m_global.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    earn_program: ctx.accounts.earn_program.to_account_info(),
                    m_token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),

                    // Optional accounts for token transfer
                    extension_mint: ctx.remaining_accounts.get(0).cloned(),
                    recipient_token_account: ctx.remaining_accounts.get(1).cloned(),
                    authority_m_token_account: ctx.remaining_accounts.get(2).cloned(),
                    extension_m_vault: ctx.remaining_accounts.get(3).cloned(),
                    extension_m_vault_authority: ctx.remaining_accounts.get(4).cloned(),
                    extension_mint_authority: ctx.remaining_accounts.get(5).cloned(),
                    extension_global: ctx.remaining_accounts.get(6).cloned(),
                    extension_token_program: ctx.remaining_accounts.get(7).cloned(),
                    extension_program: ctx.remaining_accounts.get(8).cloned(),
                    swap_global: ctx.remaining_accounts.get(9).cloned(),
                    swap_program: ctx.remaining_accounts.get(10).cloned(),
                },
            ),
            vaa.payload.encode(),
        )?;

        Ok(())
    }
}
