use std::{str::FromStr, sync::Arc};

use anchor_client::{Client, Cluster, Program};
use anchor_lang::{prelude::Pubkey, system_program, AccountDeserialize};
use anchor_spl::token_2022;
use anyhow::{Ok, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{compute_budget::ComputeBudgetInstruction, signature::Keypair};
use solana_transaction_status_client_types::UiTransactionEncoding;
use spl_token_2022::{
    extension::StateWithExtensions,
    state::{Account as TokenAccount2022, AccountState},
};

use m0_portal_common::{
    ext_swap::{self},
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
    },
    pda,
    portal::constants::{CHAIN_PATHS_SEED, GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    wormhole_adapter::{self},
    HyperlaneRemainingAccounts, PayloadData, WormholeRemainingAccounts, AUTHORITY_SEED,
};

use portal::{accounts as portal_accounts, instruction as portal_instruction, state::PortalGlobal};

use crate::{
    get_rpc_client, get_signer, set_token_account,
    util::{self, wormhole::build_versioned_tx_with_lut},
};

const AMOUNT: u64 = 1_000_000;

struct TestCtx {
    rpc: Arc<RpcClient>,
    portal: Program<Arc<Keypair>>,

    m_mint: Pubkey,
    extension_mint: Pubkey,
    extension_program: Pubkey,
    destination_token: [u8; 32],

    portal_authority: Pubkey,

    m_token_account: Pubkey,
    extension_token_account: Pubkey,
    ext_m_vault: Pubkey,
}

impl TestCtx {
    fn new() -> Result<Self> {
        let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
        let rpc: Arc<RpcClient> = get_rpc_client();
        let portal = client.program(portal::ID)?;

        let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH")?;
        let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp")?;
        let extension_program = Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko")?;
        let destination_token =
            hex::decode("000000000000000000000000437cc33344a0b27a429f795ff6b469c72698b291")?;
        let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

        let m_token_account = crate::util::tokens::get_or_create_ata_2022(
            &rpc,
            &get_signer(),
            &portal_authority,
            &m_mint,
        )?;

        let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
            &rpc,
            &get_signer(),
            &portal.payer(),
            &extension_mint,
        )?;

        let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
            &rpc,
            &get_signer(),
            &pda!(&[M_VAULT_SEED], &extension_program),
            &m_mint,
        )?;

        Ok(Self {
            rpc,
            portal,
            m_mint,
            extension_mint,
            extension_program,
            portal_authority,
            m_token_account,
            extension_token_account,
            ext_m_vault,
            destination_token: destination_token.try_into().unwrap(),
        })
    }

    fn chain_paths_pda(&self, destination_chain_id: u32) -> Pubkey {
        pda!(
            &[CHAIN_PATHS_SEED, &destination_chain_id.to_le_bytes()],
            &portal::ID
        )
    }

    fn hyperlane_remaining_accounts(&self, nonce: u64) -> Result<HyperlaneRemainingAccounts> {
        let data = self
            .rpc
            .get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
        let global = HyperlaneGlobal::try_deserialize(&mut data.as_slice())?;
        Ok(HyperlaneRemainingAccounts::new(
            &self.portal.payer(),
            &global,
            Some(&HyperlaneUserGlobal {
                nonce,
                bump: 255,
                user: Pubkey::default(),
            }),
            false,
        ))
    }
}

#[test]
fn test_01_send_token_wormhole_insufficient_funds() -> Result<()> {
    let ctx = TestCtx::new()?;
    let destination_chain_id: u32 = 1;

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            extension_global: pda!(&[GLOBAL_SEED], &ctx.extension_program),
            m_mint: ctx.m_mint,
            extension_mint: ctx.extension_mint,
            m_token_account: ctx.m_token_account,
            extension_token_account: ctx.extension_token_account,
            portal_authority: ctx.portal_authority,
            ext_m_vault: ctx.ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ctx.extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ctx.extension_program),
            swap_program: ext_swap::ID,
            extension_program: ctx.extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: AMOUNT,
            destination_token: ctx.destination_token,
            destination_chain_id,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("custom program error: 0x1"), "got: {}", s);
    assert!(s.contains("insufficient funds"), "got: {}", s);

    Ok(())
}

