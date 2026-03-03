use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{BurnV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder},
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};
use crate::state::Config;
use crate::errors::StakingError;

// Constant for time calculations
const SECONDS_PER_DAY: i64 = 86400;
// Bonus points for burning the NFT
const BURN_BONUS_MULTIPLIER: u64 = 2;

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut, 
        seeds = [b"rewards", config.key().as_ref()],
        bump = config.rewards_bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = rewards_mint,
        associated_token::authority = user,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: NFT account will be checked by the mpl core program
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account will be checked by the mpl core program
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: This is the ID of the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> BurnStakedNft<'info> {
    pub fn burn_staked_nft(&mut self, bumps: &BurnStakedNftBumps) -> Result<()> {
        
        // Verify NFT owner and update authority
        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.user.key(), StakingError::InvalidOwner);
        require!(base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()), StakingError::InvalidAuthority);
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        // Signer seeds for the update authority
        let collection_key = self.collection.key();
        let signer_seeds = &[
            b"update_authority",
            collection_key.as_ref(),
            &[bumps.update_authority],
        ];

        // Get current timestamp
        let current_timestamp = Clock::get()?.unix_timestamp;

        // Check if the NFT has the attribute plugin
        let fetched_attribute_list = match fetch_plugin::<BaseAssetV1, Attributes>(&self.nft.to_account_info(), PluginType::Attributes) {
            Err(_) => {
                return Err(StakingError::NotStaked.into());
            }
            Ok((_, attributes, _)) => attributes,
        };

        // Extract and validate staking attributes
        let mut staked_value: Option<&str> = None;
        let mut staked_at_value: Option<&str> = None;
        
        for attribute in &fetched_attribute_list.attribute_list {
            match attribute.key.as_str() {
                "staked" => {
                    staked_value = Some(&attribute.value);
                }
                "staked_at" => {
                    staked_at_value = Some(&attribute.value);
                }
                _ => {}
            }
        }

        require!(staked_value == Some("true"), StakingError::NotStaked);
        
        let staked_at_timestamp = staked_at_value
            .ok_or(StakingError::InvalidTimestamp)?
            .parse::<i64>()
            .map_err(|_| StakingError::InvalidTimestamp)?;

        // Calculate staked time in days
        let elapsed_seconds = current_timestamp
            .checked_sub(staked_at_timestamp)
            .ok_or(StakingError::InvalidTimestamp)?;
        
        let staked_time_days = elapsed_seconds
            .checked_div(SECONDS_PER_DAY)
            .ok_or(StakingError::InvalidTimestamp)?;

        require!(staked_time_days > 0, StakingError::FreezePeriodNotElapsed);
        require!(staked_time_days >= self.config.freeze_period as i64, StakingError::FreezePeriodNotElapsed);

        // Fetch and update Collection Attributes for `total_staked`
        let (_, mut collection_attributes, _) = fetch_plugin::<BaseCollectionV1, Attributes>(
            &self.collection.to_account_info(),
            PluginType::Attributes,
        )?;

        // Decrement `total_staked` on the collection
        let mut found_total_staked = false;
        for attribute in &mut collection_attributes.attribute_list {
            if attribute.key == "total_staked" {
                let current_staked: u32 = attribute.value.parse().unwrap_or(0);
                attribute.value = current_staked.saturating_sub(1).to_string();
                found_total_staked = true;
                break;
            }
        }
        
        if !found_total_staked {
            collection_attributes.attribute_list.push(Attribute {
                key: "total_staked".to_string(),
                value: "0".to_string(),
            });
        }

        UpdateCollectionPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.user.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::Attributes(collection_attributes))
            .invoke_signed(&[signer_seeds])?;

        // Unfreeze the NFT before burning
        mpl_core::instructions::UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(mpl_core::types::FreezeDelegate { frozen: false }))
            .invoke_signed(&[signer_seeds])?;

        // Burn the asset!
        // We use BurnV1CpiBuilder to burn the asset.
        // The update authority is the delegate for the burn since we added BurnDelegate earlier.
        BurnV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(&[signer_seeds])?;

        // Calculate rewards to the user
        let base_amount = (staked_time_days as u64)
            .checked_mul(self.config.points_per_stake as u64)
            .ok_or(StakingError::Overflow)?;

        // Apply bonus for burning
        let amount = base_amount.checked_mul(BURN_BONUS_MULTIPLIER)
            .ok_or(StakingError::Overflow)?;

        // Prepare signer seeds for config PDA
        let config_seeds = &[
            b"config",
            collection_key.as_ref(),
            &[self.config.config_bump],
        ];
        let config_signer_seeds = &[&config_seeds[..]];
        
        // Mint rewards tokens to user's ATA
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintToChecked {
            mint: self.rewards_mint.to_account_info(),
            to: self.user_rewards_ata.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, config_signer_seeds);
        mint_to_checked(cpi_ctx, amount, self.rewards_mint.decimals)?;
        
        Ok(())
    }
}
