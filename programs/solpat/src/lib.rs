use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use pyth_client::load_price;

declare_id!("PrediCRGdC1S7TwJWrLbZtcv3qLm3AeQHZ8c4vFpWd9");

#[program]
pub mod solpat {
    use super::*;
    pub fn create_pool(
        ctx: Context<CreatePool>,
        pool_id: u64,
        duration: i64,
        fee_rate: u64,
    ) -> ProgramResult {
        let pool = &mut ctx.accounts.pool;
        pool.pool_id = pool_id;
        pool.authority = ctx.accounts.authority.key();
        pool.token_program = ctx.accounts.token_program.key();
        pool.token_mint = ctx.accounts.token_mint.key();
        pool.feed_account = ctx.accounts.feed_account.key();
        pool.duration = duration;
        pool.fee_rate = fee_rate;
        pool.next_round = 2;
        pool.latest_time = ctx.accounts.clock.unix_timestamp;
        Ok(())
    }

    pub fn start_round(ctx: Context<StartRound>) -> ProgramResult {
        let now_ts = ctx.accounts.clock.unix_timestamp;
        let next_round = &mut ctx.accounts.next_round;
        let pool = &mut ctx.accounts.pool;
        // start new round
        next_round.start_time = now_ts;
        next_round.deposit_up = 0;
        next_round.deposit_down = 0;
        next_round.take_amount = 0;
        next_round.status = 0;
        pool.next_round += 1;
        pool.latest_time = now_ts;
        emit!(DidStartRound {
            start_time: now_ts,
            round_id: pool.next_round - 1,
        });
        Ok(())
    }

    pub fn lock_round(ctx: Context<LockRound>) -> ProgramResult {
        let price = load_price(&ctx.accounts.feed_account.try_borrow_data()?)
            .unwrap()
            .get_current_price()
            .unwrap()
            .price;
        let now_ts = ctx.accounts.clock.unix_timestamp;
        let cur_round = &mut ctx.accounts.cur_round;
        let next_round = &mut ctx.accounts.next_round;
        let pool = &mut ctx.accounts.pool;
        // lock cur round
        cur_round.status = 1;
        cur_round.lock_time = now_ts;
        cur_round.lock_price = price;
        // start new round
        next_round.start_time = now_ts;
        next_round.deposit_up = 0;
        next_round.deposit_down = 0;
        next_round.take_amount = 0;
        next_round.status = 0;
        pool.next_round += 1;
        pool.latest_time = now_ts;
        emit!(DidLockRound {
            lock_time: now_ts,
            lock_price: price,
            round_id: pool.next_round - 1,
        });
        Ok(())
    }

    pub fn process_round(ctx: Context<ProcessRound>) -> ProgramResult {
        let price = load_price(&ctx.accounts.feed_account.try_borrow_data()?)
            .unwrap()
            .get_current_price()
            .unwrap()
            .price;
        let now_ts = ctx.accounts.clock.unix_timestamp;
        let pre_round = &mut ctx.accounts.pre_round;
        let cur_round = &mut ctx.accounts.cur_round;
        let next_round = &mut ctx.accounts.next_round;
        let pool = &mut ctx.accounts.pool;
        // close pre round
        pre_round.status = 2;
        pre_round.closed_price = price;
        // lock cur round
        cur_round.status = 1;
        cur_round.lock_time = now_ts;
        cur_round.lock_price = price;
        // start new round
        next_round.start_time = now_ts;
        next_round.deposit_up = 0;
        next_round.deposit_down = 0;
        next_round.take_amount = 0;
        next_round.status = 0;
        pool.next_round += 1;
        pool.latest_time = now_ts;
        emit!(DidProcessRound {
            lock_time: now_ts,
            lock_price: price,
            round_id: pool.next_round - 1,
        });
        Ok(())
    }