#[test]
fn test_02_send_token_wormhole_success() -> Result<()> {
    let ctx = TestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // fund's payer's wrapped_m ata
    set_token_account(
        &ctx.portal.payer(),
        &ctx.extension_mint,
        serde_json::json!({ "amount": 1_000_000_000 }),
    )?;

    // assert payer's extension_token_account has 1,000,000,000 wrapped_m
    let balance = ctx
        .rpc
        .get_token_account_balance(&ctx.extension_token_account)?;
    assert_eq!(balance.amount, "1000000000");

    // unfreeze portal_authority's m_token_account
    set_token_account(
        &ctx.portal_authority,
        &ctx.m_mint,
        serde_json::json!({ "state": "initialized" }),
    )?;

    // assert portal_authority's m_token_account is unfrozen
    let acc = ctx.rpc.get_account(&ctx.m_token_account)?;
    let ta = StateWithExtensions::<TokenAccount2022>::unpack(&acc.data)?;
    assert!(ta.base.state == AccountState::Initialized);

    let sig = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            extension_global: pda!(&[GLOBAL_SEED], &ctx.extension_program),
            m_mint: ctx.m_mint,
            extension_mint: ctx.extension_mint,
            m_token_account: ctx.m_token_account,
            extension_token_account: ctx.extension_token_account,
            portal_authority: ctx.portal_authority,
            ext_m_vault: ctx.ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ctx.extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ctx.extension_program),
            swap_program: ext_swap::ID,
            extension_program: ctx.extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: AMOUNT,
            destination_token: ctx.destination_token,
            destination_chain_id,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()?;

    let tx = ctx.rpc.get_transaction(&sig, UiTransactionEncoding::Json)?;
    let payload =
        util::wormhole::find_post_message_payload(&tx).expect("Send Token payload not found");

    let portal_global_bytes = ctx
        .rpc
        .get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut portal_global_bytes.as_slice())?;

    match payload.data {
        PayloadData::TokenTransfer(token_payload) => {
            assert_eq!(payload.header.index, portal_global.m_index);
            assert_eq!(token_payload.amount, AMOUNT as u128); // exact extension amount is sent
            assert_eq!(token_payload.destination_token, ctx.destination_token);
            assert_eq!(token_payload.sender, ctx.portal.payer().to_bytes());
            assert_eq!(token_payload.recipient, ctx.portal.payer().to_bytes());
        }
        _ => panic!("Expected TokenTransferPayload"),
    }

    // assert the $M was burned
    let balance = ctx.rpc.get_token_account_balance(&ctx.m_token_account)?;
    let amount: u64 = balance.amount.parse().unwrap();
    assert!(amount < 50); // some residual tokens in the account

    Ok(())
}

#[test]
fn test_03_send_token_hyperlane_success() -> Result<()> {
    let ctx = TestCtx::new()?;
    let destination_chain_id: u32 = 1;
    let hyp = ctx.hyperlane_remaining_accounts(1)?;

    let instructions = ctx
        .portal
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(600_000))
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            extension_global: pda!(&[GLOBAL_SEED], &ctx.extension_program),
            m_mint: ctx.m_mint,
            extension_mint: ctx.extension_mint,
            m_token_account: ctx.m_token_account,
            extension_token_account: ctx.extension_token_account,
            portal_authority: ctx.portal_authority,
            ext_m_vault: ctx.ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ctx.extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ctx.extension_program),
            swap_program: ext_swap::ID,
            extension_program: ctx.extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: hyperlane_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: AMOUNT,
            destination_token: ctx.destination_token,
            destination_chain_id,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(hyp.to_account_metas())
        .instructions()?;

    let versioned_tx = build_versioned_tx_with_lut(ctx.rpc.clone(), instructions)?;
    ctx.rpc.send_and_confirm_transaction(&versioned_tx)?;

    // assert the $M was burned
    let balance = ctx.rpc.get_token_account_balance(&ctx.m_token_account)?;
    let amount: u64 = balance.amount.parse().unwrap();
    assert!(amount < 50); // some residual tokens in the account

    let message_data = ctx.rpc.get_account_data(&hyp.dispatched_message)?;
    let (payload, _) = util::hyperlane::decode_payload_from_message_account(&message_data)
        .expect("Failed to decode index payload");

    match payload.data {
        PayloadData::TokenTransfer(payload) => {
            assert_eq!(payload.recipient, ctx.portal.payer().to_bytes());
            assert_eq!(payload.amount, AMOUNT as u128); // exact extension amount is sent
        }
        _ => panic!("Expected TokenTransfer"),
    }

    Ok(())
}

#[test]
fn test_04_send_token_invalid_m_mint() -> Result<()> {
    let ctx = TestCtx::new()?;
    let destination_chain_id: u32 = 1;

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            extension_global: pda!(&[GLOBAL_SEED], &ctx.extension_program),
            m_mint: Pubkey::new_unique(), // invalid m_mint
            extension_mint: ctx.extension_mint,
            m_token_account: ctx.m_token_account,
            extension_token_account: ctx.extension_token_account,
            portal_authority: ctx.portal_authority,
            ext_m_vault: ctx.ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ctx.extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ctx.extension_program),
            swap_program: ext_swap::ID,
            extension_program: ctx.extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: AMOUNT,
            destination_token: ctx.destination_token,
            destination_chain_id,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("custom program error: 0xbbf"), "got: {}", s);
    assert!(
        s.contains("AnchorError caused by account: m_mint"),
        "got: {}",
        s
    );

    Ok(())
}
