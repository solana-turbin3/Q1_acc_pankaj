use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{prelude::*, InstructionData};

use tuktuk_program::{
    compile_transaction,
    tuktuk::{
        cpi::{accounts::QueueTaskV0, queue_task_v0},
        program::Tuktuk,
        types::{QueueTaskArgsV0, TriggerV0},
    },
    TransactionSourceV0,
};

use crate::state::GptRequest;

#[derive(Accounts)]
#[instruction(task_id: u16)]
pub struct ScheduleRequest<'info> {
    #[account(
        init,
        payer = user,
        space = GptRequest::LEN,
        seeds = [b"gpt_request", user.key().as_ref(), &task_id.to_le_bytes()],
        bump
    )]
    pub request_state: Account<'info, GptRequest>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Safe
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,

    /// CHECK: Safe
    #[account(mut)]
    pub task_queue_authority: UncheckedAccount<'info>,

    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,

    /// CHECK: Via seeds
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: UncheckedAccount<'info>,

    /// The GPT Oracle context account (must be created beforehand via create_llm_context)
    /// CHECK: Passed through to execute_request
    pub oracle_context_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}

pub fn schedule_request(
    ctx: Context<ScheduleRequest>,
    task_id: u16,
    prompt: String,
    delay: i64, // Delay in seconds
) -> Result<()> {
    // 1. Initialize State
    let request_state = &mut ctx.accounts.request_state;
    request_state.task_id = task_id;
    request_state.prompt = prompt.clone();
    request_state.is_completed = false;
    request_state.context_account = ctx.accounts.oracle_context_account.key();
    request_state.bump = ctx.bumps.request_state;

    // 2. Derive the interaction PDA on the GPT Oracle
    // seeds: [b"interaction", user, context_account]
    let (oracle_interaction, _) = Pubkey::find_program_address(
        &[
            b"interaction",
            ctx.accounts.user.key().as_ref(),
            ctx.accounts.oracle_context_account.key().as_ref(),
        ],
        &solana_gpt_oracle::ID,
    );

    // 3. Prepare Transaction
    // The scheduled task will ( call execute_request ) on this program
    let (compiled_tx, _) = compile_transaction(
        vec![Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(request_state.key(), false),
                AccountMeta::new(ctx.accounts.user.key(), true),
                AccountMeta::new_readonly(ctx.accounts.oracle_context_account.key(), false),
                AccountMeta::new(oracle_interaction, false),
                AccountMeta::new_readonly(solana_gpt_oracle::ID, false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            ],
            data: crate::instruction::ExecuteRequest {
                prompt: prompt.clone(),
            }
            .data(),
        }],
        vec![],
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // 4. Schedule with Tuktuk
    let clock = Clock::get()?;
    let trigger_time = clock.unix_timestamp + delay;

    queue_task_v0(
        CpiContext::new_with_signer(
            ctx.accounts.tuktuk_program.to_account_info(),
            QueueTaskV0 {
                payer: ctx.accounts.user.to_account_info(),
                queue_authority: ctx.accounts.queue_authority.to_account_info(),
                task_queue: ctx.accounts.task_queue.to_account_info(),
                task_queue_authority: ctx.accounts.task_queue_authority.to_account_info(),
                task: ctx.accounts.task.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            &[&[b"queue_authority", &[ctx.bumps.queue_authority]]],
        ),
        QueueTaskArgsV0 {
            trigger: TriggerV0::Timestamp(trigger_time),
            transaction: TransactionSourceV0::CompiledV0(compiled_tx),
            crank_reward: Some(1000000),
            free_tasks: 0,
            id: task_id,
            description: "GPT Oracle Request".to_string(),
        },
    )?;

    msg!("Scheduled GPT Request Task #{}", task_id);
    Ok(())
}
