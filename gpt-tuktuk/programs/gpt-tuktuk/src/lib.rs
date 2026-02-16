use anchor_lang::prelude::*;

declare_id!("3LWmo92AxMjU5tLjfneoqCYVMQSoA6teNeYhSiQojpSG");
//solana gpt LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab
//tuktuk devnet LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab

pub mod instructions;
pub mod state;

use instructions::*;

#[program]
pub mod gpt_tuktuk {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize(ctx)
    }

    pub fn schedule_request(
        ctx: Context<ScheduleRequest>,
        task_id: u16,
        prompt: String,
        delay: i64,
    ) -> Result<()> {
        instructions::schedule_request(ctx, task_id, prompt, delay)
    }

    pub fn execute_request(ctx: Context<ExecuteRequest>, prompt: String) -> Result<()> {
        instructions::execute_request(ctx, prompt)
    }

    pub fn consume_result(ctx: Context<ConsumeResult>, result: String) -> Result<()> {
        instructions::consume_result(ctx, result)
    }
}
