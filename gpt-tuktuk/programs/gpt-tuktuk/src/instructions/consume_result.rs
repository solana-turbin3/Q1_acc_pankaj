use crate::state::GptRequest;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ConsumeResult<'info> {
    #[account(
        mut,
        seeds = [b"gpt_request", user.key().as_ref(), &request_state.task_id.to_le_bytes()],
        bump = request_state.bump,
    )]
    pub request_state: Account<'info, GptRequest>,

    /// The GPT Oracle's Identity PDA that signs the callback
    /// We verify this is owned by the GPT Oracle program
    /// CHECK: Verified as the oracle identity PDA signer
    #[account(
        signer,
        constraint = oracle_identity.owner == &solana_gpt_oracle::ID
    )]
    pub oracle_identity: UncheckedAccount<'info>,

    /// CHECK: User needed fro PDA seed derivation
    pub user: UncheckedAccount<'info>,
}

pub fn consume_result(ctx: Context<ConsumeResult>, result: String) -> Result<()> {
    let request_state = &mut ctx.accounts.request_state;

    msg!("Received GPT Response: {}", result);
    request_state.result = Some(result);
    request_state.is_completed = true;

    Ok(())
}

/*
pub fn callback_from_agent(ctx: Context<CallbackFromAgent>, response: String) -> Result<()> {
    if !ctx.accounts.identity.to_account_info().is_signer {
        return Err(ProgramError::InvalidAccountData.into());
    }
    msg!("Agent Response: {:?}", response);
    Ok(())
}

    we are doing above solana gpt oracle use anchor 0.32.1
    and our program use 0.31.1 which is required by tuk-tuk
*/
