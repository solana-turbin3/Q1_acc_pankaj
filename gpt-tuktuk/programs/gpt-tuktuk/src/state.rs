use anchor_lang::prelude::*;

/// The GPT Oracle Program ID
pub const GPT_ORACLE_PROGRAM_ID: Pubkey = solana_gpt_oracle::ID_CONST;

#[account]
pub struct GptRequest {
    pub task_id: u16,
    pub prompt: String,
    pub result: Option<String>,
    pub is_completed: bool,
    pub context_account: Pubkey, // The LLM context account on the GPT Oracle
    pub bump: u8,
}

impl GptRequest {
    // 8 discriminator + 2 task_id + (4 + 200) prompt + (1 + 4 + 200) result option
    // + 1 is_completed + 32 context_account + 1 bump
    pub const LEN: usize = 8 + 2 + 4 + 200 + 1 + 4 + 200 + 1 + 32 + 1;
}
