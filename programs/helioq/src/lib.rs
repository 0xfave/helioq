use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("4jpGDc9FUDqn3tEL5e7LXt2ek9TqhDbe8GEeczzvupW3");

#[program]
pub mod helioq {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let admin_account = &mut ctx.accounts.admin_account;
        admin_account.authority = ctx.accounts.authority.key();
        admin_account.reward_pool = 0;
        admin_account.paused = false;
        Ok(())
    }

    pub fn register_server(
        ctx: Context<RegisterServer>,
        server_id: String,
    ) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        require!(server_id.len() <= 32, ErrorCode::ServerIdTooLong);

        let server = &mut ctx.accounts.server;
        let now = Clock::get()?.unix_timestamp;
        
        server.id = server_id;
        server.owner = ctx.accounts.owner.key();
        server.active = true;
        server.registered_at = now;
        server.pending_rewards = 0;
        server.last_metrics_update = 0;
        server.grace_period_end = now + 7 * 24 * 60 * 60; // 7 days grace period
        
        emit!(ServerRegistered {
            server_id: server.id.clone(),
            wallet_address: server.owner,
        });
        
        emit!(GracePeriodStarted {
            server_id: server.id.clone(),
            end_timestamp: server.grace_period_end,
        });
        
        Ok(())
    }

    pub fn submit_metrics(
        ctx: Context<SubmitMetrics>,
        uptime: u8,
        tasks_completed: u64,
        points: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        require!(uptime <= 100, ErrorCode::InvalidUptime);
        
        let server = &mut ctx.accounts.server;
        let now = Clock::get()?.unix_timestamp;
        
        server.last_metrics_update = now;
        server.pending_rewards = server.pending_rewards.checked_add(points)
            .ok_or(ErrorCode::NumericOverflow)?;

        emit!(MetricsUpdated {
            server_id: server.id.clone(),
            points,
        });
        
        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        
        let server = &mut ctx.accounts.server;
        let admin_account = &mut ctx.accounts.admin_account;
        let now = Clock::get()?.unix_timestamp;
        
        require!(
            now.checked_sub(server.last_metrics_update).unwrap() >= 7 * 24 * 60 * 60,
            ErrorCode::ClaimCooldownActive
        );
        
        let rewards = server.pending_rewards;
        require!(rewards <= admin_account.reward_pool, ErrorCode::InsufficientRewardPool);

        // Transfer SOL from program to server owner
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: admin_account.to_account_info(),
                to: ctx.accounts.owner.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, rewards)?;
        
        server.pending_rewards = 0;
        admin_account.reward_pool = admin_account.reward_pool.checked_sub(rewards)
            .ok_or(ErrorCode::NumericOverflow)?;
        
        emit!(RewardsClaimed {
            wallet_address: ctx.accounts.owner.key(),
            reward_amount: rewards,
        });
        
        Ok(())
    }

    pub fn deposit_rewards(ctx: Context<DepositRewards>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        
        // Transfer SOL from authority to program
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.admin_account.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, amount)?;

        let admin_account = &mut ctx.accounts.admin_account;
        admin_account.reward_pool = admin_account.reward_pool
            .checked_add(amount)
            .ok_or(ErrorCode::NumericOverflow)?;
        
        emit!(RewardsDeposited {
            amount,
            new_balance: admin_account.reward_pool,
        });
        
        Ok(())
    }

    pub fn reclaim_stale_rewards(ctx: Context<ReclaimRewards>) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        
        let server = &mut ctx.accounts.server;
        let now = Clock::get()?.unix_timestamp;
        
        require!(
            now.checked_sub(server.last_metrics_update).unwrap() >= 365 * 24 * 60 * 60,
            ErrorCode::RewardsNotStale
        );
        
        let reclaimed_amount = server.pending_rewards;
        server.pending_rewards = 0;
        
        emit!(RewardsReclaimed {
            server_id: server.id.clone(),
            amount: reclaimed_amount
        });
        
        Ok(())
    }

    pub fn deactivate_server(ctx: Context<DeactivateServer>) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        
        let server = &mut ctx.accounts.server;
        server.active = false;
        
        emit!(ServerDeregistered {
            server_id: server.id.clone()
        });
        
        Ok(())
    }

    pub fn reassign_server(ctx: Context<ReassignServer>) -> Result<()> {
        require!(!ctx.accounts.admin_account.paused, ErrorCode::ProgramPaused);
        require!(ctx.accounts.server.active, ErrorCode::ServerNotActive);
        
        let server = &mut ctx.accounts.server;
        let old_owner = server.owner;
        server.owner = ctx.accounts.new_owner.key();
        
        emit!(ServerReassigned {
            server_id: server.id.clone(),
            old_owner,
            new_owner: server.owner,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + AdminAccount::LEN)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterServer<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(init, payer = authority, space = 8 + Server::LEN)]
    pub server: Account<'info, Server>,
    pub owner: SystemAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitMetrics<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub server: Account<'info, Server>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut, has_one = owner)]
    pub server: Account<'info, Server>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositRewards<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReclaimRewards<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub server: Account<'info, Server>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeactivateServer<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub server: Account<'info, Server>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ReassignServer<'info> {
    #[account(mut, has_one = authority)]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub server: Account<'info, Server>,
    pub new_owner: SystemAccount<'info>,
    pub authority: Signer<'info>,
}

#[account]
pub struct AdminAccount {
    pub authority: Pubkey,
    pub reward_pool: u64,
    pub paused: bool,
}

#[account]
pub struct Server {
    pub id: String,
    pub owner: Pubkey,
    pub active: bool,
    pub registered_at: i64,
    pub pending_rewards: u64,
    pub last_metrics_update: i64,
    pub grace_period_end: i64,
}

impl AdminAccount {
    pub const LEN: usize = 32 + 8 + 1;
}

impl Server {
    pub const LEN: usize = 32 + 32 + 1 + 8 + 8 + 8 + 8;
}

#[event]
pub struct GracePeriodStarted {
    pub server_id: String,
    pub end_timestamp: i64,
}

#[event]
pub struct RewardsDeposited {
    pub amount: u64,
    pub new_balance: u64,
}

#[event]
pub struct RewardsReclaimed {
    pub server_id: String,
    pub amount: u64,
}

#[event]
pub struct ServerRegistered {
    pub server_id: String,
    pub wallet_address: Pubkey,
}

#[event]
pub struct ServerDeregistered {
    pub server_id: String,
}

#[event]
pub struct RewardsClaimed {
    pub wallet_address: Pubkey,
    pub reward_amount: u64,
}

#[event]
pub struct MetricsUpdated {
    pub server_id: String,
    pub points: u64,
}

#[event]
pub struct ServerReassigned {
    pub server_id: String,
    pub old_owner: Pubkey,
    pub new_owner: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Program is paused")]
    ProgramPaused,
    #[msg("Server ID too long")]
    ServerIdTooLong,
    #[msg("Invalid uptime percentage")]
    InvalidUptime,
    #[msg("Numeric overflow")]
    NumericOverflow,
    #[msg("Claim cooldown period active")]
    ClaimCooldownActive,
    #[msg("Server is not active")]
    ServerNotActive,
    #[msg("Rewards are not stale yet")]
    RewardsNotStale,
    #[msg("Insufficient reward pool balance")]
    InsufficientRewardPool,
}