    pub fn pause_round(ctx: Context<PauseRound>) -> ProgramResult {
        let price = load_price(&ctx.accounts.feed_account.try_borrow_data()?)
            .unwrap()
            .get_current_price()
            .unwrap()
            .price;
        let now_ts = ctx.accounts.clock.unix_timestamp;
        let pre_round = &mut ctx.accounts.pre_round;
        let cur_round = &mut ctx.accounts.cur_round;
        // close pre round
        pre_round.status = 2;
        pre_round.closed_price = price;
        // lock cur round
        cur_round.status = 1;
        cur_round.lock_time = now_ts;
        cur_round.lock_price = price;
        Ok(())
    }

    pub fn close_round(ctx: Context<CloseRound>) -> ProgramResult {
        let price = load_price(&ctx.accounts.feed_account.try_borrow_data()?)
            .unwrap()
            .get_current_price()
            .unwrap()
            .price;
        let cur_round = &mut ctx.accounts.cur_round;
        // close pre round
        cur_round.status = 2;
        cur_round.closed_price = price;
        Ok(())
    }

    pub fn bet(ctx: Context<Bet>, bet_amount: u64, round_id: u64, bet_type: u8) -> ProgramResult {
        let cur_round = &mut ctx.accounts.cur_round;
        let user_bet = &mut ctx.accounts.user_bet;
        if bet_type == 0 {
            cur_round.deposit_down += bet_amount;
            user_bet.bet_down += bet_amount;
        } else {
            cur_round.deposit_up += bet_amount;
            user_bet.bet_up += bet_amount;
        }
        user_bet.bet_time = ctx.accounts.clock.unix_timestamp;
        user_bet.is_active = true;
        token::transfer(ctx.accounts.into_transfer_context(), bet_amount)?;
        emit!(DidBet {
            round_id,
            user_pubkey: ctx.accounts.authority.key(),
            bet_amount,
            bet_type,
        });
        Ok(())
    }

