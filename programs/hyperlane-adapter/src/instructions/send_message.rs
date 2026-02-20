use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::{get_return_data, invoke_signed},
};
use m0_portal_common::{
    portal::{self, accounts::PortalGlobal},
    BridgeError, Payload, PayloadData, PayloadHeader, AUTHORITY_SEED,
};
use std::vec;

use crate::{
    instructions::{Mailbox, SplNoop, H256},
    state::{
        HyperlaneGlobal, HyperlaneUserGlobal, DASH_SEED, DISPATCHED_MESSAGE_SEED, DISPATCH_SEED_1,
        DISPATCH_SEED_2, GAS_PAYMENT_SEED, GLOBAL_SEED, HYPERLANE_IGP_SEED, HYPERLANE_SEED,
        OUTBOX_SEED, PROGRAM_DATA_SEED, UNIQUE_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        constraint = !hyperlane_global.outgoing_paused @ BridgeError::Paused,
        has_one = igp_program_id @ BridgeError::InvalidIgpAccount,
        has_one = igp_account @ BridgeError::InvalidIgpAccount,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    #[account(
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
    /// Only relay messages coming from the Portal
    pub portal_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
        seeds::program = mailbox_program,
        bump
    )]
    /// CHECK: dispatch authority for mailbox
    pub mailbox_outbox: AccountInfo<'info>,

    #[account(
        seeds = [DISPATCH_SEED_1, DASH_SEED, DISPATCH_SEED_2],
        bump
    )]
    /// CHECK: dispatch authority for mailbox
    pub dispatch_authority: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = HyperlaneUserGlobal::size(),
        seeds = [GLOBAL_SEED, DASH_SEED, payer.key().as_ref()],
        bump,
    )]
    pub hyperlane_user_global: Account<'info, HyperlaneUserGlobal>,

    #[account(
        seeds = [
            UNIQUE_MESSAGE_SEED,
            hyperlane_user_global.key().as_ref(),
            &hyperlane_user_global.nonce.to_le_bytes(),
        ],
        bump
    )]
    /// CHECK: only used to create unique message accounts
    /// Using a PDA here instead of a Keypair for better UX
    pub unique_message: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            HYPERLANE_SEED,
            DASH_SEED,
            DISPATCHED_MESSAGE_SEED,
            DASH_SEED,
            unique_message.key().as_ref(),
        ],
        seeds::program = mailbox_program,
        bump
    )]
    /// CHECK: dispatched message account verfied by mailbox program
    pub dispatched_message: AccountInfo<'info>,

    /// CHECK: program matches global setting
    #[account(executable)]
    pub igp_program_id: AccountInfo<'info>,

    /// CHECK: validated by seeds and IGP CPI
    #[account(
        mut,
        seeds = [HYPERLANE_IGP_SEED, DASH_SEED, PROGRAM_DATA_SEED],
        seeds::program = igp_program_id.key(),
        bump
    )]
    pub igp_program_data: AccountInfo<'info>,

    /// CHECK: validated by seeds and IGP CPI
    #[account(
        mut,
        seeds = [
            HYPERLANE_IGP_SEED,
            DASH_SEED,
            GAS_PAYMENT_SEED,
            DASH_SEED,
            unique_message.key().as_ref(),
        ],
        seeds::program = igp_program_id.key(),
        bump
    )]
    pub igp_gas_payment: AccountInfo<'info>,

    /// CHECK: verfied against global setting and IGP program
    #[account(
        mut,
        owner = igp_program_id.key(),
    )]
    pub igp_account: AccountInfo<'info>,

    // CHECK: optional account only needed for overhead IGPs
    #[account(
        owner = igp_program_id.key(),
        constraint = hyperlane_global.igp_overhead_account == Some(igp_overhead_account.key()) @ BridgeError::InvalidIgpAccount,
    )]
    pub igp_overhead_account: Option<AccountInfo<'info>>,

    pub mailbox_program: Program<'info, Mailbox>,

    pub spl_noop_program: Program<'info, SplNoop>,

    pub system_program: Program<'info, System>,
}

