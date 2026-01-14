use anchor_lang::prelude::*;

use crate::state::{PermissionedExtension, PortalGlobal, GLOBAL_SEED, PERMISSIONED_PATH_SEED};

#[derive(Accounts)]
#[instruction(extension_mint: Pubkey)]
pub struct SetPermissionedPath<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + PermissionedExtension::INIT_SPACE,
        seeds = [PERMISSIONED_PATH_SEED, extension_mint.as_ref()],
        bump,
    )]
    pub permissioned_path: Account<'info, PermissionedExtension>,

    pub system_program: Program<'info, System>,
}

impl SetPermissionedPath<'_> {
    pub fn handler(
        ctx: Context<Self>,
        extension_mint: Pubkey,
        destination_token: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.permissioned_path.extension_mint = extension_mint;
        ctx.accounts.permissioned_path.destination_token = destination_token;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(extension_mint: Pubkey)]
pub struct RemovePermissionedPath<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        mut,
        close = admin,
        seeds = [PERMISSIONED_PATH_SEED, extension_mint.as_ref()],
        bump,
    )]
    pub permissioned_path: Account<'info, PermissionedExtension>,
}

impl RemovePermissionedPath<'_> {
    pub fn handler(_ctx: Context<Self>, _extension_mint: Pubkey) -> Result<()> {
        Ok(())
    }
}