    pub fn claim(ctx: Context<Claim>, round_id: u64) -> ProgramResult {
        let cur_round = &mut ctx.accounts.cur_round;
        let user_bet = &mut ctx.accounts.user_bet;
        let bonus = (cur_round.deposit_down + cur_round.deposit_up)
            * (10000 - ctx.accounts.pool.fee_rate)
            / 10000;
        let amount = if cur_round.closed_price > cur_round.lock_price {
            if cur_round.deposit_up > 0 {
                let amount = bonus as u128 * user_bet.bet_up as u128 / cur_round.deposit_up as u128;
                amount as u64
            } else {
                0
            }
        } else {
            if cur_round.deposit_down > 0 {
                let amount =
                    bonus as u128 * user_bet.bet_down as u128 / cur_round.deposit_down as u128;
                amount as u64
            } else {
                0
            }
        };
        user_bet.is_active = false;
        cur_round.take_amount += amount;
        if amount > 0 {
            let pool_id_bytes = ctx.accounts.pool.pool_id.to_be_bytes();
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[pool_id_bytes.as_ref()], ctx.program_id);
            let authority_seeds = [pool_id_bytes.as_ref(), &[vault_authority_bump]];
            token::transfer(
                ctx.accounts
                    .into_transfer_context()
                    .with_signer(&[&authority_seeds]),
                amount,
            )?;
        }
        emit!(DidClaim {
            round_id,
            user_pubkey: ctx.accounts.authority.key(),
            claim_amount: amount,
        });
        Ok(())
    }

    pub fn claim_and_bet(
        ctx: Context<ClaimAndBet>,
        claim_round_id: u64,
        bet_round_id: u64,
        bet_amount: u64,
        bet_type: u8,
    ) -> ProgramResult {
        let claim_round = &mut ctx.accounts.claim_round;
        let claim_bet = &mut ctx.accounts.claim_bet;
        let bonus = (claim_round.deposit_down + claim_round.deposit_up)
            * (10000 - ctx.accounts.pool.fee_rate)
            / 10000;
        let amount = if claim_round.closed_price > claim_round.lock_price {
            if claim_round.deposit_up > 0 {
                let amount =
                    bonus as u128 * claim_bet.bet_up as u128 / claim_round.deposit_up as u128;
                amount as u64
            } else {
                0
            }
        } else {
            if claim_round.deposit_down > 0 {
                let amount =
                    bonus as u128 * claim_bet.bet_down as u128 / claim_round.deposit_down as u128;
                amount as u64
            } else {
                0
            }
        };
        claim_bet.is_active = false;
        claim_round.take_amount += amount;
        if amount > 0 {
            let pool_id_bytes = ctx.accounts.pool.pool_id.to_be_bytes();
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[pool_id_bytes.as_ref()], ctx.program_id);
            let authority_seeds = [pool_id_bytes.as_ref(), &[vault_authority_bump]];
            token::transfer(
                ctx.accounts
                    .into_claim_context()
                    .with_signer(&[&authority_seeds]),
                amount,
            )?;
        }
        emit!(DidClaim {
            round_id: claim_round_id,
            user_pubkey: ctx.accounts.authority.key(),
            claim_amount: amount,
        });

        // bet
        let bet_round = &mut ctx.accounts.bet_round;
        let user_bet = &mut ctx.accounts.user_bet;
        if bet_type == 0 {
            bet_round.deposit_down += bet_amount;
            user_bet.bet_down += bet_amount;
        } else {
            bet_round.deposit_up += bet_amount;
            user_bet.bet_up += bet_amount;
        }
        user_bet.bet_time = ctx.accounts.clock.unix_timestamp;
        user_bet.is_active = true;
        token::transfer(ctx.accounts.into_bet_context(), bet_amount)?;
        emit!(DidBet {
            round_id: bet_round_id,
            user_pubkey: ctx.accounts.authority.key(),
            bet_amount,
            bet_type,
        });
        Ok(())
    }

    pub fn take_fee(ctx: Context<TakeFee>, round_id: u64) -> ProgramResult {
        let cur_round = &mut ctx.accounts.cur_round;
        let amount =
            (cur_round.deposit_down + cur_round.deposit_up) * ctx.accounts.pool.fee_rate / 10000;
        cur_round.status = 3;
        if amount > 0 {
            let pool_id_bytes = ctx.accounts.pool.pool_id.to_be_bytes();
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[pool_id_bytes.as_ref()], ctx.program_id);
            let authority_seeds = [pool_id_bytes.as_ref(), &[vault_authority_bump]];
            token::transfer(
                ctx.accounts
                    .into_transfer_context()
                    .with_signer(&[&authority_seeds]),
                amount,
            )?;
        }
        emit!(DidTakeFee {
            round_id,
            take_amount: amount,
        });
        Ok(())
    }

    pub fn update_pool(ctx: Context<UpdatePool>, duration: i64, fee_rate: u64) -> ProgramResult {
        let pool = &mut ctx.accounts.pool;
        pool.fee_rate = fee_rate;
        pool.duration = duration;
        pool.authority = ctx.accounts.new_auth.key();
        pool.feed_account = ctx.accounts.feed_account.key();
        Ok(())
    }

    pub fn free_round(ctx: Context<FreeRound>, _round_id: u64) -> ProgramResult {
        let cur_round = &mut ctx.accounts.cur_round;
        cur_round.status = 3;
        let amount = cur_round.deposit_down + cur_round.deposit_down - cur_round.take_amount;
        let pool_id_bytes = ctx.accounts.pool.pool_id.to_be_bytes();
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[pool_id_bytes.as_ref()], ctx.program_id);
        let authority_seeds = [pool_id_bytes.as_ref(), &[vault_authority_bump]];
        if amount > 0 {
            token::transfer(
                ctx.accounts
                    .into_transfer_context()
                    .with_signer(&[&authority_seeds]),
                amount,
            )?;
        }
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(pool_id: u64)]
pub struct CreatePool<'info> {
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [pool_id.to_be_bytes().as_ref()],
        bump,
        payer = authority,
    )]
    pub pool: Box<Account<'info, Pool>>,
    #[account(
        init,
        seeds = [b"token", pool.key().as_ref()],
        bump,
        payer = authority,
        token::mint = token_mint,
        token::authority = pool,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_mint: Box<Account<'info, Mint>>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct StartRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
        has_one = token_program,
        has_one = token_mint
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        seeds = [b"round", pool.key().as_ref(), pool.next_round.to_be_bytes().as_ref()],
        bump,
        payer = authority,
        constraint = pool.latest_time + pool.duration <= clock.unix_timestamp
    )]
    pub next_round: Box<Account<'info, Round>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct LockRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
        has_one = feed_account,
        has_one = token_program,
        has_one = token_mint
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        seeds = [b"round", pool.key().as_ref(), pool.next_round.to_be_bytes().as_ref()],
        bump,
        payer = authority,
    )]
    pub next_round: Box<Account<'info, Round>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-1).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.start_time + pool.duration <= clock.unix_timestamp,
        constraint = cur_round.status == 0,
    )]
    pub cur_round: Account<'info, Round>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

