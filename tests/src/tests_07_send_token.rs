use std::{str::FromStr, sync::Arc};

use anchor_client::{Client, Cluster, Program};
use anchor_lang::{prelude::Pubkey, system_program, AccountDeserialize};
use anchor_spl::token_2022;
use anyhow::{Ok, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    compute_budget::ComputeBudgetInstruction,
    message::{v0, VersionedMessage},
    signature::Keypair,
    transaction::VersionedTransaction,
};
use solana_transaction_status_client_types::UiTransactionEncoding;
use spl_token_2022::{
    extension::StateWithExtensions,
    state::{Account as TokenAccount2022, AccountState},
};

use common::{
    ext_swap::{self, accounts::SwapGlobal},
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
    },
    m_ext::{self, accounts::ExtGlobalV2},
    pda,
    portal::constants::{GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    wormhole_adapter::{self, accounts::WormholeGlobal},
    HyperlaneRemainingAccounts, PayloadData, WormholeRemainingAccounts, AUTHORITY_SEED,
};

use portal::{accounts as portal_accounts, instruction as portal_instruction, state::PortalGlobal};

use crate::{get_rpc_client, get_signer, set_account, set_token_account, util};

const AMOUNT: u64 = 1_000_000;

struct TestCtx {
    client: Client<Arc<Keypair>>,
    rpc: Arc<RpcClient>,
    portal: Program<Arc<Keypair>>,

    m_mint: Pubkey,
    extension_mint: Pubkey,
    extension_program: Pubkey,
    ext_global_pk: Pubkey,
    swap_global_pk: Pubkey,

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
        let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
        let ext_global_pk = pda!(&[GLOBAL_SEED], &extension_program);
        let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID);

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
            client,
            rpc,
            portal,
            ext_global_pk,
            swap_global_pk,
            m_mint,
            extension_mint,
            extension_program,
            portal_authority,
            m_token_account,
            extension_token_account,
            ext_m_vault,
        })
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
        ))
    }
}

fn assert_err_contains(err: impl ToString, any_of: &[&str], must: &[&str]) {
    let s = err.to_string();

    assert!(
        any_of.iter().any(|needle| s.contains(needle)),
        "Expected one of {:?}, got: {}",
        any_of,
        s
    );

    for needle in must {
        assert!(s.contains(needle), "Expected '{}' in: {}", needle, s);
    }
}

fn whitelist_portal_authority(ctx: &TestCtx) -> Result<()> {
    let mut swap_account = ctx.rpc.get_account(&ctx.swap_global_pk)?;
    let admin_offset = 9; // Offset: 8 (discriminator) + 1 (bump) = 9
    swap_account.data[admin_offset..admin_offset + 32]
        .copy_from_slice(&ctx.portal.payer().to_bytes());
    set_account(&ctx.swap_global_pk, &swap_account).expect("failed to set account");

    let mut ext_account: solana_sdk::account::Account = ctx.rpc.get_account(&ctx.ext_global_pk)?;
    let admin_offset = 8; // Offset: 8 (discriminator)
    ext_account.data[admin_offset..admin_offset + 32]
        .copy_from_slice(&ctx.portal.payer().to_bytes());
    set_account(&ctx.ext_global_pk, &ext_account).expect("failed to set account");

    // re-fetch bytes and deserialize that
    let swap_data = ctx.rpc.get_account_data(&ctx.swap_global_pk)?;
    let swap_acc = SwapGlobal::try_deserialize(&mut swap_data.as_slice())?;

    // re-fetch bytes and deserialize that
    let ext_data = ctx.rpc.get_account_data(&ctx.ext_global_pk)?;
    let ext_acc = ExtGlobalV2::try_deserialize(&mut ext_data.as_slice())?;

    assert_eq!(swap_acc.admin, ctx.portal.payer());
    assert_eq!(ext_acc.admin, ctx.portal.payer());

    // whitelist portal_authority as admin unwrap on swap
    let swap_program = ctx.client.program(ext_swap::ID)?;
    swap_program
        .request()
        .accounts(ext_swap::client::accounts::WhitelistUnwrapper {
            admin: ctx.portal.payer(),
            swap_global: ctx.swap_global_pk,
            system_program: system_program::ID,
        })
        .args(ext_swap::client::args::WhitelistUnwrapper {
            authority: ctx.portal_authority,
        })
        .send()?;

    // whitelist portal_authority as wrap auth on extension
    let extension_program = ctx.client.program(ctx.extension_program)?;
    extension_program
        .request()
        .accounts(m_ext::client::accounts::AddWrapAuthority {
            admin: ctx.portal.payer(),
            global_account: ctx.ext_global_pk,
            system_program: system_program::ID,
        })
        .args(m_ext::client::args::AddWrapAuthority {
            new_wrap_authority: ctx.portal_authority,
        })
        .send()?;

    Ok(())
}

