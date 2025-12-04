use anchor_lang::prelude::{
    instruction::Instruction,
    program::{get_return_data, invoke_signed},
    *,
};
use common::{portal, BridgeError, AUTHORITY_SEED};
use std::vec;

use crate::{
    instructions::{Mailbox, SplNoop, H256},
    state::{
        HyperlaneGlobal, DASH_SEED, DISPATCHED_MESSAGE_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2,
        GAS_PAYMENT_SEED, GLOBAL_SEED, HYPERLANE_IGP_SEED, HYPERLANE_SEED, OUTBOX_SEED,
        PROGRAM_DATA_SEED, UNIQUE_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        mut,
        constraint = !hyperlane_global.paused @ BridgeError::Paused,
        has_one = igp_program_id @ BridgeError::InvalidIgpAccount,
        has_one = igp_account @ BridgeError::InvalidIgpAccount,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

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
        seeds = [UNIQUE_MESSAGE_SEED, &hyperlane_global.nonce.to_le_bytes()],
        bump
    )]
    /// CHECK: only used to create unique message accounts
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
        constraint = igp_overhead_account.key() == hyperlane_global.igp_overhead_account.unwrap() @ BridgeError::InvalidIgpAccount,
    )]
    pub igp_overhead_account: Option<AccountInfo<'info>>,

    pub mailbox_program: Program<'info, Mailbox>,

    pub spl_noop_program: Program<'info, SplNoop>,

    pub system_program: Program<'info, System>,
}

impl SendMessage<'_> {
    pub fn handler(ctx: Context<Self>, message: Vec<u8>, destination_chain_id: u32) -> Result<()> {
        // Dispatch the message via the mailbox program
        let message_id = {
            let peer = ctx
                .accounts
                .hyperlane_global
                .get_peer(destination_chain_id)?;

            // OutboxDispatch discriminant
            let mut instruction_data = vec![4u8];

            // Serialize OutboxDispatch struct fields
            instruction_data.extend_from_slice(&crate::ID.to_bytes());
            instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());
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
                        &ctx.accounts.hyperlane_global.nonce.to_le_bytes(),
                        &[ctx.bumps.unique_message],
                    ],
                ],
            )?;

            let (returning_program_id, returned_data) = get_return_data().unwrap();
            assert_eq!(returning_program_id, ctx.accounts.mailbox_program.key());

            H256::try_from_slice(&returned_data)?
        };

        // Pay for gas via the IGP
        {
            // PayForGas discriminator
            let mut instruction_data = vec![3u8];

            // Serialize PayForGas struct fields
            instruction_data.extend_from_slice(message_id.as_bytes());
            instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());

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
                    &ctx.accounts.hyperlane_global.nonce.to_le_bytes(),
                    &[ctx.bumps.unique_message],
                ]],
            )?;
        }

        // Bump the nonce used to generate unique message accounts
        ctx.accounts.hyperlane_global.nonce += 1;

        Ok(())
    }
}
