use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

declare_id!("GZUKiow4NdgwoWTxXCPUYtfQoWZoWi4vQ18CPDMipyk");

#[program]
pub mod gg_token {
    use super::*;

    // Initialize token with a total supply
    pub fn initialize(ctx: Context<Initialize>, total_supply: u64, name: String, symbol: String) -> Result<()> {
        // Ensure total supply is greater than zero
        require!(total_supply > 0, CustomError::InvalidTotalSupply);
        
        // Ensure token name and symbol are not empty
        require!(!name.is_empty(), CustomError::InvalidTokenName);
        require!(!symbol.is_empty(), CustomError::InvalidTokenSymbol);
        
        // Ensure initialization can only happen once
        require!(ctx.accounts.token_details.total_supply == 0, CustomError::AlreadyInitialized);

        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // Mint the total supply of tokens to the token account
        token::mint_to(cpi_ctx, total_supply)?;
        
        // Store token details in the global state
        let token_details = &mut ctx.accounts.token_details;
        token_details.name = name;
        token_details.symbol = symbol;
        token_details.total_supply = total_supply;
        token_details.mint = ctx.accounts.mint.key();
        token_details.owner = *ctx.accounts.mint_authority.key;
        token_details.initial_offering = 5_000_000;
        token_details.second_offering = 5_000_000;
        token_details.collective_reserves = 11_000_000;
        token_details.development_fund = 9_900_000;
        token_details.reserve = 1_100_000;

        // Set lock period for second offering (1 year)
        token_details.second_offering_unlock_time = Clock::get()?.unix_timestamp + 365 * 24 * 60 * 60;

        Ok(())
    }

    // Lock tokens for a fixed period (specific to second offering)
    pub fn lock_tokens(ctx: Context<LockTokens>, amount: u64) -> Result<()> {
        // Ensure the amount to lock is greater than zero
        require!(amount > 0, CustomError::InvalidLockAmount);
        
        // Ensure the owner has enough tokens to lock
        require!(ctx.accounts.owner_token_account.amount >= amount, CustomError::InsufficientBalance);

        let lock_account = &mut ctx.accounts.lock_account;
        lock_account.amount = amount;
        lock_account.lock_period = 365 * 24 * 60 * 60; // 365 days in seconds
        lock_account.owner = *ctx.accounts.owner.key;
        lock_account.unlock_time = Clock::get()?.unix_timestamp + lock_account.lock_period as i64;

        Ok(())
    }

    // Unlock tokens after the lock period has ended
    pub fn unlock_tokens(ctx: Context<UnlockTokens>) -> Result<()> {
        let lock_account = &mut ctx.accounts.lock_account;

        // Ensure the current time is past the unlock time
        require!(
            Clock::get()?.unix_timestamp >= lock_account.unlock_time,
            CustomError::LockPeriodNotOver
        );

        // Ensure the lock account has a positive balance
        require!(lock_account.amount > 0, CustomError::NoTokensToUnlock);

        // Store the amount to transfer
        let amount = lock_account.amount;

        // Transfer the locked tokens to the destination account
        let cpi_accounts = Transfer {
            from: ctx.accounts.lock_account.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    // Distribute revenue proportionally to token holders
pub fn distribute_revenue(ctx: Context<DistributeRevenue>, amount: u64) -> Result<()> {
    let token_details = &ctx.accounts.token_details;

    // Ensure revenue amount is greater than zero
    require!(amount > 0, CustomError::InvalidRevenueAmount);
    
    // Ensure total supply is greater than zero to avoid division by zero
    require!(token_details.total_supply > 0, CustomError::ZeroTotalSupply);

    let total_supply = token_details.total_supply;

    for holder_info in ctx.remaining_accounts.iter() {
        distribute_to_holder(&ctx, holder_info, total_supply, amount)?;  // Pass &ctx here
    }

    Ok(())
}




    // Create new governance proposal (restricted to Admin)
    pub fn create_proposal(ctx: Context<CreateProposal>, description: String) -> Result<()> {
        // Ensure the proposal description is not empty
        require!(!description.is_empty(), CustomError::InvalidProposalDescription);

        let proposal = &mut ctx.accounts.proposal;
        proposal.description = description;
        proposal.creator = ctx.accounts.creator.key();
        proposal.votes_for = 0;
        proposal.votes_against = 0;
        proposal.voting_deadline = Clock::get()?.unix_timestamp + 7 * 24 * 60 * 60; // 1 week voting period
        proposal.passed = false;

        Ok(())
    }

    // Vote on existing governance proposal (one vote per wallet)
    pub fn vote_on_proposal(ctx: Context<VoteOnProposal>, vote_for: bool) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        // Check that voting is still open
        require!(
            Clock::get()?.unix_timestamp <= proposal.voting_deadline,
            CustomError::VotingPeriodEnded
        );

        // Check that voter hasn't already voted
        let voter = &ctx.accounts.voter;
        require!(
            !proposal.voters.contains(&voter.key()),
            CustomError::AlreadyVoted
        );

        if vote_for {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }

        // Add voter to list of voters
        proposal.voters.push(voter.key());

        // Determine if the proposal passes (quorum and majority needed)
        let total_votes = proposal.votes_for + proposal.votes_against;
        if total_votes > 0 && proposal.votes_for as f64 / total_votes as f64 > 0.5 {
            proposal.passed = true;
        }
        Ok(())
    }

    // Handle the initial and second token sale (without KYC)
    pub fn initial_sale(ctx: Context<Sale>, amount: u64) -> Result<()> {
        // Transfer tokens to the buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.sale_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.sale_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn second_sale(ctx: Context<Sale>, amount: u64) -> Result<()> {
        // Transfer tokens to the buyer (after the 1-year lock period)
        let token_details = &ctx.accounts.token_details;
        require!(
            Clock::get()?.unix_timestamp >= token_details.second_offering_unlock_time,
            CustomError::LockPeriodNotOver
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.sale_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.sale_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    // Add other functions as needed (e.g., staking, burning)
}

// Contexts for function calls

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>, // Token mint account
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>, // Token account to receive the total supply
    #[account(mut)]  
    pub mint_authority: Signer<'info>, // Authority allowed to mint tokens
    #[account(init, payer = mint_authority, space = 8 + 192)]  // Adjust space calculation here
    pub token_details: Account<'info, TokenDetails>, // Token details account
    pub token_program: Program<'info, token::Token>, // Token program
    pub system_program: Program<'info, System>, // System program
}

#[derive(Accounts)]
pub struct LockTokens<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 8 + 8 + 8)] // Adjust space calculation
    pub lock_account: Account<'info, LockAccount>, // Lock account to store locked tokens
    #[account(mut)]
    pub owner: Signer<'info>, // Owner of the locked tokens
    #[account(mut)]
    pub owner_token_account: Account<'info, TokenAccount>, // Owner's token account to check balance
    pub system_program: Program<'info, System>, // System program
    pub token_program: Program<'info, token::Token>, // Token program
}

