use anchor_lang::{
    prelude::*,
    solana_program::{entrypoint::MAX_PERMITTED_DATA_INCREASE, instruction::Instruction},
    system_program, InstructionData,
};
use anchor_spl::{
    associated_token::spl_associated_token_account::solana_program::system_instruction::MAX_PERMITTED_DATA_LENGTH,
    token_2022,
};
use m0_portal_common::{
    earn::{self, accounts::EarnGlobal},
    ext_swap::{self, accounts::SwapGlobal},
    pda,
    portal::{self, constants::MESSAGE_SEED},
    require_metas, wormhole_verify_vaa_shim, BridgeError, Extension,
};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use wormhole_svm_definitions::{zero_copy::GuardianSet, GUARDIAN_SET_SEED};

use crate::{
    consts::{AUTHORITY_SEED, CORE_BRIDGE_PROGRAM_ID},
    executor::{
        find_account, missing_account, InstructionGroup, InstructionGroups, MissingAccounts,
        Resolver, SerializableInstruction, RESOLVER_PUBKEY_GUARDIAN_SET, RESOLVER_PUBKEY_PAYER,
        RESOLVER_PUBKEY_SHIM_VAA_SIGS, RESOLVER_RESULT_ACCOUNT, RESOLVER_RESULT_ACCOUNT_SEED,
    },
    instruction::ReceiveMessage,
    instructions::VaaBody,
    state::{WormholeGlobal, GLOBAL_SEED},
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
        let message_id = vaa.payload.header.message_id;

        let result_account = pda!(&[RESOLVER_RESULT_ACCOUNT_SEED], &crate::ID);
        let global = pda!(&[GLOBAL_SEED], &crate::ID);
        let earn_global = pda!(&[GLOBAL_SEED], &earn::ID);
        let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);

        let mut fill_report_token_in: Option<Pubkey> = None;

        // Check for missing accounts
        {
            let mut accounts_required = vec![
                System::id(),
                result_account,
                global,
                earn_global,
                swap_global,
            ];

            match vaa.payload.data {
                m0_portal_common::PayloadData::FillReport(ref report) => {
                    // Need mint account to see which token program owns it
                    accounts_required.push(report.token_in.into());
                    fill_report_token_in = Some(report.token_in.into());
                }
                _ => {}
            }

            let mut missing: Vec<_> = accounts_required
                .into_iter()
                .filter(|&account| find_account(ctx.remaining_accounts, account).is_none())
                .collect();

            if !missing.is_empty() {
                // Placeholder for payer and vaa sigs we know are missing
                missing.extend([RESOLVER_PUBKEY_PAYER, RESOLVER_PUBKEY_SHIM_VAA_SIGS]);

                msg!("Missing {} accounts", missing.len());

                return Ok(Resolver::Missing(MissingAccounts {
                    accounts: missing,
                    address_lookup_tables: Vec::new(),
                }));
            }
        }

        let mut guardian_set_index: Option<u32> = None;
        let mut guardian_set_pubkey: Option<Pubkey> = None;

        // Attempt to find guardian set account.
        for account_info in ctx.remaining_accounts.iter() {
            // Try to deserialize as GuardianSet
            if let Ok(data) = account_info.try_borrow_data() {
                if let Some(guardian_set_account) = GuardianSet::new(&data) {
                    let index = guardian_set_account.guardian_set_index();
                    let (derived_guardian_set_pubkey, _) = Pubkey::find_program_address(
                        &[GUARDIAN_SET_SEED, &index.to_be_bytes()],
                        &CORE_BRIDGE_PROGRAM_ID,
                    );

                    // Verify the account matches the expected PDA
                    if *account_info.key == derived_guardian_set_pubkey {
                        msg!(
                            "Found guardian set account with index {}: {}",
                            index,
                            derived_guardian_set_pubkey
                        );

                        guardian_set_index = Some(index);
                        guardian_set_pubkey = Some(derived_guardian_set_pubkey);
                        break;
                    }
                }
            }
        }

        let (guardian_set, guardian_index) = match (guardian_set_pubkey, guardian_set_index) {
            (Some(pubkey), Some(index)) => (pubkey, index),
            _ => {
                msg!("Missing guardian set account");
                return Ok(missing_account(RESOLVER_PUBKEY_GUARDIAN_SET));
            }
        };

        // Increase the size of the return account then parse it
        let mut ret = {
            let return_account = find_account(ctx.remaining_accounts, result_account).unwrap();
            let system_account = find_account(ctx.remaining_accounts, System::id()).unwrap();

            // Find the payer account
            let payer_account = ctx
                .remaining_accounts
                .iter()
                .find(|acc_info| acc_info.is_signer && acc_info.is_writable)
                .ok_or(BridgeError::MissingPayerAccount)?;

            if !return_account.is_writable {
                return err!(BridgeError::InvalidReturnAccount);
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

        // Parse Earn global to get m_mint
        let earn_global_data: EarnGlobal = deserialize_account(ctx.remaining_accounts, earn_global)
            .expect("earn global account should be present");

        // Receive instruction on Wormhole adapter
        let mut receive_message_ix = SerializableInstruction {
            program_id: crate::ID,
            data: ReceiveMessage {
                guardian_set_index: guardian_index,
                vaa_body,
            }
            .data(),
            accounts: vec![
                AccountMeta::new(RESOLVER_PUBKEY_PAYER, true).into(),
                AccountMeta::new_readonly(pda!(&[GLOBAL_SEED], &crate::ID), false).into(),
                AccountMeta::new(pda!(&[GLOBAL_SEED], &portal::ID), false).into(),
                AccountMeta::new_readonly(pda!(&[AUTHORITY_SEED], &crate::ID), false).into(),
                AccountMeta::new_readonly(pda!(&[AUTHORITY_SEED], &portal::ID), false).into(),
                AccountMeta::new(pda!(&[MESSAGE_SEED, &message_id], &portal::ID), false).into(),
                AccountMeta::new_readonly(guardian_set, false).into(),
                AccountMeta::new_readonly(RESOLVER_PUBKEY_SHIM_VAA_SIGS, false).into(),
                AccountMeta::new_readonly(wormhole_verify_vaa_shim::ID, false).into(),
                AccountMeta::new(pda!(&[GLOBAL_SEED], &earn::ID), false).into(),
                AccountMeta::new(earn_global_data.m_mint, false).into(),
                AccountMeta::new_readonly(token_2022::ID, false).into(),
                AccountMeta::new_readonly(earn::ID, false).into(),
                AccountMeta::new_readonly(portal::ID, false).into(),
                AccountMeta::new_readonly(system_program::ID, false).into(),
            ],
        };

        // Parse SwapGlobal for whitelisted extensions
        let swap_global_data: SwapGlobal = deserialize_account(ctx.remaining_accounts, swap_global)
            .expect("swap global account should be present");

        let whitelisted_extensions = swap_global_data
            .whitelisted_extensions
            .iter()
            .map(|&ext| Extension::from(ext))
            .collect();

        let required_remaining = require_metas(
            &vaa.payload.data,
            earn_global_data.m_mint,
            whitelisted_extensions,
            fill_report_token_in.and_then(|mint| find_account(ctx.remaining_accounts, mint)),
        )?;

        // Add expected remaining accounts based on payload type
        receive_message_ix
            .accounts
            .extend(required_remaining.iter().cloned().map(|a| a.into()));

        // Get LUT from Wormhole global if set
        let wormhole_global: WormholeGlobal = deserialize_account(ctx.remaining_accounts, global)?;
        let mut luts = vec![];
        luts.extend(wormhole_global.receive_lut);

        // Increase compute budget
        let compute_budget_ix: Instruction =
            ComputeBudgetInstruction::set_compute_unit_limit(300_000).into();
        let compute_budget_ix = SerializableInstruction {
            program_id: compute_budget_ix.program_id,
            accounts: compute_budget_ix
                .accounts
                .into_iter()
                .map(|a| a.into())
                .collect(),
            data: compute_budget_ix.data,
        };

        ret.set_inner(ExecutorAccountResolverResult(Resolver::Resolved(
            InstructionGroups(vec![InstructionGroup {
                instructions: vec![compute_budget_ix, receive_message_ix],
                address_lookup_tables: luts,
            }]),
        )));
        ret.exit(ctx.program_id)?;

        Ok(Resolver::Account())
    }
}

fn deserialize_account<T: AccountDeserialize>(
    remaining_accounts: &[AccountInfo],
    pubkey: Pubkey,
) -> Result<T> {
    let account =
        find_account(remaining_accounts, pubkey).ok_or(BridgeError::MissingOptionalAccount)?;

    match T::try_deserialize(&mut &account.try_borrow_mut_data()?[..]) {
        Ok(data) => Ok(data),
        Err(e) => {
            msg!("Failed to deserialize account {}", pubkey);
            Err(e)
        }
    }
}
