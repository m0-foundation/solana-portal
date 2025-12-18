use anchor_client::{Client, Cluster};
use anchor_lang::{prelude::Pubkey, system_program};
use anchor_spl::token_2022;
use anyhow::Result;
use common::{
    ext_swap::{self},
    pda,
    portal::constants::{GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    wormhole_adapter::{self},
    WormholeRemainingAccounts, AUTHORITY_SEED,
};
use std::sync::Arc;

use portal::{accounts as portal_accounts, instruction as portal_instruction};
use std::str::FromStr;

use crate::{get_rpc_client, get_signer};
use solana_sdk::signature::Keypair;

#[test]
fn test_01_send_token_wormhole_unauthorized_unwrapper() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let extension_program =
        Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

    let m_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[AUTHORITY_SEED], &portal::ID),
        &m_mint,
    )?;
    let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &program.payer(),
        &extension_mint,
    )?;
    let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[M_VAULT_SEED], &extension_program),
        &m_mint,
    )?;

    // Send token update
    let err = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &extension_program),
            m_mint,
            extension_mint,
            m_token_account,
            extension_token_account,
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
            swap_program: ext_swap::ID,
            extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: 1_000_000,
            destination_token: m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: program.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnauthorizedUnwrapper"));

    Ok(())
}
