use anchor_lang::prelude::{instruction::Instruction, program::invoke_signed, *};
use common::{portal, BridgeError, AUTHORITY_SEED};

use crate::{
    instructions::{Mailbox, SplNoop},
    state::{
        HyperlaneGlobal, DASH_SEED, DISPATCHED_MESSGAGE_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2,
        GLOBAL_SEED, HYPERLANE_SEED, OUTBOX_SEED, UNIQUE_MESSGAGE_SEED,
    },
};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        mut,
        constraint = !hyperlane_global.paused @ BridgeError::Paused,
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
    pub messenger_authority: Signer<'info>,

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
        seeds = [UNIQUE_MESSGAGE_SEED, &hyperlane_global.nonce.to_le_bytes()],
        bump
    )]
    /// CHECK: only used to create unique message accounts
    pub unique_message: AccountInfo<'info>,

    #[account(
        seeds = [
            HYPERLANE_SEED,
            DASH_SEED,
            DISPATCHED_MESSGAGE_SEED,
            DASH_SEED,
            unique_message.key().as_ref(),
        ],
        seeds::program = mailbox_program,
        bump
    )]
    /// CHECK: dispatched message account verfied by mailbox program
    pub dispatched_message: AccountInfo<'info>,

    pub mailbox_program: Program<'info, Mailbox>,

    pub spl_noop_program: Program<'info, SplNoop>,

    pub system_program: Program<'info, System>,
}

impl SendMessage<'_> {
    pub fn handler(ctx: Context<Self>, message: Vec<u8>, destination_chain_id: u32) -> Result<()> {
        let peer = ctx
            .accounts
            .hyperlane_global
            .get_peer(destination_chain_id)?;

        // OutboxDispatch discriminant
        let mut instruction_data = vec![4u8];

        // Serialize OutboxDispatch struct fields
        instruction_data.extend_from_slice(&ctx.accounts.payer.key().to_bytes());
        instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes()); // TODO: convert our internal chain ID to Hyperlane chain ID
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
            &[&[
                DISPATCH_SEED_1,
                DASH_SEED,
                DISPATCH_SEED_2,
                &[ctx.bumps.dispatch_authority],
            ]],
        )?;

        // Bump the nonce used to generate unique message accounts
        ctx.accounts.hyperlane_global.nonce += 1;

        Ok(())
    }
}
