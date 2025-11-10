#![allow(unexpected_cfgs)]

mod instructions;
mod state;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("mZhPGteS36G7FhMTcRofLQU8ocBNAsGq7u8SKSHfL2X");

#[program]
pub mod hyperlane_adapter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Initialize::handler(ctx)
    }

    pub fn send_message(ctx: Context<SendMessage>, message: Vec<u8>) -> Result<()> {
        SendMessage::handler(ctx, message)
    }

    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx)
    }
}
