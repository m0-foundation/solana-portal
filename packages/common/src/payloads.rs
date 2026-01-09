use anchor_lang::{err, Result};

use crate::BridgeError;

#[derive(Debug, Clone)]
pub struct Payload {
    pub header: PayloadHeader,
    pub data: PayloadData,
}

#[derive(Debug, Clone)]
pub struct PayloadHeader {
    pub payload_type: u8,
    pub destination_chain_id: u32,
    pub destination_peer: [u8; 32],
    pub message_id: [u8; 32],
}

impl PayloadHeader {
    pub const SIZE: usize = 1 + 4 + 32 + 32;

    pub fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.payload_type.to_be_bytes());
        data.extend_from_slice(&self.destination_chain_id.to_be_bytes());
        data.extend_from_slice(&self.destination_peer);
        data.extend_from_slice(&self.message_id);
        data
    }

    pub fn decode(data: &[u8]) -> (Self, &[u8]) {
        let (payload_type_bytes, data) = data.split_at(1);
        let (chain_id_bytes, data) = data.split_at(4);
        let (destination_peer_bytes, data) = data.split_at(32);
        let (message_id_bytes, data) = data.split_at(32);

        (
            PayloadHeader {
                payload_type: payload_type_bytes[0],
                destination_chain_id: u32::from_be_bytes(chain_id_bytes.try_into().unwrap()),
                destination_peer: destination_peer_bytes.try_into().unwrap(),
                message_id: message_id_bytes.try_into().unwrap(),
            },
            data,
        )
    }
}

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum PayloadData {
    TokenTransfer(TokenTransferPayload),
    Index(IndexPayload),
    FillReport(FillReportPayload),
    EarnerMerkleRoot(EarnerMerkleRootPayload),
    CancelReport(CancelReportPayload),
}

impl PayloadData {
    pub const TOKEN_TRANSFER_DISCRIMINANT: u8 = 0;
    pub const INDEX_DISCRIMINANT: u8 = 1;
    pub const FILL_REPORT_DISCRIMINANT: u8 = 4;
    pub const EARNER_MERKLE_ROOT_DISCRIMINANT: u8 = 5;
    pub const CANCEL_REPORT_DISCRIMINANT: u8 = 6;

    pub fn encode(&self) -> Vec<u8> {
        match &self {
            PayloadData::TokenTransfer(payload) => {
                let mut data = vec![];
                data.extend_from_slice(&payload.amount.to_be_bytes());
                data.extend_from_slice(&payload.destination_token);
                data.extend_from_slice(&payload.sender);
                data.extend_from_slice(&payload.recipient);
                data.extend_from_slice(&payload.index.to_be_bytes());
                data
            }
            PayloadData::Index(payload) => payload.index.to_be_bytes().to_vec(),
            PayloadData::EarnerMerkleRoot(payload) => {
                let mut data = vec![];
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.merkle_root);
                data
            }
            PayloadData::FillReport(payload) => {
                let mut data = vec![];
                data.extend_from_slice(&payload.order_id);
                data.extend_from_slice(&payload.amount_in_to_release.to_be_bytes());
                data.extend_from_slice(&payload.amount_out_filled.to_be_bytes());
                data.extend_from_slice(&payload.origin_recipient);
                data.extend_from_slice(&payload.token_in);
                data
            }
            PayloadData::CancelReport(payload) => {
                let mut data = vec![];
                data.extend_from_slice(&payload.order_id);
                data.extend_from_slice(&payload.order_sender);
                data.extend_from_slice(&payload.token_in);
                data
            }
        }
    }

    pub fn decode(discriminant: u8, data: &[u8]) -> Result<Self> {
        match discriminant {
            Self::TOKEN_TRANSFER_DISCRIMINANT => {
                let (amount_bytes, data) = data.split_at(16);
                let (destination_token_bytes, data) = data.split_at(32);
                let (sender_bytes, data) = data.split_at(32);
                let (recipient_bytes, data) = data.split_at(32);
                let (index_bytes, _) = data.split_at(16);

                Ok(PayloadData::TokenTransfer(TokenTransferPayload {
                    amount: u128::from_be_bytes(amount_bytes.try_into().unwrap()),
                    destination_token: destination_token_bytes.try_into().unwrap(),
                    sender: sender_bytes.try_into().unwrap(),
                    recipient: recipient_bytes.try_into().unwrap(),
                    index: u128::from_be_bytes(index_bytes.try_into().unwrap()),
                }))
            }
            Self::INDEX_DISCRIMINANT => Ok(PayloadData::Index(IndexPayload {
                index: u128::from_be_bytes(data.try_into().unwrap()),
            })),
            Self::FILL_REPORT_DISCRIMINANT => {
                let (order_id_bytes, data) = data.split_at(32);
                let (amount_in_to_release_bytes, data) = data.split_at(16);
                let (amount_out_filled_bytes, data) = data.split_at(16);
                let (origin_recipient_bytes, data) = data.split_at(32);
                let (token_in_bytes, _) = data.split_at(32);

                Ok(PayloadData::FillReport(FillReportPayload {
                    order_id: order_id_bytes.try_into().unwrap(),
                    amount_in_to_release: u128::from_be_bytes(
                        amount_in_to_release_bytes.try_into().unwrap(),
                    ),
                    amount_out_filled: u128::from_be_bytes(
                        amount_out_filled_bytes.try_into().unwrap(),
                    ),
                    origin_recipient: origin_recipient_bytes.try_into().unwrap(),
                    token_in: token_in_bytes.try_into().unwrap(),
                }))
            }
            Self::EARNER_MERKLE_ROOT_DISCRIMINANT => {
                let (index_bytes, data) = data.split_at(16);
                let (merkle_root_bytes, _) = data.split_at(32);

                Ok(PayloadData::EarnerMerkleRoot(EarnerMerkleRootPayload {
                    index: u128::from_be_bytes(index_bytes.try_into().unwrap()),
                    merkle_root: merkle_root_bytes.try_into().unwrap(),
                }))
            }
            Self::CANCEL_REPORT_DISCRIMINANT => {
                let (order_id_bytes, data) = data.split_at(32);
                let (order_sender_bytes, data) = data.split_at(32);
                let (token_in_bytes, _) = data.split_at(32);
                let (amount_in_to_refund_bytes, _) = data.split_at(16);

                Ok(PayloadData::CancelReport(CancelReportPayload {
                    order_id: order_id_bytes.try_into().unwrap(),
                    order_sender: order_sender_bytes.try_into().unwrap(),
                    token_in: token_in_bytes.try_into().unwrap(),
                    amount_in_to_refund: u128::from_be_bytes(
                        amount_in_to_refund_bytes.try_into().unwrap(),
                    ),
                }))
            }
            _ => err!(BridgeError::InvalidPayload),
        }
    }
}

