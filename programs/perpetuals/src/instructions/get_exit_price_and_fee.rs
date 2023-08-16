//! GetExitPriceAndFee instruction handler

use {
    crate::state::{
        custody::Custody,
        oracle::OraclePrice,
        perpetuals::{Perpetuals, PriceAndFee},
        pool::Pool,
        position::{Position, Side},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetExitPriceAndFee<'info> {
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
        seeds = [b"position",
                 position.owner.as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.side as u8]],
        bump = position.bump
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = custody_oracle_account.key() == custody.oracle.oracle_account
    )]
    pub custody_oracle_account: AccountInfo<'info>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = custody_custom_oracle_account.key() == custody.oracle.custom_oracle_account
    )]
    pub custody_custom_oracle_account: AccountInfo<'info>,

    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 collateral_custody.mint.as_ref()],
        bump = collateral_custody.bump
    )]
    pub collateral_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = collateral_custody_oracle_account.key() == collateral_custody.oracle.oracle_account
    )]
    pub collateral_custody_oracle_account: AccountInfo<'info>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = collateral_custody_custom_oracle_account.key() == collateral_custody.oracle.custom_oracle_account
    )]
    pub collateral_custody_custom_oracle_account: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetExitPriceAndFeeParams {}

pub fn get_exit_price_and_fee(
    ctx: Context<GetExitPriceAndFee>,
    _params: &GetExitPriceAndFeeParams,
) -> Result<PriceAndFee> {
    // compute exit price and fee
    let position = &ctx.accounts.position;
    let pool = &ctx.accounts.pool;
    let curtime = ctx.accounts.perpetuals.get_time()?;
    let custody = &ctx.accounts.custody;
    let collateral_custody = &ctx.accounts.collateral_custody;

    let (token_min_price, token_max_price, _) = OraclePrice::new_from_oracle(
        &ctx.accounts.custody_oracle_account.to_account_info(),
        &custody.oracle,
        curtime,
        &ctx.accounts.custody_custom_oracle_account.to_account_info(),
        custody.is_stable
    )?;

    let (collateral_token_min_price, _, _) = OraclePrice::new_from_oracle(
        &ctx.accounts
            .collateral_custody_oracle_account
            .to_account_info(),
        &collateral_custody.oracle,
        curtime,
        &ctx.accounts.collateral_custody_custom_oracle_account.to_account_info(),
        collateral_custody.is_stable
    )?;

    let price = pool.get_exit_price(&token_min_price, &token_max_price, position.side, custody)?;

    let mut fee = pool.get_exit_fee(position.size_usd, custody)?;

    if position.side == Side::Short || custody.is_virtual {
        let fee_amount_usd = token_max_price.get_asset_amount_usd(fee, custody.decimals)?;
        fee = collateral_token_min_price
            .get_token_amount(fee_amount_usd, collateral_custody.decimals)?;
    }

    Ok(PriceAndFee { price, fee })
}
