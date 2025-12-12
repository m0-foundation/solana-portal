#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Payload {
    TokenTransfer(TokenTransferPayload),
    Index(IndexPayload),
    RegistrarList(RegistrarListPayload),
    FillReport(FillReportPayload),
}

impl Payload {
    const TOKEN_TRANSFER_DISCRIMINANT: u8 = 0;
    const INDEX_DISCRIMINANT: u8 = 1;
    const REGISTRAR_LIST_DISCRIMINANT: u8 = 3;
    const FILL_REPORT_DISCRIMINANT: u8 = 4;

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
            Payload::RegistrarList(payload) => {
                let mut data = vec![Self::REGISTRAR_LIST_DISCRIMINANT];
                data.extend_from_slice(&payload.list_name);
                data.extend_from_slice(&payload.address);
                data.push(if payload.add { 1 } else { 0 });
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
            Self::REGISTRAR_LIST_DISCRIMINANT => {
                let (list_name, data) = data.split_at(32);
                let (address_bytes, data) = data.split_at(32);
                let (add, data) = data.split_at(1);
                let (message_id_bytes, _) = data.split_at(32);

                Payload::RegistrarList(RegistrarListPayload {
                    list_name: list_name.try_into().unwrap(),
                    address: address_bytes.try_into().unwrap(),
                    add: add[0] == 1,
                    message_id: message_id_bytes.try_into().unwrap(),
                })
            }
            _ => panic!("Invalid payload type"),
        }
    }

    pub fn message_id(&self) -> [u8; 32] {
        match self {
            Payload::TokenTransfer(payload) => payload.message_id,
            Payload::Index(payload) => payload.message_id,
            Payload::FillReport(payload) => payload.message_id,
            Payload::RegistrarList(payload) => payload.message_id,
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

impl Into<IndexPayload> for TokenTransferPayload {
    fn into(self) -> IndexPayload {
        IndexPayload {
            index: self.index,
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
pub struct RegistrarListPayload {
    pub list_name: [u8; 32],
    pub address: [u8; 32],
    pub add: bool,
    pub message_id: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListType {
    SolanaEarners,
    Unsupported(String),
}

impl RegistrarListPayload {
    pub fn name(&self) -> String {
        // Find first trailing zero (0x00); if none, use full length
        let end = self
            .list_name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.list_name.len());

        String::from_utf8(self.list_name[..end].to_vec()).expect("bytes32 contained invalid UTF-8")
    }

    pub fn list_type(&self) -> ListType {
        let name = self.name();
        match name.as_str() {
            "solana-earners" => ListType::SolanaEarners,
            _ => ListType::Unsupported(name),
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
    fn test_registrar_list_encode_decode() {
        let original = RegistrarListPayload {
            list_name: [10u8; 32],
            address: [11u8; 32],
            add: true,
            message_id: [12u8; 32],
        };

        let payload = Payload::RegistrarList(original);
        let encoded = payload.encode();
        let decoded = Payload::decode(&encoded);

        match decoded {
            Payload::RegistrarList(decoded_payload) => {
                assert_eq!(decoded_payload.list_name, [10u8; 32]);
                assert_eq!(decoded_payload.address, [11u8; 32]);
                assert_eq!(decoded_payload.add, true);
                assert_eq!(decoded_payload.message_id, [12u8; 32]);
            }
            _ => panic!("Expected RegistrarList payload"),
        }
    }

    #[test]
    fn test_registrar_list_name() {
        let payload = RegistrarListPayload {
            list_name: [
                0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x2d, 0x65, 0x61, 0x72, 0x6e, 0x65, 0x72, 0x73,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
            address: [0u8; 32],
            add: true,
            message_id: [0u8; 32],
        };

        assert_eq!(payload.name(), "solana-earners");
    }
}