impl SendMessage<'_> {
    pub fn handler(
        ctx: Context<Self>,
        m0_destination_chain_id: u32,
        message_id: [u8; 32],
        payload: Vec<u8>,
        payload_type: u8,
    ) -> Result<()> {
        let peer = ctx
            .accounts
            .hyperlane_global
            .peers
            .get_m0_peer(m0_destination_chain_id)?
            .clone();

        // Dispatch the message via the mailbox program
        let message_id = {
            let message = Payload {
                header: PayloadHeader {
                    message_id,
                    destination_chain_id: m0_destination_chain_id,
                    destination_peer: peer.address,
                    payload_type,
                    index: ctx.accounts.portal_global.m_index,
                },
                data: PayloadData::decode(payload_type, &payload)?,
            }
            .encode();

            // OutboxDispatch discriminant
            let mut instruction_data = vec![4u8];

            // Serialize OutboxDispatch struct fields
            instruction_data.extend_from_slice(&crate::ID.to_bytes());
            instruction_data.extend_from_slice(&peer.adapter_chain_id.to_le_bytes());
            instruction_data.extend_from_slice(&peer.address);
            instruction_data.extend_from_slice(&(message.len() as u32).to_le_bytes());
            instruction_data.extend_from_slice(&message);

            let mailbox_ixn = Instruction {
                program_id: ctx.accounts.mailbox_program.key(),
                data: instruction_data,
                accounts: vec![
                    AccountMeta::new(ctx.accounts.mailbox_outbox.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.dispatch_authority.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.spl_noop_program.key(), false),
                    AccountMeta::new(ctx.accounts.payer.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.unique_message.key(), true),
                    AccountMeta::new(ctx.accounts.dispatched_message.key(), false),
                ],
            };

            invoke_signed(
                &mailbox_ixn,
                &[
                    ctx.accounts.mailbox_outbox.clone(),
                    ctx.accounts.dispatch_authority.clone(),
                    ctx.accounts.system_program.to_account_info(),
                    ctx.accounts.spl_noop_program.to_account_info(),
                    ctx.accounts.payer.to_account_info(),
                    ctx.accounts.unique_message.to_account_info(),
                    ctx.accounts.dispatched_message.clone(),
                ],
                &[
                    &[
                        DISPATCH_SEED_1,
                        DASH_SEED,
                        DISPATCH_SEED_2,
                        &[ctx.bumps.dispatch_authority],
                    ],
                    &[
                        UNIQUE_MESSAGE_SEED,
                        ctx.accounts.hyperlane_user_global.key().as_ref(),
                        &ctx.accounts.hyperlane_user_global.nonce.to_le_bytes(),
                        &[ctx.bumps.unique_message],
                    ],
                ],
            )?;

            let (returning_program_id, returned_data) = get_return_data().unwrap();
            require!(
                returning_program_id == ctx.accounts.mailbox_program.key(),
                BridgeError::InvalidReturnData
            );

            H256::try_from_slice(&returned_data)?
        };

        // Pay for gas via the IGP
        {
            // PayForGas discriminator
            let mut instruction_data = vec![3u8];

            // Serialize PayForGas struct fields
            instruction_data.extend_from_slice(message_id.as_bytes());
            instruction_data.extend_from_slice(&peer.adapter_chain_id.to_le_bytes());

            let gas_amount = ctx.accounts.hyperlane_global.igp_gas_amount;
            instruction_data.extend_from_slice(&gas_amount.to_le_bytes());

            let mut accounts = vec![
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new(ctx.accounts.igp_program_data.key(), false),
                AccountMeta::new_readonly(ctx.accounts.unique_message.key(), true),
                AccountMeta::new(ctx.accounts.igp_gas_payment.key(), false),
                AccountMeta::new(ctx.accounts.igp_account.key(), false),
            ];

            let mut account_infos = vec![
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.igp_program_data.clone(),
                ctx.accounts.unique_message.to_account_info(),
                ctx.accounts.igp_gas_payment.clone(),
                ctx.accounts.igp_account.clone(),
            ];

            // Include overhead IGP account if applicable
            if let Some(igp_overhead_account) = &ctx.accounts.igp_overhead_account {
                accounts.push(AccountMeta::new_readonly(igp_overhead_account.key(), false));
                account_infos.push(igp_overhead_account.clone());
            }

            let igp_ixn = Instruction {
                program_id: ctx.accounts.igp_program_id.key(),
                data: instruction_data,
                accounts,
            };

            invoke_signed(
                &igp_ixn,
                &account_infos,
                &[&[
                    UNIQUE_MESSAGE_SEED,
                    ctx.accounts.hyperlane_user_global.key().as_ref(),
                    &ctx.accounts.hyperlane_user_global.nonce.to_le_bytes(),
                    &[ctx.bumps.unique_message],
                ]],
            )?;
        }

        // Might not be initialized
        ctx.accounts
            .hyperlane_user_global
            .set_inner(HyperlaneUserGlobal {
                bump: ctx.bumps.hyperlane_user_global,
                user: ctx.accounts.payer.key(),
                // Bump the nonce used to generate unique message accounts
                nonce: ctx.accounts.hyperlane_user_global.nonce + 1,
            });

        Ok(())
    }
}
