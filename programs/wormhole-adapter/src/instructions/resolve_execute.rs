use anchor_lang::{
    prelude::*, solana_program::entrypoint::MAX_PERMITTED_DATA_INCREASE, system_program,
    InstructionData,
};

use anchor_spl::{
    associated_token::spl_associated_token_account::solana_program::system_instruction::MAX_PERMITTED_DATA_LENGTH,
    token_2022,
};
use executor_account_resolver_svm::{
    find_account, InstructionGroup, InstructionGroups, MissingAccounts, Resolver,
    SerializableAccountMeta, SerializableInstruction, RESOLVER_PUBKEY_PAYER,
    RESOLVER_PUBKEY_SHIM_VAA_SIGS, RESOLVER_RESULT_ACCOUNT, RESOLVER_RESULT_ACCOUNT_SEED,
};

use crate::{
    errors::WormholeError,
    instruction::ReceiveMessage,
    instructions::{
        earn::{self, accounts::EarnGlobal},
        ext_swap::{self, accounts::SwapGlobal},
        portal, wormhole_verify_vaa_shim, VaaBody,
    },
    state::GLOBAL_SEED,
};

#[derive(Accounts)]
pub struct ResolveExecuteVaa {}

#[account(discriminator = RESOLVER_RESULT_ACCOUNT)]
pub struct ExecutorAccountResolverResult(Resolver<InstructionGroups>);

impl ResolveExecuteVaa {
    pub fn handler<'info>(
        ctx: Context<'_, '_, 'info, 'info, ResolveExecuteVaa>,
        vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        let vaa = VaaBody::from_bytes(&vaa_body)?;

        // Accounts we need read data on
        let swap_global = pda(&[GLOBAL_SEED], &ext_swap::ID);
        let earn_global = pda(&[GLOBAL_SEED], &earn::ID);
        let result_account = pda(&[RESOLVER_RESULT_ACCOUNT_SEED], &crate::ID);

        // Check for missing accounts
        {
            let mut missing = Vec::new();

            for account in [swap_global, earn_global, result_account] {
                if find_account(ctx.remaining_accounts, account).is_none() {
                    missing.push(account);
                }
            }

            if !missing.is_empty() {
                // Placeholder for payer we know is missing
                missing.push(RESOLVER_PUBKEY_PAYER);
                missing.push(RESOLVER_PUBKEY_SHIM_VAA_SIGS);

                return Ok(Resolver::Missing(MissingAccounts {
                    accounts: missing,
                    address_lookup_tables: Vec::new(),
                }));
            }
        }

        // Increase the size of the return account then parse it
        let mut ret = {
            let return_account = find_account(ctx.remaining_accounts, result_account).unwrap();
            let system_account = find_account(ctx.remaining_accounts, System::id()).unwrap();

            // Find the payer account
            let payer_account = ctx
                .remaining_accounts
                .iter()
                .find(|acc_info| acc_info.is_signer && acc_info.is_writable)
                .ok_or(WormholeError::MissingPayerAccount)?;

            if !return_account.is_writable {
                return err!(WormholeError::InvalidReturnAccount);
            }

            let size = usize::min(
                return_account.data_len() + MAX_PERMITTED_DATA_INCREASE,
                MAX_PERMITTED_DATA_LENGTH.try_into()?,
            );

            let lamports = Rent::get()
                .unwrap()
                .minimum_balance(size)
                .saturating_sub(return_account.lamports());

            system_program::transfer(
                CpiContext::new(
                    system_account.to_account_info(),
                    system_program::Transfer {
                        from: payer_account.to_account_info(),
                        to: return_account.to_account_info(),
                    },
                ),
                lamports,
            )?;

            return_account.resize(size)?;

            Account::<ExecutorAccountResolverResult>::try_from(return_account)?
        };

        // Parse accounts we know are on remaining_accounts
        let swap_global_data =
            deserialize_account::<SwapGlobal>(ctx.remaining_accounts, swap_global)?;
        let earn_global_data =
            deserialize_account::<EarnGlobal>(ctx.remaining_accounts, earn_global)?;

        let receive_message_ix = SerializableInstruction {
            program_id: crate::ID,
            data: ReceiveMessage {
                guardian_set_index: 0, // TODO
                vaa_body,
            }
            .data(),
            accounts: vec![
                SerializableAccountMeta {
                    pubkey: pda(&[GLOBAL_SEED], &crate::ID),
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: pda(&[b"authority"], &portal::ID),
                    is_writable: false,
                    is_signer: false,
                },
                // SerializableAccountMeta {
                //     pubkey: guardian_set,
                //     is_writable: false,
                //     is_signer: false,
                // },
                // SerializableAccountMeta {
                //     pubkey: guardian_signatures,
                //     is_writable: false,
                //     is_signer: false,
                // },
                SerializableAccountMeta {
                    pubkey: Pubkey::find_program_address(&[GLOBAL_SEED], &earn::ID).0,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: earn_global_data.m_mint,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: wormhole_verify_vaa_shim::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: earn::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: token_2022::ID,
                    is_writable: false,
                    is_signer: false,
                },
            ],
        };

        ret.set_inner(ExecutorAccountResolverResult(Resolver::Resolved(
            InstructionGroups(vec![InstructionGroup {
                instructions: vec![receive_message_ix],
                address_lookup_tables: vec![],
            }]),
        )));
        ret.exit(ctx.program_id)?;
        Ok(Resolver::Account())
    }
}

pub fn pda(seeds: &[&[u8]], program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(seeds, program_id).0
}

fn deserialize_account<T: AccountDeserialize>(
    remaining_accounts: &[AccountInfo],
    pubkey: Pubkey,
) -> Result<T> {
    let account = find_account(remaining_accounts, pubkey).unwrap();

    match T::try_deserialize(&mut &account.try_borrow_mut_data()?[..]) {
        Ok(data) => Ok(data),
        Err(e) => {
            msg!("Failed to deserialize account {}", pubkey);
            Err(e)
        }
    }
}
