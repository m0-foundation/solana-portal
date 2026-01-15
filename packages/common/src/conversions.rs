use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        scaled_ui_amount::ScaledUiAmountConfig, BaseStateWithExtensions, StateWithExtensions,
    },
    state,
};

const INDEX_SCALE_F64: f64 = 1e12;
const INDEX_SCALE_U64: u64 = 1_000_000_000_000;

pub fn amount_to_principal_down(amount: u128, multiplier: f64) -> u128 {
    if multiplier == 1.0 {
        return amount;
    }

    let index = (multiplier * INDEX_SCALE_F64).trunc() as u128;

    amount
        .checked_mul(INDEX_SCALE_U64 as u128)
        .expect("overflow")
        .checked_div(index)
        .expect("underflow")
}

pub fn amount_to_principal_up(amount: u128, multiplier: f64) -> u128 {
    // If the index is 1, return the amount directly
    if multiplier == 1.0 {
        return amount;
    }

    // Calculate the principal from the amount and index, rounding up
    let index = (multiplier * INDEX_SCALE_F64).trunc() as u128;

    (amount as u128)
        .checked_mul(INDEX_SCALE_U64 as u128)
        .expect("overflow")
        .checked_add(index.checked_sub(1u128).expect("underflow"))
        .expect("overflow")
        .checked_div(index)
        .expect("underflow")
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

pub fn principal_to_amount_up(principal: u64, multiplier: f64) -> u128 {
    // If the index is 1, return the principal directly
    if multiplier == 1.0 {
        return principal as u128;
    }

    let index = (multiplier * INDEX_SCALE_F64).trunc() as u128;

    // Calculate the amount from the principal and index, rounding up
    (index as u128)
        .checked_mul(principal as u128)
        .expect("overflow")
        .checked_add(
            (INDEX_SCALE_U64 as u128)
                .checked_sub(1u128)
                .expect("underflow"),
        )
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
