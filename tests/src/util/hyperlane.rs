use anyhow::Result;
use common::Payload;

const DESTINATION_PEER: [u8; 32] = [
    11, 106, 134, 128, 106, 3, 84, 200, 43, 143, 4, 158, 183, 93, 156, 151, 227, 112, 166, 240,
    192, 207, 161, 95, 71, 144, 156, 63, 225, 200, 247, 148,
];

pub fn decode_payload_from_message_account(account_data: &[u8]) -> Result<(Payload, &[u8])> {
    // Scan for DESTINATION_PEER to find where the payload header starts
    // Search backwards to find the last occurrence (the actual payload header)
    let peer_index = account_data
        .windows(32)
        .rposition(|window| window == DESTINATION_PEER)
        .ok_or_else(|| anyhow::anyhow!("destination_peer not found in account data"))?;

    // The payload starts 5 bytes before the destination_peer field
    let payload_start = peer_index - 5;
    let payload_bytes = &account_data[payload_start..];

    println!("Payload bytes: {:?}", payload_bytes);
    let message = Payload::decode(&payload_bytes.to_vec())?;

    // Parse message recipient
    let recipient = &account_data[payload_start - 32..payload_start];

    Ok((message, recipient))
}
