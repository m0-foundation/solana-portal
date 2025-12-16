use crate::util::constants::{WH_EVENT_DISCRIMINATOR, WH_SHIM_POST_MESSAGE_DISCRIMINATOR};
use crate::util::DecodedIndex;
use common::IndexPayload;
use common::Payload;
use solana_sdk::bs58;
use solana_transaction_status_client_types::{
    UiInstruction,
    EncodedConfirmedTransactionWithStatusMeta,
    UiInnerInstructions,
    option_serializer::OptionSerializer,
};


// Matches the MessageEvent 
// see: https://github.com/wormhole-foundation/wormhole/blob/main/svm/wormhole-core-shims/programs/post-message/src/lib.rs#L211
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WormholeMessageEvent {
    pub emitter: [u8; 32],
    pub sequence: u64,
    pub timestamp: u32,
}

pub fn decode_event_cpi(data: &[u8]) -> Option<WormholeMessageEvent> {
    if data.get(0..8)? != WH_EVENT_DISCRIMINATOR {
        return None;
    }

    let emitter: [u8; 32] = data.get(16..48)?.try_into().ok()?;
    let sequence = u64::from_le_bytes(data.get(48..56)?.try_into().ok()?);
    let timestamp = u32::from_le_bytes(data.get(56..60)?.try_into().ok()?);

    Some(WormholeMessageEvent {
        emitter,
        sequence,
        timestamp,
    })
}

// Matches the Wormhole shim PostMessageData instruction data layout
// see: https://github.com/wormhole-foundation/wormhole/blob/c113791abd5241bc7a23655e3a7475085d51dab7/svm/wormhole-core-shims/crates/shim/src/post_message.rs#L108
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShimPostMessageData<'a> {
    pub nonce: u32,
    pub finality_byte: u8,
    pub payload: &'a [u8],
}

pub fn decode_shim_post_message<'a>(data: &'a [u8]) -> Option<ShimPostMessageData<'a>> {
    if data.get(0..8)? != WH_SHIM_POST_MESSAGE_DISCRIMINATOR {
        return None;
    }

    let shim = data.get(8..)?;
    if shim.len() < 9 {
        return None;
    }
    let nonce = u32::from_le_bytes(shim.get(0..4)?.try_into().ok()?);
    let finality_byte = *shim.get(4)?;
    let payload_len = u32::from_le_bytes(shim.get(5..9)?.try_into().ok()?) as usize;
    let end = 9usize.checked_add(payload_len)?;

    if shim.len() < end {
        return None;
    }

    let payload = &shim[9..end];
    Some(ShimPostMessageData {
        nonce,
        finality_byte,
        payload,
    })
}

pub fn decode_payload_from_shim_ix_data(data: &[u8]) -> Option<DecodedIndex> {
    let shim = decode_shim_post_message(data)?;
    let decoded = Payload::decode(&shim.payload.to_vec());

    if let Payload::Index(idx) = decoded {
        return Some(DecodedIndex {
            index: idx.index,
            message_id: idx.message_id,
        });
    }

    None
}

pub fn decode_payload_from_shim_ix_data_full(data: &[u8]) -> Option<Payload> {
    let shim = decode_shim_post_message(data)?;
    Some(Payload::decode(&shim.payload.to_vec()))
}

/// Generic helper: scan inner instructions, decode compiled ix data, and run a decoder.
/// Returns the first successful decode.
pub fn find_in_inner_instructions<T>( inner_instructions: &[UiInnerInstructions], mut decode: impl FnMut(&[u8]) -> Option<T>) -> Option<T> {
    for inner in inner_instructions {
        for ix in &inner.instructions {
            let UiInstruction::Compiled(compiled_ix) = ix else {
                continue;
            };

            let bytes: Vec<u8> = match bs58::decode(&compiled_ix.data).into_vec() {
                Ok(b) => b,
                Err(_) => continue,
            };

            if let Some(found) = decode(&bytes) {
                return Some(found);
            }
        }
    }
    None
}

pub fn find_message_event_in_tx(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Option<WormholeMessageEvent> {
    find_in_tx_inner_instructions(tx, decode_event_cpi)
}

pub fn find_index_payload_in_tx(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Option<IndexPayload> {
    find_in_tx_inner_instructions(tx, |bytes| {
        match decode_payload_from_shim_ix_data_full(bytes)? {
            Payload::Index(idx) => Some(idx),
            _ => None,
        }
    })
}

pub fn inner_instructions_from_tx(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Option<&[UiInnerInstructions]> {
    let meta = tx.transaction.meta.as_ref()?;

    match meta.inner_instructions.as_ref() {
        OptionSerializer::Some(v) => Some(v.as_slice()),
        OptionSerializer::None | OptionSerializer::Skip => None,
    }
}

/// Transaction-level wrapper
pub fn find_in_tx_inner_instructions<T>(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    decode: impl FnMut(&[u8]) -> Option<T>,
) -> Option<T> {
    let inner = inner_instructions_from_tx(tx)?;
    find_in_inner_instructions(inner, decode)
}