// Start the next round n, lock price for round n-1, end round n-2
#[derive(Accounts)]
pub struct ProcessRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
        has_one = feed_account,
        has_one = token_program,
        has_one = token_mint
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        seeds = [b"round", pool.key().as_ref(), pool.next_round.to_be_bytes().as_ref()],
        bump,
        payer = authority,
    )]
    pub next_round: Box<Account<'info, Round>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-1).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.start_time + pool.duration <= clock.unix_timestamp,
        constraint = cur_round.status == 0,
    )]
    pub cur_round: Account<'info, Round>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-2).to_be_bytes().as_ref()],
        bump,
        constraint = pre_round.lock_time + pool.duration <= clock.unix_timestamp,
        constraint = pre_round.status == 1,
    )]
    pub pre_round: Account<'info, Round>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct PauseRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
        has_one = feed_account
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-1).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.start_time + pool.duration <= clock.unix_timestamp,
        constraint = cur_round.status == 0,
    )]
    pub cur_round: Account<'info, Round>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-2).to_be_bytes().as_ref()],
        bump,
        constraint = pre_round.lock_time + pool.duration <= clock.unix_timestamp,
        constraint = pre_round.status == 1,
    )]
    pub pre_round: Account<'info, Round>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct CloseRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
        has_one = feed_account
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (pool.next_round-1).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.lock_time + pool.duration <= clock.unix_timestamp,
        constraint = cur_round.status == 1,
    )]
    pub cur_round: Account<'info, Round>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(bet_amount: u64, round_id: u64)]
pub struct Bet<'info> {
    pub authority: Signer<'info>,
    pub pool: Box<Account<'info, Pool>>,
    #[account(
        mut,
        seeds = [b"token", pool.key().as_ref()],
        bump,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = bet_amount > 0,
        constraint = token_user.amount >= bet_amount
    )]
    pub token_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (round_id).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.status == 0,
    )]
    pub cur_round: Box<Account<'info, Round>>,
    #[account(
        init_if_needed,
        seeds = [b"bet", cur_round.key().as_ref(), authority.key().as_ref()],
        bump,
        payer = authority,
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Bet<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_user.to_account_info(),
            to: self.token_vault.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(round_id: u64)]
pub struct Claim<'info> {
    pub authority: Signer<'info>,
    pub pool: Box<Account<'info, Pool>>,
    #[account(
        mut,
        seeds = [b"token", pool.key().as_ref()],
        bump,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (round_id).to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.status >= 2,
    )]
    pub cur_round: Box<Account<'info, Round>>,
    #[account(
        mut,
        seeds = [b"bet", cur_round.key().as_ref(), authority.key().as_ref()],
        bump,
        constraint = user_bet.is_active,
        close = authority
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Claim<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_vault.to_account_info(),
            to: self.token_user.to_account_info(),
            authority: self.pool.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(bet_amount: u64, claim_round_id: u64, bet_round_id: u64)]
