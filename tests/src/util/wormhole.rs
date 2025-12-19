use crate::util::constants::{WH_EVENT_DISCRIMINATOR, WH_SHIM_POST_MESSAGE_DISCRIMINATOR};
use anyhow::Result;
use common::Payload;
use solana_sdk::bs58;
use solana_transaction_status_client_types::{
    option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta, UiInstruction,
};

// Wormhole MessageEvent structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WormholeMessageEvent {
    pub emitter: [u8; 32],
    pub sequence: u64,
    pub timestamp: u32,
}

pub fn decode_event_cpi(data: Vec<u8>) -> Option<WormholeMessageEvent> {
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

// Wormhole shim PostMessageData instruction layout
#[derive(Debug, Clone)]
pub struct ShimPostMessageData {
    pub nonce: u32,
    pub finality_byte: u8,
    pub payload: Payload,
}

pub fn decode_shim_post_message(data: Vec<u8>) -> Option<ShimPostMessageData> {
    let (disc, data) = data.split_at(8);
    if disc != WH_SHIM_POST_MESSAGE_DISCRIMINATOR || data.len() < 9 {
        return None;
    }

    let (nonce_bytes, data) = data.split_at(4);
    let (finality_byte_slice, data) = data.split_at(1);
    let (payload_len_bytes, data) = data.split_at(4);

    let nonce = u32::from_le_bytes(nonce_bytes.try_into().ok()?);
    let finality_byte = finality_byte_slice[0];
    let payload_len = u32::from_le_bytes(payload_len_bytes.try_into().ok()?) as usize;
    let (payload_bytes, _) = data.split_at(payload_len);
    let payload = Payload::decode(&payload_bytes.to_vec()).ok()?;

    Some(ShimPostMessageData {
        nonce,
        finality_byte,
        payload,
    })
}

pub fn find_post_message_payload(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Payload> {
    get_instructions_data(tx)?
        .into_iter()
        .find_map(decode_shim_post_message)
        .map(|msg| msg.payload)
        .ok_or_else(|| anyhow::anyhow!("Payload not found in inner instructions"))
}

pub fn find_message_event(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<WormholeMessageEvent> {
    get_instructions_data(tx)?
        .into_iter()
        .find_map(decode_event_cpi)
        .ok_or_else(|| anyhow::anyhow!("Event not found in inner instructions"))
}

pub fn get_instructions_data(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Vec<Vec<u8>>> {
    let inner = &tx.transaction.meta.as_ref().unwrap().inner_instructions;

    let instructions = match inner {
        OptionSerializer::Some(v) => v.as_slice(),
        OptionSerializer::None | OptionSerializer::Skip => {
            return Err(anyhow::anyhow!("No inner instructions found"))
        }
    };

    Ok(instructions
        .iter()
        .flat_map(|inner| &inner.instructions)
        .filter_map(|ix| match ix {
            UiInstruction::Compiled(compiled) => {
                Some(bs58::decode(&compiled.data).into_vec().unwrap())
            }
            _ => None,
        })
        .collect())
}
