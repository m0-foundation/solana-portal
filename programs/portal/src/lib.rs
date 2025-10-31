use anchor_lang::prelude::*;

declare_id!("7eVvG1ofbppUebcB744q5VVjUBFCphGgzBuHhdLHkNzT");

#[program]
pub mod portal {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
