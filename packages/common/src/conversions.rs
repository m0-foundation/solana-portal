use anchor_lang::prelude::*;
use spl_token_2022::{
    extension::{
        scaled_ui_amount::ScaledUiAmountConfig, BaseStateWithExtensions, StateWithExtensions,
    },
    state,
};

const INDEX_SCALE_F64: f64 = 1e12;
const INDEX_SCALE_U64: u64 = 1_000_000_000_000;

pub fn amount_to_principal_up(amount: u64, multiplier: f64) -> u64 {
    if multiplier == 1.0 {
        return amount;
    }

    let index = (multiplier * INDEX_SCALE_F64).trunc() as u128;

    (amount as u128)
        .checked_mul(INDEX_SCALE_U64 as u128)
        .expect("overflow")
        .checked_add(index - 1)
        .expect("overflow")
        .checked_div(index)
        .expect("underflow")
        .try_into()
        .expect("overflow")
}

pub fn principal_to_amount_down(principal: u64, multiplier: f64) -> u128 {
    if multiplier == 1.0 {
        return principal as u128;
    }

    let index = (multiplier * INDEX_SCALE_F64).trunc() as u128;

    index
        .checked_mul(principal as u128)
        .expect("overflow")
        .checked_div(INDEX_SCALE_U64 as u128)
        .expect("underflow")
}

pub fn get_scaled_ui_config<'info>(mint: &AccountInfo<'info>) -> Result<ScaledUiAmountConfig> {
    let mint_data = mint.try_borrow_data()?;
    let mint_ext_data = StateWithExtensions::<state::Mint>::unpack(&mint_data)?;

    // Get the scaled UI config extension
    let scaled_ui_config = mint_ext_data.get_extension::<ScaledUiAmountConfig>()?;

    Ok(*scaled_ui_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Realistic multiplier range: 1.0 to ~2.0 (100% yield)
    // Represented as scaled integers: 1_000_000_000_000 to 2_000_000_000_000
    fn multiplier_strategy() -> impl Strategy<Value = f64> {
        (1_000_000_000_000u64..=2_000_000_000_000u64).prop_map(|scaled| scaled as f64 / 1e12)
    }

    // Amount strategy: 1 to max safe amount (avoid overflow in calculations)
    fn amount_strategy() -> impl Strategy<Value = u64> {
        1u64..=1_000_000_000_000_000u64 // Up to 1 quadrillion (well within u64)
    }

    proptest! {
        /// Principal calculated with round-up should always be sufficient to cover
        /// the original amount when converted back with round-down.
        #[test]
        fn principal_covers_original_amount(
            amount in amount_strategy(),
            multiplier in multiplier_strategy()
        ) {
            let principal = amount_to_principal_up(amount, multiplier);
            let recovered = principal_to_amount_down(principal, multiplier);

            prop_assert!(
                recovered >= amount as u128,
                "Principal {} (from amount {} at multiplier {}) only recovers {}, expected >= {}",
                principal, amount, multiplier, recovered, amount
            );
        }

        /// The overmint from rounding up should be bounded - at most 1 unit of principal
        /// worth of extra tokens.
        #[test]
        fn overmint_is_bounded(
            amount in amount_strategy(),
            multiplier in multiplier_strategy()
        ) {
            let principal = amount_to_principal_up(amount, multiplier);
            let recovered = principal_to_amount_down(principal, multiplier);

            // The maximum overmint should be at most what 1 additional principal would yield
            let one_principal_value = principal_to_amount_down(1, multiplier);
            let overmint = recovered - amount as u128;

            prop_assert!(
                overmint <= one_principal_value,
                "Overmint {} exceeds 1 principal value {} (amount: {}, multiplier: {})",
                overmint, one_principal_value, amount, multiplier
            );
        }

        /// When multiplier is exactly 1.0, conversions should be identity
        #[test]
        fn identity_at_multiplier_one(amount in amount_strategy()) {
            let principal = amount_to_principal_up(amount, 1.0);
            let recovered = principal_to_amount_down(principal, 1.0);

            prop_assert_eq!(principal, amount);
            prop_assert_eq!(recovered, amount as u128);
        }

        /// Principal should always be <= amount (since multiplier >= 1.0)
        #[test]
        fn principal_lte_amount(
            amount in amount_strategy(),
            multiplier in multiplier_strategy()
        ) {
            let principal = amount_to_principal_up(amount, multiplier);

            prop_assert!(
                principal <= amount,
                "Principal {} exceeds amount {} at multiplier {}",
                principal, amount, multiplier
            );
        }

        /// Simulates a full send/receive cycle:
        /// 1. User sends `amount` extension tokens
        /// 2. Payload contains `amount`
        /// 3. Receiver calculates principal with round-up
        /// 4. Receiver mints `amount` extension tokens
        /// The principal should be sufficient to back the minted tokens.
        #[test]
        fn send_receive_roundtrip(
            amount in amount_strategy(),
            multiplier in multiplier_strategy()
        ) {
            // On receive: calculate principal needed to back `amount` extension tokens
            let principal = amount_to_principal_up(amount, multiplier);

            // Verify: the principal, when converted to extension tokens, covers the amount
            let backed_amount = principal_to_amount_down(principal, multiplier);

            prop_assert!(
                backed_amount >= amount as u128,
                "Send/receive failed: sent {}, principal {}, backed only {} (multiplier: {})",
                amount, principal, backed_amount, multiplier
            );
        }
    }

    #[test]
    fn test_known_values() {
        // At multiplier 1.1 (10% yield), 1_000_000 extension tokens
        // should require ceil(1_000_000 / 1.1) = 909_091 principal
        let multiplier = 1.1;
        let amount = 1_000_000u64;

        let principal = amount_to_principal_up(amount, multiplier);
        // 1_000_000 * 1e12 / (1.1 * 1e12) = 909090.909..., rounded up = 909091
        assert_eq!(principal, 909_091);

        // Converting back: 909_091 * 1.1 = 1_000_000.1, truncated = 1_000_000
        let recovered = principal_to_amount_down(principal, multiplier);
        assert!(recovered >= amount as u128);
    }

    #[test]
    fn test_edge_case_small_amount() {
        // Very small amount at high multiplier
        let multiplier = 1.5;
        let amount = 1u64;

        let principal = amount_to_principal_up(amount, multiplier);
        // ceil(1 / 1.5) = ceil(0.666...) = 1
        assert_eq!(principal, 1);

        let recovered = principal_to_amount_down(principal, multiplier);
        // 1 * 1.5 = 1.5, truncated = 1
        assert!(recovered >= amount as u128);
    }
}
