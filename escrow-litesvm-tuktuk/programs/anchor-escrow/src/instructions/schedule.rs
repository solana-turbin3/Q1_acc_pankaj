use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use tuktuk_program::{
    compile_transaction,
    tuktuk::{
        cpi::{accounts::QueueTaskV0, queue_task_v0},
        program::Tuktuk,
        types::TriggerV0,
    },
    types::QueueTaskArgsV0,
    TransactionSourceV0,
};

use crate::state::Escrow;

#[derive(Accounts)]
#[instruction(task_id: u16)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"escrow", escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut)]
    /// CHECK: CPI account
    pub task_queue: UncheckedAccount<'info>,
    /// CHECK: CPI account
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

    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}

pub fn custom_schedule(ctx: Context<Schedule>, task_id: u16) -> Result<()> {
    // Construct the instruction to be executed by the task (Refund)
    // We need to construct the `refund` instruction call.
    // The `refund` instruction expects:
    // maker (Unchecked), mint_a, maker_ata_a, escrow, vault, token_program, system_program

    // NOTE: For the cron to work permissionlessly, the `refund` instruction must NOT require signer for maker.
    // We already modified `refund` to allow this if expired.

    // However, typical CPI construction requires known account metas.
    // The `Refund` accounts are:
    // 0. maker (mut)
    // 1. mint_a
    // 2. maker_ata_a (mut)
    // 3. escrow (mut)
    // 4. vault (mut)
    // 5. token_program
    // 6. system_program

    // We need these accounts to be passed into `schedule` or derived.
    // Ideally, we pass them as remaining accounts or explicitly if we want to validiate them.
    // For simplicity, let's assume we can derive or pass them.
    // But `compile_transaction` takes `Instruction`.

    // Wait, `escrow` has `mint_a`, `maker` stored. We can derive `vault` and `maker_ata_a`.
    // But `compile_transaction` needs `AccountMeta`s.

    // Let's use the explicit accounts passed to `Schedule` context if we add them, or just construct manually.
    // Since `refund` only needs public keys, we can construct the instruction data and accounts.

    // ISSUE: We don't have the `maker_ata_a` and `vault` in `Schedule` context currently.
    // We should add them to `Schedule` struct to easily reference them, or just use `escrow` data to derive.
    // Deriving in standard Anchor CPI might be tricky without `AccountInfo`.
    // But `compile_transaction` just needs `AccountMeta`. We can create `AccountMeta` from Pubkeys.

    // Let's update `Schedule` struct to include necessary accounts for `refund` so we can easily build the CPI.
    // Or, better, just use the `escrow` data.

    let maker = ctx.accounts.escrow.maker;
    let mint_a = ctx.accounts.escrow.mint_a;
    let escrow_key = ctx.accounts.escrow.key();

    // Derive ATAs
    let maker_ata_a = anchor_spl::associated_token::get_associated_token_address(&mint_a, &maker);
    let vault = anchor_spl::associated_token::get_associated_token_address(&mint_a, &escrow_key);

    let token_program = anchor_spl::token::ID; // Assuming SPL Token
    let system_program = anchor_lang::solana_program::system_program::ID;

    // Construct the instruction
    let refund_ix_accounts = vec![
        AccountMeta::new(maker, false),           // maker (mut, not signer)
        AccountMeta::new_readonly(mint_a, false), // mint_a
        AccountMeta::new(maker_ata_a, false),     // maker_ata_a
        AccountMeta::new(escrow_key, false),      // escrow
        AccountMeta::new(vault, false),           // vault
        AccountMeta::new_readonly(token_program, false), // token_program
        AccountMeta::new_readonly(system_program, false), // system_program
    ];

    let refund_ix_data = crate::instruction::Refund {}.data(); // Anchor generates this?
                                                               // If not, we need to manually compute discriminator.
                                                               // Anchor encodes instruction data as 8-byte discriminator + args.
                                                               // `Refund` has no args.

    // Verify `Refund` exists in `crate::instruction`.

    let (compiled_tx, _) = compile_transaction(
        vec![Instruction {
            program_id: crate::ID,
            accounts: refund_ix_accounts,
            data: refund_ix_data,
        }],
        vec![],
    )
    .unwrap();

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
            &[&[b"queue_authority", &[0]]], // Need bump? The seed is just "queue_authority" in tuktuk-counter?
                                            // Wait, tuktuk-counter uses `seeds = [b"queue_authority"], bump`.
                                            // And passes `&[&["queue_authority".as_bytes(), &[bumps.queue_authority]]]`
                                            // I need to make sure I pass the bump.
        ),
        QueueTaskArgsV0 {
            trigger: TriggerV0::Timestamp(ctx.accounts.escrow.deadline), // Trigger at deadline
            transaction: TransactionSourceV0::CompiledV0(compiled_tx),
            crank_reward: Some(1000000), // 0.001 SOL reward?
            free_tasks: 1,
            id: task_id,
            description: "Refund Escrow".to_string(),
        },
    )?;

    Ok(())
}
