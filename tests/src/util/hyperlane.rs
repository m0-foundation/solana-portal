use anyhow::Result;
use common::{IndexPayload, Payload, PayloadData};

pub fn decode_message_account_index_payload(account_data: &[u8]) -> Result<IndexPayload> {
    // The last 40 bytes of the account data contain the message body
    let len = account_data.len();
    let message_body = &account_data[len - 41..];
    let message = Payload::decode(&message_body.to_vec())?;

    let PayloadData::Index(payload) = message.data else {
        panic!("Expected IndexPayload");
    };

    Ok(payload)
}
