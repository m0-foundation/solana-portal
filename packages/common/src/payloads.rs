#[repr(u8)]
#[derive(Debug)]
pub enum Payload {
    TokenTransfer(TokenTransferPayload),
    Index(IndexPayload),
    FillReport(FillReportPayload),
    EarnerMerkleRoot(EarnerMerkleRootPayload),
}

impl Payload {
    const TOKEN_TRANSFER_DISCRIMINANT: u8 = 0;
    const INDEX_DISCRIMINANT: u8 = 1;
    const FILL_REPORT_DISCRIMINANT: u8 = 4;
    const EARNER_MERKLE_ROOT_DISCRIMINANT: u8 = 5;

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Payload::TokenTransfer(payload) => {
                let mut data = vec![Self::TOKEN_TRANSFER_DISCRIMINANT];
                data.extend_from_slice(&payload.amount.to_be_bytes());
                data.extend_from_slice(&payload.destination_token);
                data.extend_from_slice(&payload.sender);
                data.extend_from_slice(&payload.recipient);
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.message_id);
                data
            }
            Payload::Index(payload) => {
                let mut data = vec![Self::INDEX_DISCRIMINANT];
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.message_id);
                data
            }
            Payload::EarnerMerkleRoot(payload) => {
                let mut data = vec![Self::EARNER_MERKLE_ROOT_DISCRIMINANT];
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.merkle_root);
                data.extend_from_slice(&payload.message_id);
                data
            }
            Payload::FillReport(payload) => {
                let mut data = vec![Self::FILL_REPORT_DISCRIMINANT];
                data.extend_from_slice(&payload.order_id);
                data.extend_from_slice(&payload.amount_in_to_release.to_be_bytes());
                data.extend_from_slice(&payload.amount_out_filled.to_be_bytes());
                data.extend_from_slice(&payload.origin_recipient);
                data.extend_from_slice(&payload.token_in);
                data.extend_from_slice(&payload.message_id);
                data
            }
        }
    }

    pub fn decode(data: &Vec<u8>) -> Self {
        let (payload_type, data) = data.split_at(1);

        match payload_type[0] {
            Self::TOKEN_TRANSFER_DISCRIMINANT => {
                let (amount_bytes, data) = data.split_at(16);
                let (destination_token_bytes, data) = data.split_at(32);
                let (sender_bytes, data) = data.split_at(32);
                let (recipient_bytes, data) = data.split_at(32);
                let (index_bytes, data) = data.split_at(8);
                let (message_id_bytes, _) = data.split_at(32);

                Payload::TokenTransfer(TokenTransferPayload {
                    amount: u128::from_be_bytes(amount_bytes.try_into().unwrap()),
                    destination_token: destination_token_bytes.try_into().unwrap(),
                    sender: sender_bytes.try_into().unwrap(),
                    recipient: recipient_bytes.try_into().unwrap(),
                    index: u64::from_be_bytes(index_bytes.try_into().unwrap()),
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            Self::INDEX_DISCRIMINANT => {
                let (index_bytes, message_id_bytes) = data.split_at(8);
                let (message_id_bytes, _) = message_id_bytes.split_at(32);

                Payload::Index(IndexPayload {
                    index: u64::from_be_bytes(index_bytes.try_into().unwrap()),
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            Self::FILL_REPORT_DISCRIMINANT => {
                let (order_id_bytes, data) = data.split_at(32);
                let (amount_in_to_release_bytes, data) = data.split_at(16);
                let (amount_out_filled_bytes, data) = data.split_at(16);
                let (origin_recipient_bytes, data) = data.split_at(32);
                let (token_in_bytes, data) = data.split_at(32);
                let (message_id_bytes, _) = data.split_at(32);

                Payload::FillReport(FillReportPayload {
                    order_id: order_id_bytes.try_into().unwrap(),
                    amount_in_to_release: u128::from_be_bytes(
                        amount_in_to_release_bytes.try_into().unwrap(),
                    ),
                    amount_out_filled: u128::from_be_bytes(
                        amount_out_filled_bytes.try_into().unwrap(),
                    ),
                    origin_recipient: origin_recipient_bytes.try_into().unwrap(),
                    token_in: token_in_bytes.try_into().unwrap(),
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            Self::EARNER_MERKLE_ROOT_DISCRIMINANT => {
                let (index_bytes, merkle_root_bytes) = data.split_at(8);
                let (merkle_root_bytes, data) = merkle_root_bytes.split_at(32);
                let (message_id_bytes, _) = data.split_at(32);

                Payload::EarnerMerkleRoot(EarnerMerkleRootPayload {
                    index: u64::from_be_bytes(index_bytes.try_into().unwrap()),
                    merkle_root: merkle_root_bytes.try_into().unwrap(),
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            _ => panic!("Invalid payload type"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenTransferPayload {
    pub amount: u128,
    pub destination_token: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub index: u64,
    pub message_id: [u8; 32],
}

impl Into<EarnerMerkleRootPayload> for TokenTransferPayload {
    fn into(self) -> EarnerMerkleRootPayload {
        EarnerMerkleRootPayload {
            index: self.index,
            merkle_root: [0; 32],
            message_id: self.message_id,
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
    pub message_id: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct IndexPayload {
    pub index: u64,
    pub message_id: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct EarnerMerkleRootPayload {
    pub index: u64,
    pub merkle_root: [u8; 32],
    pub message_id: [u8; 32],
}

impl Into<EarnerMerkleRootPayload> for IndexPayload {
    fn into(self) -> EarnerMerkleRootPayload {
        EarnerMerkleRootPayload {
            index: self.index,
            merkle_root: [0; 32],
            message_id: self.message_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_transfer_encode_decode() {
        let original = TokenTransferPayload {
            amount: 1000000000000u128,
            destination_token: [1u8; 32],
            sender: [2u8; 32],
            recipient: [3u8; 32],
            index: 42,
            message_id: [4u8; 32],
        };

        let payload = Payload::TokenTransfer(original);
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded);

        match decoded {
            Payload::TokenTransfer(decoded_payload) => {
                assert_eq!(decoded_payload.amount, 1000000000000u128);
                assert_eq!(decoded_payload.destination_token, [1u8; 32]);
                assert_eq!(decoded_payload.sender, [2u8; 32]);
                assert_eq!(decoded_payload.recipient, [3u8; 32]);
                assert_eq!(decoded_payload.index, 42);
                assert_eq!(decoded_payload.message_id, [4u8; 32]);
            }
            _ => panic!("Expected TokenTransfer payload"),
        }
    }

    #[test]
    fn test_index_encode_decode() {
        let original = IndexPayload {
            index: 123,
            message_id: [5u8; 32],
        };

        let payload = Payload::Index(original);
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded);

        match decoded {
            Payload::Index(decoded_payload) => {
                assert_eq!(decoded_payload.index, 123);
                assert_eq!(decoded_payload.message_id, [5u8; 32]);
            }
            _ => panic!("Expected Index payload"),
        }
    }

    #[test]
    fn test_fill_report_encode_decode() {
        let original = FillReportPayload {
            order_id: [6u8; 32],
            amount_in_to_release: 5000000000000u128,
            amount_out_filled: 4900000000000u128,
            origin_recipient: [7u8; 32],
            token_in: [8u8; 32],
            message_id: [9u8; 32],
        };

        let payload = Payload::FillReport(original);
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded);

        match decoded {
            Payload::FillReport(decoded_payload) => {
                assert_eq!(decoded_payload.order_id, [6u8; 32]);
                assert_eq!(decoded_payload.amount_in_to_release, 5000000000000u128);
                assert_eq!(decoded_payload.amount_out_filled, 4900000000000u128);
                assert_eq!(decoded_payload.origin_recipient, [7u8; 32]);
                assert_eq!(decoded_payload.token_in, [8u8; 32]);
                assert_eq!(decoded_payload.message_id, [9u8; 32]);
            }
            _ => panic!("Expected FillReport payload"),
        }
    }

    #[test]
    fn test_earner_merkle_root_encode_decode() {
        let original = EarnerMerkleRootPayload {
            index: 999,
            merkle_root: [10u8; 32],
            message_id: [11u8; 32],
        };

        let payload = Payload::EarnerMerkleRoot(original);
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded);

        match decoded {
            Payload::EarnerMerkleRoot(decoded_payload) => {
                assert_eq!(decoded_payload.index, 999);
                assert_eq!(decoded_payload.merkle_root, [10u8; 32]);
                assert_eq!(decoded_payload.message_id, [11u8; 32]);
            }
            _ => panic!("Expected EarnerMerkleRoot payload"),
        }
    }
}