impl Payload {
    pub fn encode(&self) -> Vec<u8> {
        let mut data = vec![];
        data.extend(&self.header.encode());
        data.extend(&self.data.encode());
        data
    }

    pub fn decode(data: &Vec<u8>) -> Result<Self> {
        let (header, data) = PayloadHeader::decode(data);
        let payload_data = PayloadData::decode(header.payload_type, data)?;

        Ok(Payload {
            header,
            data: payload_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TokenTransferPayload {
    pub amount: u128,
    pub destination_token: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub index: u128,
}

impl Into<EarnerMerkleRootPayload> for TokenTransferPayload {
    fn into(self) -> EarnerMerkleRootPayload {
        EarnerMerkleRootPayload {
            index: self.index,
            merkle_root: [0; 32],
        }
    }
}

#[derive(Debug, Clone)]
pub struct FillReportPayload {
    pub order_id: [u8; 32],
    pub amount_in_to_release: u128,
    pub amount_out_filled: u128,
    pub origin_recipient: [u8; 32],
    pub token_in: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct CancelReportPayload {
    pub order_id: [u8; 32],
    pub order_sender: [u8; 32],
    pub token_in: [u8; 32],
    pub amount_in_to_refund: u128,
}

#[derive(Debug, Clone)]
pub struct IndexPayload {
    pub index: u128,
}

#[derive(Debug, Clone)]
pub struct EarnerMerkleRootPayload {
    pub index: u128,
    pub merkle_root: [u8; 32],
}

impl Into<EarnerMerkleRootPayload> for IndexPayload {
    fn into(self) -> EarnerMerkleRootPayload {
        EarnerMerkleRootPayload {
            index: self.index,
            merkle_root: [0; 32],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_transfer_encode_decode() {
        let header = PayloadHeader {
            payload_type: 0,
            destination_chain_id: 56,
            destination_peer: [5u8; 32],
            message_id: [4u8; 32],
        };

        let payload_data = TokenTransferPayload {
            amount: 1000000000000u128,
            destination_token: [1u8; 32],
            sender: [2u8; 32],
            recipient: [3u8; 32],
            index: 42,
        };

        let payload = Payload {
            header: header.clone(),
            data: PayloadData::TokenTransfer(payload_data),
        };
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded).unwrap();

        match decoded.data {
            PayloadData::TokenTransfer(decoded_payload) => {
                assert_eq!(decoded_payload.amount, 1000000000000u128);
                assert_eq!(decoded_payload.destination_token, [1u8; 32]);
                assert_eq!(decoded_payload.sender, [2u8; 32]);
                assert_eq!(decoded_payload.recipient, [3u8; 32]);
                assert_eq!(decoded_payload.index, 42);
                assert_eq!(decoded.header.message_id, [4u8; 32]);
                assert_eq!(decoded.header.destination_chain_id, 56);
                assert_eq!(decoded.header.destination_peer, [5u8; 32]);
                assert_eq!(decoded.header.payload_type, 0);
            }
            _ => panic!("Expected TokenTransfer payload"),
        }
    }

    #[test]
    fn test_index_encode_decode() {
        let header = PayloadHeader {
            payload_type: 1,
            destination_chain_id: 56,
            destination_peer: [6u8; 32],
            message_id: [5u8; 32],
        };

        let payload_data = IndexPayload { index: 123 };

        let payload = Payload {
            header: header.clone(),
            data: PayloadData::Index(payload_data),
        };
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded).unwrap();

        match decoded.data {
            PayloadData::Index(decoded_payload) => {
                assert_eq!(decoded_payload.index, 123);
                assert_eq!(decoded.header.message_id, [5u8; 32]);
                assert_eq!(decoded.header.destination_chain_id, 56);
                assert_eq!(decoded.header.destination_peer, [6u8; 32]);
                assert_eq!(decoded.header.payload_type, 1);
            }
            _ => panic!("Expected Index payload"),
        }
    }

    #[test]
    fn test_fill_report_encode_decode() {
        let header = PayloadHeader {
            payload_type: 4,
            destination_chain_id: 56,
            destination_peer: [10u8; 32],
            message_id: [9u8; 32],
        };

        let payload_data = FillReportPayload {
            order_id: [6u8; 32],
            amount_in_to_release: 5000000000000u128,
            amount_out_filled: 4900000000000u128,
            origin_recipient: [7u8; 32],
            token_in: [8u8; 32],
        };

        let payload = Payload {
            header: header.clone(),
            data: PayloadData::FillReport(payload_data),
        };
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded).unwrap();

        match decoded.data {
            PayloadData::FillReport(decoded_payload) => {
                assert_eq!(decoded_payload.order_id, [6u8; 32]);
                assert_eq!(decoded_payload.amount_in_to_release, 5000000000000u128);
                assert_eq!(decoded_payload.amount_out_filled, 4900000000000u128);
                assert_eq!(decoded_payload.origin_recipient, [7u8; 32]);
                assert_eq!(decoded_payload.token_in, [8u8; 32]);
                assert_eq!(decoded.header.message_id, [9u8; 32]);
                assert_eq!(decoded.header.destination_chain_id, 56);
                assert_eq!(decoded.header.destination_peer, [10u8; 32]);
                assert_eq!(decoded.header.payload_type, 4);
            }
            _ => panic!("Expected FillReport payload"),
        }
    }

    #[test]
    fn test_earner_merkle_root_encode_decode() {
        let header = PayloadHeader {
            payload_type: 5,
            destination_chain_id: 56,
            destination_peer: [12u8; 32],
            message_id: [11u8; 32],
        };

        let payload_data = EarnerMerkleRootPayload {
            index: 999,
            merkle_root: [10u8; 32],
        };

        let payload = Payload {
            header: header.clone(),
            data: PayloadData::EarnerMerkleRoot(payload_data),
        };
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded).unwrap();

        match decoded.data {
            PayloadData::EarnerMerkleRoot(decoded_payload) => {
                assert_eq!(decoded_payload.index, 999);
                assert_eq!(decoded_payload.merkle_root, [10u8; 32]);
                assert_eq!(decoded.header.message_id, [11u8; 32]);
                assert_eq!(decoded.header.destination_chain_id, 56);
                assert_eq!(decoded.header.destination_peer, [12u8; 32]);
                assert_eq!(decoded.header.payload_type, 5);
            }
            _ => panic!("Expected EarnerMerkleRoot payload"),
        }
    }

    #[test]
    fn test_cancel_report_encode_decode() {
        let header = PayloadHeader {
            payload_type: 6,
            destination_chain_id: 56,
            destination_peer: [14u8; 32],
            message_id: [13u8; 32],
        };
        let payload_data = CancelReportPayload {
            order_id: [15u8; 32],
            order_sender: [16u8; 32],
            token_in: [17u8; 32],
            amount_in_to_refund: 1000_000000u128,
        };
        let payload = Payload {
            header: header.clone(),
            data: PayloadData::CancelReport(payload_data),
        };
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded).unwrap();
        match decoded.data {
            PayloadData::CancelReport(decoded_payload) => {
                assert_eq!(decoded_payload.order_id, [15u8; 32]);
                assert_eq!(decoded_payload.order_sender, [16u8; 32]);
                assert_eq!(decoded_payload.token_in, [17u8; 32]);
                assert_eq!(decoded_payload.amount_in_to_refund, 1000_000000u128);
                assert_eq!(decoded.header.message_id, [13u8; 32]);
                assert_eq!(decoded.header.destination_chain_id, 56);
                assert_eq!(decoded.header.destination_peer, [14u8; 32]);
                assert_eq!(decoded.header.payload_type, 6);
            }
            _ => panic!("Expected CancelReport payload"),
        }
    }
}
