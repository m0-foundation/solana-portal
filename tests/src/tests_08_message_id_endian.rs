use anchor_lang::prelude::Pubkey;
use solana_sdk::keccak::hashv;
use portal::state::PortalGlobal;

#[test]
fn test_message_id_endian() {
    // Example values
    let chain_id: u32 = 1;
    let destination_chain_id: u32 = 1;
    let nonce: u64 = 42;

    // EVM-style/network order: big-endian bytes
    let be_id = hashv(&[&destination_chain_id.to_be_bytes(),&chain_id.to_be_bytes(), &nonce.to_be_bytes()]).to_bytes();
        // println!("Expected be digest: 0x{}", hex::encode(be_id));

    // Helper mirrors on-chain LE logic
    let expected = crate::util::compute_expected_message_id(destination_chain_id, chain_id, nonce);
        // println!("Expected digest: 0x{}", hex::encode(expected));

    // Call the production function directly on a constructed PortalGlobal
    // Note: generate_message_id() increments message_nonce, so start at nonce-1
    let mut pg = PortalGlobal {
        bump: 0,
        chain_id,
        admin: Pubkey::default(),
        paused: false,
        m_index: 0,
        message_nonce: nonce - 1,
        pending_admin: None,
        padding: [0u8; 128],
    };

    let prod_id = pg.generate_message_id(destination_chain_id);
    // println!("Prod digest: 0x{}", hex::encode(prod_id));

    // Production function should equal the helper
    assert_eq!(prod_id, expected, "Prod matches helper computation");

    // Sanity: BE digest should equal the prod id since both derive from same inputs
    assert_eq!(prod_id, be_id, "BE digest differs; cross-chain must agree on order");

    // And it should have incremented the nonce to the requested value
    assert_eq!(pg.message_nonce, nonce, "message_nonce should increment to nonce");
}
