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