#[derive(Accounts)]
pub struct UnlockTokens<'info> {
    #[account(mut, has_one = owner)]
    pub lock_account: Account<'info, LockAccount>, // Lock account storing locked tokens
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>, // Destination account to receive unlocked tokens
    pub owner: Signer<'info>, // Owner of the locked tokens
    pub token_program: Program<'info, token::Token>, // Token program
}

#[derive(Accounts)]
pub struct DistributeRevenue<'info> {
    #[account(mut)]
    pub revenue_pool: Account<'info, TokenAccount>, // Account holding the revenue to be distributed
    #[account(mut)]
    pub token_details: Account<'info, TokenDetails>, // Token details account
    pub distributor: Signer<'info>, // Authority allowed to distribute revenue
    pub token_program: Program<'info, token::Token>, // Token program
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(init, payer = creator, space = 8 + 32 + 4 + 64 + 8 + 8 + 8 + 1 + (4 + 32 * 100))] // Adjust space calculation
    pub proposal: Account<'info, Proposal>, // Proposal account
    #[account(mut)]
    pub creator: Signer<'info>, // Creator of the proposal
    pub system_program: Program<'info, System>, // System program
}

#[derive(Accounts)]
pub struct VoteOnProposal<'info> {
    #[account(mut, has_one = creator)]
    pub proposal: Account<'info, Proposal>, // Proposal account
    pub voter: Signer<'info>, // Voter
    pub creator: Signer<'info>, // Creator of the proposal
}

#[derive(Accounts)]
pub struct Sale<'info> {
    #[account(mut)]
    pub sale_account: Account<'info, TokenAccount>, // Account holding the tokens for sale
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>, // Buyer's token account
    pub sale_authority: Signer<'info>, // Authority allowed to sell tokens
    pub token_program: Program<'info, token::Token>, // Token program
    #[account(mut)]  // Include if you need token details in sale
    pub token_details: Account<'info, TokenDetails>, // Token details account
}

// Data Structures

#[account]
pub struct TokenDetails {
    pub name: String,         // Token name
    pub symbol: String,       // Token symbol
    pub total_supply: u64,    // Total supply of tokens
    pub mint: Pubkey,         // Mint account
    pub owner: Pubkey,        // Contract owner
    pub initial_offering: u64, // Initial offering allocation
    pub second_offering: u64, // Second offering allocation
    pub collective_reserves: u64, // Collective reserves
    pub development_fund: u64, // Development fund
    pub reserve: u64,         // Reserve fund
    pub second_offering_unlock_time: i64, // Unlock time for second offering
}

#[account]
pub struct LockAccount {
    pub owner: Pubkey,        // Owner of the lock account
    pub amount: u64,          // Amount of tokens locked
    pub lock_period: u64,     // Lock period in seconds (fixed at 365 days)
    pub unlock_time: i64,     // Timestamp when tokens can be unlocked
}

#[account]
pub struct Proposal {
    pub creator: Pubkey,      // Creator of the proposal
    pub description: String,  // Description of the proposal
    pub votes_for: u64,       // Number of votes in favor
    pub votes_against: u64,   // Number of votes against
    pub voting_deadline: i64, // Voting deadline timestamp
    pub passed: bool,         // Whether the proposal has passed
    pub voters: Vec<Pubkey>,  // List of voters who have voted
}

// Error Handling

#[error_code]
pub enum CustomError {
    #[msg("Lock period is not over yet.")]
    LockPeriodNotOver,
    #[msg("Voting period has ended.")]
    VotingPeriodEnded,
    #[msg("You have already voted on this proposal.")]
    AlreadyVoted,
    #[msg("Invalid total supply.")]
    InvalidTotalSupply,
    #[msg("Invalid token name.")]
    InvalidTokenName,
    #[msg("Invalid token symbol.")]
    InvalidTokenSymbol,
    #[msg("Contract already initialized.")]
    AlreadyInitialized,
    #[msg("Invalid lock amount.")]
    InvalidLockAmount,
    #[msg("Insufficient balance to lock tokens.")]
    InsufficientBalance,
    #[msg("No tokens available to unlock.")]
    NoTokensToUnlock,
    #[msg("Invalid revenue amount.")]
    InvalidRevenueAmount,
    #[msg("Total supply is zero.")]
    ZeroTotalSupply,
    #[msg("Invalid proposal description.")]
    InvalidProposalDescription,
    #[msg("Not authorized to create proposals.")]
    NotAuthorized,
}
