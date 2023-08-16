//! GetRemoveLiquidityAmountAndFee instruction handler

use {
    crate::{
        math,
        state::{
            custody::Custody,
            oracle::OraclePrice,
            perpetuals::{AmountAndFee, Perpetuals},
            pool::{AumCalcMode, Pool},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Mint,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct GetRemoveLiquidityAmountAndFee<'info> {
    #[account(
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the token
    #[account(
        constraint = custody_oracle_account.key() == custody.oracle.oracle_account
    )]
    pub custody_oracle_account: AccountInfo<'info>,

    /// CHECK: oracle account for the token
    #[account(
        constraint = custody_custom_oracle_account.key() == custody.oracle.custom_oracle_account
    )]
    pub custody_custom_oracle_account: AccountInfo<'info>,

    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetRemoveLiquidityAmountAndFeeParams {
    lp_amount_in: u64,
}

pub fn get_remove_liquidity_amount_and_fee(
    ctx: Context<GetRemoveLiquidityAmountAndFee>,
    params: &GetRemoveLiquidityAmountAndFeeParams,
) -> Result<AmountAndFee> {
    // validate inputs
    if params.lp_amount_in == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }
    let pool = &ctx.accounts.pool;
    let custody = &ctx.accounts.custody;
    let token_id = pool.get_token_id(&custody.key())?;

    // compute position price
    let curtime = ctx.accounts.perpetuals.get_time()?;

    let (_token_min_price, token_max_price, _) = OraclePrice::new_from_oracle(
        &ctx.accounts.custody_oracle_account.to_account_info(),
        &custody.oracle,
        curtime,
        &ctx.accounts.custody_custom_oracle_account.to_account_info(),
        custody.is_stable
    )?;

    let pool_amount_usd =
        pool.get_assets_under_management_usd(AumCalcMode::Min, ctx.remaining_accounts, curtime, false)?;

    let remove_amount_usd = math::checked_as_u64(math::checked_div(
        math::checked_mul(pool_amount_usd, params.lp_amount_in as u128)?,
        ctx.accounts.lp_token_mint.supply as u128,
    )?)?;

    let remove_amount = token_max_price.get_token_amount(remove_amount_usd, custody.decimals)?;

    let fee_amount =
        pool.get_remove_liquidity_fee(token_id, remove_amount, custody, &token_max_price)?;

    let transfer_amount = math::checked_sub(remove_amount, fee_amount)?;

    Ok(AmountAndFee {
        amount: transfer_amount,
        fee: fee_amount,
    })
}
