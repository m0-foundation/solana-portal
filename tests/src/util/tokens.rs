#![allow(warnings)]

use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token_2022};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{
    extension::ExtensionType, instruction as token2022_ix, state::Mint as Token2022MintState,
};

use spl_associated_token_account::instruction::create_associated_token_account;

pub fn create_token2022_mint(rpc: &RpcClient, payer: &Keypair, decimals: u8) -> Result<Pubkey> {
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();

    let extensions: &[ExtensionType] = &[]; // add your mint extensions here
    let mint_len = ExtensionType::try_calculate_account_len::<Token2022MintState>(extensions)?;
    let lamports = rpc.get_minimum_balance_for_rent_exemption(mint_len)?;

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        lamports,
        mint_len as u64,
        &token_2022::ID,
    );

    let init_mint_ix =
        token2022_ix::initialize_mint2(&token_2022::ID, &mint, &payer.pubkey(), None, decimals)?;

    rpc.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[payer, &mint_kp],
        rpc.get_latest_blockhash().unwrap(),
    ))?;

    Ok(mint)
}

pub fn get_or_create_ata_2022(
    rpc: &RpcClient,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey> {
    let ata = get_associated_token_address_with_program_id(owner, mint, &token_2022::ID);

    // Create if missing
    if rpc.get_account(&ata).is_err() {
        let ix = create_associated_token_account(&payer.pubkey(), owner, mint, &token_2022::ID);

        rpc.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[payer],
            rpc.get_latest_blockhash().unwrap(),
        ))?;
    }

    Ok(ata)
}

pub fn mint_2022_to_owner(
    rpc: &RpcClient,
    payer: &Keypair,
    mint: &Pubkey,
    dest_owner: &Pubkey,
    amount: u64,
) -> Result<Pubkey> {
    let ata = get_or_create_ata_2022(rpc, payer, dest_owner, mint)?;

    let ix = token2022_ix::mint_to(&token_2022::ID, mint, &ata, &payer.pubkey(), &[], amount)?;
    rpc.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        rpc.get_latest_blockhash().unwrap(),
    ))?;

    Ok(ata)
}