#[test]
fn test_01_send_token_wormhole_unauthorized_unwrapper() -> Result<()> {
    let ctx = TestCtx::new()?;

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: ctx.swap_global_pk,
            extension_global: ctx.ext_global_pk,
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
            destination_token: ctx.m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    assert_err_contains(
        err,
        &["6003", "custom program error: 0x1778"],
        &["UnauthorizedUnwrapper"],
    );

    Ok(())
}

#[test]
fn test_02_send_token_hyperlane_unauthorized_unwrapper() -> Result<()> {
    let ctx = TestCtx::new()?;
    let hyp = ctx.hyperlane_remaining_accounts(0)?;

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
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
            destination_token: ctx.m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(hyp.to_account_metas())
        .send()
        .unwrap_err();

    assert_err_contains(
        err,
        &["6003", "custom program error: 0x1778"],
        &["UnauthorizedUnwrapper"],
    );

    Ok(())
}

#[test]
fn test_03_send_token_wormhole_insufficient_funds() -> Result<()> {
    let ctx = TestCtx::new()?;

    // whitelist portal authority as unwrapper and wrap authority
    whitelist_portal_authority(&ctx)?;

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
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
            destination_token: ctx.m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("custom program error: 0x1"), "got: {}", s);
    assert!(s.contains("insufficient funds"), "got: {}", s);

    Ok(())
}

#[test]
fn test_04_send_token_wormhole_success() -> Result<()> {
    let ctx = TestCtx::new()?;

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
            destination_token: ctx.m_mint.to_bytes(),
            destination_chain_id: 1,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
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
            assert_eq!(token_payload.index, portal_global.m_index);
            assert!(token_payload.amount < AMOUNT as u128 && token_payload.amount > 0); // will change depending on current index value
            assert_eq!(token_payload.destination_token, ctx.m_mint.to_bytes());
            assert_eq!(token_payload.sender, ctx.portal.payer().to_bytes());
            assert_eq!(token_payload.recipient, ctx.portal.payer().to_bytes());
        }
        _ => panic!("Expected TokenTransferPayload"),
    }

    // assert the $M was burned
    let balance = ctx.rpc.get_token_account_balance(&ctx.m_token_account)?;
    assert_eq!(balance.amount, "0");

    Ok(())
}

#[test]
fn test_05_send_token_hyperlane_success() -> Result<()> {
    let ctx = TestCtx::new()?;
    let hyp = ctx.hyperlane_remaining_accounts(1)?;

    let data_wh = ctx
        .rpc
        .get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;

    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;

    let lut = global_wh
        .receive_lut
        .expect("expected receive LUT to be initialized");

    let instructions = ctx
        .portal
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(600_000))
        .accounts(portal_accounts::SendToken {
            sender: ctx.portal.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
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
            destination_token: ctx.m_mint.to_bytes(),
            destination_chain_id: 1,
            recipient: ctx.portal.payer().to_bytes(),
        })
        .accounts(hyp.to_account_metas())
        .instructions()?;

    let recent_blockhash = ctx.rpc.get_latest_blockhash()?;

    // fetch the address lookup table account
    let lut_account = ctx.rpc.get_account(&lut)?;
    let address_lookup_table = AddressLookupTableAccount {
        key: lut,
        addresses: AddressLookupTable::deserialize(&lut_account.data)?
            .addresses
            .to_vec(),
    };

    // create versioned transaction with address lookup table
    let message = v0::Message::try_compile(
        &ctx.portal.payer(),
        &instructions,
        &[address_lookup_table],
        recent_blockhash,
    )?;

    // Send transaction
    let versioned_message = VersionedMessage::V0(message);
    let versioned_tx = VersionedTransaction::try_new(versioned_message, &[get_signer()])?;
    ctx.rpc.send_and_confirm_transaction(&versioned_tx)?;

    // assert the $M was burned
    let balance = ctx.rpc.get_token_account_balance(&ctx.m_token_account)?;
    assert_eq!(balance.amount, "0");

    Ok(())
}
