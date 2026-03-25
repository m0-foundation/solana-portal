use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::Mint;
use m0_portal_common::{
    earn::{self, accounts::EarnGlobal, program::Earn},
    portal::{self, accounts::PortalGlobal, program::Portal},
    BridgeError, Payload, AUTHORITY_SEED,
};

use crate::{
    consts::CLEAR_DISCRIMINATOR,
    state::{ClearParams, LayerZeroGlobal, LzReceiveParams, GLOBAL_SEED},
};

/// Number of remaining_accounts consumed by the clear CPI.
pub const CLEAR_ACCOUNTS_COUNT: usize = 8;

#[derive(Accounts)]
pub struct LzReceive<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
        constraint = !lz_global.incoming_paused @ BridgeError::Paused,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump
    )]
    /// CHECK: Account does not hold data; signs Portal CPI
    pub lz_adapter_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = portal::ID,
        bump = portal_global.bump,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

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
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = earn_global.bump,
    )]
    pub earn_global: Account<'info, EarnGlobal>,

    #[account(
        mut,
        address = portal_global.m_mint @ BridgeError::InvalidMint,
    )]
    pub m_mint: InterfaceAccount<'info, Mint>,

    pub m_token_program: Program<'info, Token2022>,

    pub earn_program: Program<'info, Earn>,

    pub portal_program: Program<'info, Portal>,

    pub system_program: Program<'info, System>,
}

impl<'info> LzReceive<'info> {
    fn validate(&self, params: &LzReceiveParams) -> Result<()> {
        let peer = self.lz_global.peers.get_peer(params.src_eid)?;

        if peer.address != params.sender {
            return err!(BridgeError::InvalidPeer);
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(&params))]
    pub fn handler(
        ctx: Context<'_, '_, '_, 'info, Self>,
        params: LzReceiveParams,
    ) -> Result<()> {
        let remaining = ctx.remaining_accounts;

        // First CLEAR_ACCOUNTS_COUNT remaining accounts are for the clear CPI
        let clear_accounts = &remaining[..CLEAR_ACCOUNTS_COUNT];
        let portal_remaining = &remaining[CLEAR_ACCOUNTS_COUNT..];

        // Call clear() on the LZ endpoint before processing
        let clear_params = ClearParams {
            receiver: ctx.accounts.lz_global.key(),
            src_eid: params.src_eid,
            sender: params.sender,
            nonce: params.nonce,
            guid: params.guid,
            message: params.message.clone(),
        };

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&CLEAR_DISCRIMINATOR);
        instruction_data.extend_from_slice(&clear_params.try_to_vec()?);

        let endpoint_key = ctx.accounts.lz_global.endpoint_program;

        // Clear accounts layout:
        // 0: endpoint_program
        // 1: receiver/oapp_pda (lz_global)
        // 2: oapp_registry
        // 3: nonce (writable)
        // 4: payload_hash (writable)
        // 5: endpoint_settings (writable)
        // 6: event_authority
        // 7: endpoint_program (again)
        let clear_ix = Instruction {
            program_id: endpoint_key,
            data: instruction_data,
            accounts: vec![
                AccountMeta::new_readonly(ctx.accounts.lz_global.key(), true), // receiver signs
                AccountMeta::new_readonly(clear_accounts[0].key(), false), // oapp_registry
                AccountMeta::new(clear_accounts[1].key(), false),          // nonce
                AccountMeta::new(clear_accounts[2].key(), false),          // payload_hash
                AccountMeta::new(clear_accounts[3].key(), false),          // endpoint_settings
                AccountMeta::new_readonly(clear_accounts[4].key(), false), // event_authority
                AccountMeta::new_readonly(endpoint_key, false),            // endpoint_program
            ],
        };

        let mut clear_account_infos = Vec::with_capacity(CLEAR_ACCOUNTS_COUNT + 1);
        clear_account_infos.push(ctx.accounts.lz_global.to_account_info());
        for account in clear_accounts.iter() {
            clear_account_infos.push(account.to_account_info());
        }

        let global_bump = ctx.accounts.lz_global.bump;
        invoke_signed(
            &clear_ix,
            &clear_account_infos,
            &[&[GLOBAL_SEED, &[global_bump]]],
        )?;

        // Decode the M0 payload and CPI to Portal receive_message
        let payload = Payload::decode(&params.message)?;
        let m0_source_chain_id = ctx
            .accounts
            .lz_global
            .peers
            .get_peer(params.src_eid)?
            .m0_chain_id;

        portal::cpi::receive_message(
            CpiContext::new_with_signer(
                ctx.accounts.portal_program.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    payer: ctx.accounts.payer.to_account_info(),
                    adapter_authority: ctx.accounts.lz_adapter_authority.to_account_info(),
                    message_account: ctx.accounts.message_account.to_account_info(),
                    portal_authority: ctx.accounts.portal_authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    earn_global: ctx.accounts.earn_global.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    m_token_program: ctx.accounts.m_token_program.to_account_info(),
                    earn_program: ctx.accounts.earn_program.to_account_info(),
                    portal_global: ctx.accounts.portal_global.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.lz_adapter_authority]]],
            )
            .with_remaining_accounts(portal_remaining.to_vec()),
            payload.header.message_id,
            m0_source_chain_id,
            params.message,
        )
    }
}
