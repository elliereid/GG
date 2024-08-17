use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

declare_id!("GZUKiow4NdgwoWTxXCPUYtfQoWZoWi4vQ18CPDMipyk");

#[program]
pub mod gg_token {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, total_supply: u64, name: String, symbol: String) -> Result<()> {
        require!(total_supply > 0, CustomError::InvalidTotalSupply);
        require!(!name.is_empty(), CustomError::InvalidTokenName);
        require!(!symbol.is_empty(), CustomError::InvalidTokenSymbol);
        require!(ctx.accounts.token_details.total_supply == 0, CustomError::AlreadyInitialized);

        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::mint_to(cpi_ctx, total_supply)?;
        
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
        token_details.second_offering_unlock_time = Clock::get()?.unix_timestamp + 365 * 24 * 60 * 60;

        Ok(())
    }

    pub fn lock_tokens(ctx: Context<LockTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, CustomError::InvalidLockAmount);
        require!(ctx.accounts.owner_token_account.amount >= amount, CustomError::InsufficientBalance);

        let lock_account = &mut ctx.accounts.lock_account;
        lock_account.amount = amount;
        lock_account.lock_period = 365 * 24 * 60 * 60;
        lock_account.owner = *ctx.accounts.owner.key;
        lock_account.unlock_time = Clock::get()?.unix_timestamp + lock_account.lock_period as i64;

        Ok(())
    }

    pub fn unlock_tokens(ctx: Context<UnlockTokens>) -> Result<()> {
        let lock_account = &mut ctx.accounts.lock_account;

        require!(Clock::get()?.unix_timestamp >= lock_account.unlock_time, CustomError::LockPeriodNotOver);
        require!(lock_account.amount > 0, CustomError::NoTokensToUnlock);

        let amount = lock_account.amount;

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

    pub fn distribute_revenue(ctx: Context<DistributeRevenue>, amount: u64) -> Result<()> {
        let token_details = &ctx.accounts.token_details;
        require!(amount > 0, CustomError::InvalidRevenueAmount);
        require!(token_details.total_supply > 0, CustomError::ZeroTotalSupply);

        let total_supply = token_details.total_supply;

        for holder_info in ctx.remaining_accounts.iter() {
            distribute_to_holder(&ctx, holder_info, total_supply, amount)?;
        }

        Ok(())
    }

    fn distribute_to_holder<'info>(
        ctx: &Context<DistributeRevenue<'info>>,
        holder_info: &AccountInfo<'info>,
        total_supply: u64,
        amount: u64,
    ) -> Result<()> {
        let holder_account = Account::<TokenAccount>::try_from(holder_info)?;
        let holder_share = amount * holder_account.amount / total_supply;

        let cpi_accounts = Transfer {
            from: ctx.accounts.revenue_pool.to_account_info(),
            to: holder_info.clone(),
            authority: ctx.accounts.distributor.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, holder_share)?;
        Ok(())
    }

    pub fn create_proposal(ctx: Context<CreateProposal>, description: String) -> Result<()> {
        require!(!description.is_empty(), CustomError::InvalidProposalDescription);

        let proposal = &mut ctx.accounts.proposal;
        proposal.description = description;
        proposal.creator = ctx.accounts.creator.key();
        proposal.votes_for = 0;
        proposal.votes_against = 0;
        proposal.voting_deadline = Clock::get()?.unix_timestamp + 7 * 24 * 60 * 60;
        proposal.passed = false;

        Ok(())
    }

    pub fn vote_on_proposal(ctx: Context<VoteOnProposal>, vote_for: bool) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        require!(Clock::get()?.unix_timestamp <= proposal.voting_deadline, CustomError::VotingPeriodEnded);

        let voter = &ctx.accounts.voter;
        require!(!proposal.voters.contains(&voter.key()), CustomError::AlreadyVoted);

        if vote_for {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }

        proposal.voters.push(voter.key());

        let total_votes = proposal.votes_for + proposal.votes_against;
        if total_votes > 0 && proposal.votes_for as f64 / total_votes as f64 > 0.5 {
            proposal.passed = true;
        }
        Ok(())
    }

    pub fn initial_sale(ctx: Context<Sale>, amount: u64) -> Result<()> {
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
        let token_details = &ctx.accounts.token_details;
        require!(Clock::get()?.unix_timestamp >= token_details.second_offering_unlock_time, CustomError::LockPeriodNotOver);

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
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]  
    pub mint_authority: Signer<'info>,
    #[account(init, payer = mint_authority, space = 8 + 192)]
    pub token_details: Account<'info, TokenDetails>,
    pub token_program: Program<'info, token::Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LockTokens<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 8 + 8 + 8)]
    pub lock_account: Account<'info, LockAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub owner_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, token::Token>,
}

#[derive(Accounts)]
pub struct UnlockTokens<'info> {
    #[account(mut, has_one = owner)]
    pub lock_account: Account<'info, LockAccount>,
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, token::Token>,
}

#[derive(Accounts)]
pub struct DistributeRevenue<'info> {
    #[account(mut)]
    pub revenue_pool: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_details: Account<'info, TokenDetails>,
    pub distributor: Signer<'info>,
    pub token_program: Program<'info, token::Token>,
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(init, payer = creator, space = 8 + 32 + 4 + 64 + 8 + 8 + 8 + 1 + (4 + 32 * 100))]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VoteOnProposal<'info> {
    #[account(mut, has_one = creator)]
    pub proposal: Account<'info, Proposal>,
    pub voter: Signer<'info>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct Sale<'info> {
    #[account(mut)]
    pub sale_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub sale_authority: Signer<'info>,
    pub token_program: Program<'info, token::Token>,
    #[account(mut)]
    pub token_details: Account<'info, TokenDetails>,
}

#[account]
pub struct TokenDetails {
    pub name: String,
    pub symbol: String,
    pub total_supply: u64,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub initial_offering: u64,
    pub second_offering: u64,
    pub collective_reserves: u64,
    pub development_fund: u64,
    pub reserve: u64,
    pub second_offering_unlock_time: i64,
}

#[account]
pub struct LockAccount {
    pub owner: Pubkey,
    pub amount: u64,
    pub lock_period: u64,
    pub unlock_time: i64,
}

#[account]
pub struct Proposal {
    pub creator: Pubkey,
    pub description: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub voting_deadline: i64,
    pub passed: bool,
    pub voters: Vec<Pubkey>,
}

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
