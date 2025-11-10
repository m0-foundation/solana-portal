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
                data
            }
            Payload::Index(payload) => {
                let mut data = vec![Self::INDEX_DISCRIMINANT];
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.message_id);
                data
            }
            Payload::EarnerMerkleRoot(payload) => {
                let mut data = vec![Self::FILL_REPORT_DISCRIMINANT];
                data.extend_from_slice(&payload.index.to_be_bytes());
                data.extend_from_slice(&payload.merkle_root);
                data
            }
            Payload::FillReport(payload) => {
                let mut data = vec![Self::EARNER_MERKLE_ROOT_DISCRIMINANT];
                data.extend_from_slice(&payload.order_id);
                data.extend_from_slice(&payload.amount_in_to_release.to_be_bytes());
                data.extend_from_slice(&payload.amount_out_filled.to_be_bytes());
                data.extend_from_slice(&payload.origin_recipient);
                data
            }
        }
    }

    pub fn decode(data: Vec<u8>) -> Self {
        let (payload_type, data) = data.split_at(1);

        match payload_type[0] {
            Self::TOKEN_TRANSFER_DISCRIMINANT => {
                let (amount_bytes, data) = data.split_at(16);
                let (destination_token_bytes, data) = data.split_at(32);
                let (sender_bytes, data) = data.split_at(32);
                let (recipient_bytes, data) = data.split_at(32);
                let (index_bytes, _) = data.split_at(8);

                Payload::TokenTransfer(TokenTransferPayload {
                    amount: u128::from_le_bytes(amount_bytes.try_into().unwrap()),
                    destination_token: destination_token_bytes.try_into().unwrap(),
                    sender: sender_bytes.try_into().unwrap(),
                    recipient: recipient_bytes.try_into().unwrap(),
                    index: u64::from_le_bytes(index_bytes.try_into().unwrap()),
                })
            }
            Self::INDEX_DISCRIMINANT => {
                let (index_bytes, message_id_bytes) = data.split_at(8);
                let (message_id_bytes, _) = message_id_bytes.split_at(32);

                Payload::Index(IndexPayload {
                    index: u64::from_le_bytes(index_bytes.try_into().unwrap()),
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            Self::FILL_REPORT_DISCRIMINANT => {
                let (order_id_bytes, data) = data.split_at(32);
                let (amount_in_to_release_bytes, data) = data.split_at(16);
                let (amount_out_filled_bytes, data) = data.split_at(16);
                let (origin_recipient_bytes, _) = data.split_at(32);

                Payload::FillReport(FillReportPayload {
                    order_id: order_id_bytes.try_into().unwrap(),
                    amount_in_to_release: u128::from_le_bytes(
                        amount_in_to_release_bytes.try_into().unwrap(),
                    ),
                    amount_out_filled: u128::from_le_bytes(
                        amount_out_filled_bytes.try_into().unwrap(),
                    ),
                    origin_recipient: origin_recipient_bytes.try_into().unwrap(),
                })
            }
            Self::EARNER_MERKLE_ROOT_DISCRIMINANT => {
                let (index_bytes, merkle_root_bytes) = data.split_at(8);
                let (merkle_root_bytes, _) = merkle_root_bytes.split_at(32);

                Payload::EarnerMerkleRoot(EarnerMerkleRootPayload {
                    index: u64::from_le_bytes(index_bytes.try_into().unwrap()),
                    merkle_root: merkle_root_bytes.try_into().unwrap(),
                })
            }
            _ => panic!("Invalid payload type"),
        }
    }
}

#[derive(Debug)]
pub struct TokenTransferPayload {
    pub amount: u128,
    pub destination_token: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub index: u64,
}

impl Into<EarnerMerkleRootPayload> for TokenTransferPayload {
    fn into(self) -> EarnerMerkleRootPayload {
        EarnerMerkleRootPayload {
            index: self.index,
            merkle_root: [0; 32],
        }
    }
}

#[derive(Debug)]
pub struct FillReportPayload {
    pub order_id: [u8; 32],
    pub amount_in_to_release: u128,
    pub amount_out_filled: u128,
    pub origin_recipient: [u8; 32],
}

#[derive(Debug)]
pub struct IndexPayload {
    pub index: u64,
    pub message_id: [u8; 32],
}

#[derive(Debug)]
pub struct EarnerMerkleRootPayload {
    pub index: u64,
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
