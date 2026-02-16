#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;

mod instructions;
mod state;
mod tests;

use instructions::*;

declare_id!("FircrADQ2wgGuvpm8qneNCfKM7o5zoHTWnDQxngpTQ3J");

#[program]
pub mod anchor_escrow {
    use super::*;

    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        expiry: i64,
    ) -> Result<()> {
        ctx.accounts
            .init_escrow(seed, receive, expiry, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit()?;
        ctx.accounts.withdraw_and_close_vault()
    }

    pub fn schedule(ctx: Context<Schedule>, task_id: u16) -> Result<()> {
        instructions::schedule::custom_schedule(ctx, task_id)
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Escrow has not expired.")]
    EscrowNotExpired,
}
