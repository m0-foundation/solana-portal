use anchor_lang::prelude::*;
use m0_portal_common::Peers;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

#[account]
pub struct LayerZeroGlobal {
    pub bump: u8,
    pub admin: Pubkey,
    pub pending_admin: Option<Pubkey>,
    pub chain_id: u32,
    pub endpoint_program: Pubkey,
    pub outgoing_paused: bool,
    pub incoming_paused: bool,
    pub peers: Peers,
    pub padding: [u8; 128],
}

impl LayerZeroGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + 32 + // pending_admin option + pubkey
        4 + // chain_id
        32 + // endpoint_program
        1 + // outgoing_paused
        1 + // incoming_paused
        Peers::size(peers) + // peers
        128 // padding
    }
}

/// Parameters for LZ receive instruction (called by executor).
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct LzReceiveParams {
    pub src_eid: u32,
    pub sender: [u8; 32],
    pub nonce: u64,
    pub guid: [u8; 32],
    pub message: Vec<u8>,
    pub extra_data: Vec<u8>,
}

/// Fee structure returned by the LZ endpoint quote instruction.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MessagingFee {
    pub native_fee: u64,
    pub lz_token_fee: u64,
}

/// Receipt from a successful LZ endpoint send.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MessagingReceipt {
    pub guid: [u8; 32],
    pub nonce: u64,
    pub fee: MessagingFee,
}

/// Account metadata for executor (lz_receive_types).
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct LzAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Parameters serialized into the LZ endpoint `send` CPI.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SendParams {
    pub dst_eid: u32,
    pub receiver: [u8; 32],
    pub message: Vec<u8>,
    pub options: Vec<u8>,
    pub native_fee: u64,
    pub lz_token_fee: u64,
}

/// Parameters serialized into the LZ endpoint `clear` CPI.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ClearParams {
    pub receiver: Pubkey,
    pub src_eid: u32,
    pub sender: [u8; 32],
    pub nonce: u64,
    pub guid: [u8; 32],
    pub message: Vec<u8>,
}

/// Parameters serialized into the LZ endpoint `register_oapp` CPI.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct RegisterOAppParams {
    pub delegate: Pubkey,
}

/// Parameters serialized into the LZ endpoint `set_delegate` CPI.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SetDelegateParams {
    pub delegate: Pubkey,
}

/// Parameters serialized into the LZ endpoint `quote` CPI.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct QuoteParams {
    pub sender: Pubkey,
    pub dst_eid: u32,
    pub receiver: [u8; 32],
    pub message: Vec<u8>,
    pub options: Vec<u8>,
    pub pay_in_lz_token: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use m0_portal_common::Peer;

    #[test]
    fn test_layerzero_global_size() {
        let peers = Peers::default().updated_peers(Peer {
            adapter_chain_id: 1,
            address: [0; 32],
            m0_chain_id: 1,
        });

        let instance = LayerZeroGlobal {
            bump: 0,
            admin: Pubkey::default(),
            pending_admin: Some(Pubkey::default()),
            chain_id: 0,
            endpoint_program: Pubkey::default(),
            outgoing_paused: false,
            incoming_paused: false,
            peers: peers.clone(),
            padding: [0u8; 128],
        };

        let mut buf = Vec::new();
        instance.serialize(&mut buf).unwrap();

        assert_eq!(LayerZeroGlobal::size(peers.len()), buf.len() + 8);
    }
}
