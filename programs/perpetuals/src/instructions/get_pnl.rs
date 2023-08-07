//! GetPnl instruction handler

use {
    crate::state::{
        custody::Custody,
        oracle::OraclePrice,
        perpetuals::{Perpetuals, ProfitAndLoss},
        pool::Pool,
        position::Position,
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetPnl<'info> {
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

    #[account(
        constraint = custody_custom_oracle_account.key() == custody.oracle.custom_oracle_account
    )]
    pub custody_custom_oracle_account: AccountInfo<'info>,

    #[account(
        constraint = position.collateral_custody == collateral_custody.key()
    )]
    pub collateral_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = collateral_custody_oracle_account.key() == collateral_custody.oracle.oracle_account
    )]
    pub collateral_custody_oracle_account: AccountInfo<'info>,

    #[account(
        constraint = collateral_custody_custom_oracle_account.key() == collateral_custody.oracle.custom_oracle_account
    )]
    pub collateral_custody_custom_oracle_account: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetPnlParams {}

pub fn get_pnl(ctx: Context<GetPnl>, _params: &GetPnlParams) -> Result<ProfitAndLoss> {
    // get oracle prices
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

    let (collateral_token_min_price, collateral_token_max_price, _) = OraclePrice::new_from_oracle(
        &ctx.accounts
            .collateral_custody_oracle_account
            .to_account_info(),
        &collateral_custody.oracle,
        curtime,
        &ctx.accounts.collateral_custody_custom_oracle_account.to_account_info(),
        collateral_custody.is_stable
    )?;

    // compute pnl
    let (profit, loss, _) = pool.get_pnl_usd(
        position,
        &token_min_price,
        &token_max_price,
        custody,
        &collateral_token_min_price,
        &collateral_token_max_price,
        collateral_custody,
        curtime,
        false,
    )?;

    Ok(ProfitAndLoss { profit, loss })
}
