use crate::state::GptRequest;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;

/// GPT Oracle Program ID
const GPT_ORACLE_PROGRAM: Pubkey = solana_gpt_oracle::ID_CONST;

#[derive(Accounts)]
pub struct ExecuteRequest<'info> {
    #[account(
        mut,
        seeds = [b"gpt_request", user.key().as_ref(), &request_state.task_id.to_le_bytes()],
        bump = request_state.bump,
    )]
    pub request_state: Account<'info, GptRequest>,

    /// CHECK: The user who scheduled this request
    #[account(mut)]
    pub user: Signer<'info>,

    /// The GPT Oracle context account (created via create_llm_context on the oracle)
    /// CHECK: Validated by the GPT Oracle program
    pub oracle_context_account: UncheckedAccount<'info>,

    /// The interaction PDA on the GPT Oracle
    /// CHECK: Derived and validated by the GPT Oracle program
    #[account(mut)]
    pub oracle_interaction: UncheckedAccount<'info>,

    /// The GPT Oracle program
    /// CHECK: Address validated against known program ID
    #[account(address = GPT_ORACLE_PROGRAM)]
    pub gpt_oracle_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn execute_request(ctx: Context<ExecuteRequest>, prompt: String) -> Result<()> {
    let request_state = &mut ctx.accounts.request_state;

    msg!(
        "Executing GPT Request for Task ID: {}",
        request_state.task_id
    );
    msg!("Prompt: {}", prompt);

    // Store the context account reference
    request_state.context_account = ctx.accounts.oracle_context_account.key();

    // Build the callback discriminator for our `consume_result` instruction
    // Anchor discriminator = first 8 bytes of sha256("global:consume_result")
    let callback_discriminator: [u8; 8] =
        anchor_lang::solana_program::hash::hash(b"global:consume_result").to_bytes()[..8]
            .try_into()
            .unwrap();

    // Build callback account metas so the oracle knows which accounts
    // to pass when it CPIs back into our consume_result.
    // The oracle automatically prepends its Identity PDA as the first account (signer),
    // so we only specify our additional accounts here.
    let callback_account_metas: Vec<solana_gpt_oracle::AccountMeta> = vec![
        solana_gpt_oracle::AccountMeta {
            pubkey: request_state.key(),
            is_signer: false,
            is_writable: true,
        },
        solana_gpt_oracle::AccountMeta {
            pubkey: ctx.accounts.user.key(),
            is_signer: false,
            is_writable: false,
        },
    ];

    // Manually build the interact_with_llm instruction data
    // Anchor discriminator for "interact_with_llm" + borsh-serialized args
    let discriminator = anchor_lang::solana_program::hash::hash(b"global:interact_with_llm")
        .to_bytes()[..8]
        .to_vec();

    // Borsh-serialize the arguments manually:
    // text: String, callback_program_id: Pubkey, callback_discriminator: [u8; 8],
    // account_metas: Option<Vec<AccountMeta>>
    let mut ix_data = discriminator;

    // text (String = u32 len + bytes) the prompt Borsh-serialize the prompt string
    ix_data.extend_from_slice(&(prompt.len() as u32).to_le_bytes());
    ix_data.extend_from_slice(prompt.as_bytes());

    // callback_program_id (Pubkey = 32 bytes) Tell oracle which to callback
    ix_data.extend_from_slice(&crate::ID.to_bytes());

    // callback_discriminator ([u8; 8])
    ix_data.extend_from_slice(&callback_discriminator);

    // account_metas (Option<Vec<AccountMeta>> = 1 byte option tag + u32 len + each meta)
    ix_data.push(1u8); // Some
    ix_data.extend_from_slice(&(callback_account_metas.len() as u32).to_le_bytes());
    for meta in &callback_account_metas {
        ix_data.extend_from_slice(&meta.pubkey.to_bytes());
        ix_data.push(meta.is_signer as u8);
        ix_data.push(meta.is_writable as u8);
    }

    // CPI to the GPT Oracle's interact_with_llm instruction
    let ix = Instruction {
        program_id: ctx.accounts.gpt_oracle_program.key(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.user.key(),
                true,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new(
                ctx.accounts.oracle_interaction.key(),
                false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.oracle_context_account.key(),
                false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                ctx.accounts.system_program.key(),
                false,
            ),
        ],
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke(
        &ix,
        &[
            ctx.accounts.user.to_account_info(),
            ctx.accounts.oracle_interaction.to_account_info(),
            ctx.accounts.oracle_context_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("GPT Oracle request sent successfully!");
    Ok(())
}
