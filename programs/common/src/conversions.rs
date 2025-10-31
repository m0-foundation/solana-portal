use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            scaled_ui_amount::ScaledUiAmountConfig, BaseStateWithExtensions, StateWithExtensions,
        },
        state,
    },
    token_interface::Mint,
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

pub fn get_scaled_ui_config<'info>(
    mint: &InterfaceAccount<'info, Mint>,
) -> Result<ScaledUiAmountConfig> {
    // Get the mint account data with extensions
    let account_info = mint.to_account_info();
    let mint_data = account_info.try_borrow_data()?;
    let mint_ext_data = StateWithExtensions::<state::Mint>::unpack(&mint_data)?;

    // Get the scaled UI config extension
    let scaled_ui_config = mint_ext_data.get_extension::<ScaledUiAmountConfig>()?;

    Ok(*scaled_ui_config)
}