pub struct ClaimAndBet<'info> {
    pub authority: Signer<'info>,
    pub pool: Box<Account<'info, Pool>>,
    #[account(
        mut,
        seeds = [b"token", pool.key().as_ref()],
        bump,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = bet_amount > 0,
        constraint = token_user.amount >= bet_amount
    )]
    pub token_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), (claim_round_id).to_be_bytes().as_ref()],
        bump,
        constraint = claim_round.status >= 2,
    )]
    pub claim_round: Box<Account<'info, Round>>,
    #[account(
        mut,
        seeds = [b"bet", claim_round.key().as_ref(), authority.key().as_ref()],
        bump,
        constraint = claim_bet.is_active,
        close = authority
    )]
    pub claim_bet: Box<Account<'info, UserBet>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), bet_round_id.to_be_bytes().as_ref()],
        bump,
        constraint = bet_round.status == 0,
    )]
    pub bet_round: Box<Account<'info, Round>>,
    #[account(
        init_if_needed,
        seeds = [b"bet", bet_round.key().as_ref(), authority.key().as_ref()],
        bump,
        payer = authority,
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

impl<'info> ClaimAndBet<'info> {
    fn into_bet_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_user.to_account_info(),
            to: self.token_vault.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    fn into_claim_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_vault.to_account_info(),
            to: self.token_user.to_account_info(),
            authority: self.pool.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(round_id: u64)]
pub struct TakeFee<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"token", pool.key().as_ref()],
        bump,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), round_id.to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.status == 2,
    )]
    pub cur_round: Box<Account<'info, Round>>,
    #[account(
        has_one = authority,
    )]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> TakeFee<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_vault.to_account_info(),
            to: self.token_user.to_account_info(),
            authority: self.pool.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct UpdatePool<'info> {
    pub authority: Signer<'info>,
    pub new_auth: AccountInfo<'info>,
    #[account(
        mut,
        has_one = authority,
    )]
    pub pool: Account<'info, Pool>,
    pub feed_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_round_id: u64)]
pub struct FreeRound<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"token", pool.key().as_ref()],
        bump,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"round", pool.key().as_ref(), _round_id.to_be_bytes().as_ref()],
        bump,
        constraint = cur_round.status >= 2,
        constraint = (cur_round.take_amount + 5000 > (cur_round.deposit_up + cur_round.deposit_down)) || cur_round.start_time + 15552000 <= clock.unix_timestamp,
        close = authority
    )]
    pub cur_round: Box<Account<'info, Round>>,
    #[account(
        has_one = authority,
    )]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

impl<'info> FreeRound<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token_vault.to_account_info(),
            to: self.token_user.to_account_info(),
            authority: self.pool.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub pool_id: u64,
    // Priviledged account.
    pub authority: Pubkey,
    pub fee_rate: u64,
    // duration of one round (s)
    pub duration: i64,
    pub next_round: u64,
    pub latest_time: i64,
    // Swap frontend for the dex.
    pub token_program: Pubkey,
    pub token_mint: Pubkey,
    // price feed account
    pub feed_account: Pubkey,
}

#[account]
#[derive(Default)]
pub struct Round {
    pub start_time: i64,
    pub lock_time: i64,
    pub deposit_up: u64,
    pub deposit_down: u64,
    pub take_amount: u64,
    pub lock_price: i64,
    pub closed_price: i64,
    // 0: active, 1: locked, 2: closed, 3: fee taked
    pub status: u8,
}

#[account]
#[derive(Default)]
pub struct UserBet {
    pub bet_time: i64,
    pub bet_up: u64,
    pub bet_down: u64,
    pub is_active: bool,
}

#[event]
pub struct DidStartRound {
    start_time: i64,
    round_id: u64,
}

#[event]
pub struct DidLockRound {
    lock_time: i64,
    lock_price: i64,
    round_id: u64,
}

#[event]
pub struct DidProcessRound {
    lock_time: i64,
    lock_price: i64,
    round_id: u64,
}

#[event]
pub struct DidBet {
    round_id: u64,
    user_pubkey: Pubkey,
    bet_amount: u64,
    bet_type: u8,
}

#[event]
pub struct DidClaim {
    round_id: u64,
    user_pubkey: Pubkey,
    claim_amount: u64,
}

#[event]
pub struct DidTakeFee {
    round_id: u64,
    take_amount: u64,
}
