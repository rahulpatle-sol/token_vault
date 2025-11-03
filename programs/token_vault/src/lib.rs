use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_lang::solana_program::clock::Clock; 

// 1. Program ID: You MUST update this in Anchor.toml after running 'anchor keys list'
declare_id!("8WijvK9GJ5q1KSP1o1xuH4J1qw9VHie47riZecc9zmBS"); 

#[program]
pub mod token_vault {
    use super::*;

    // Instruction 1: Initialize the Vault
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        _vault_bump: u8,         // Passed from client, stored for future PDA checks
        _authority_bump: u8,     // Passed from client, stored for future PDA checks
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        
        // Initialize the Vault account's fields
        vault.authority = ctx.accounts.payer.key();
        vault.token_account = ctx.accounts.token_account.key();
        vault.bump = _vault_bump;
        vault.authority_bump = _authority_bump;
        vault.is_locked = false; // Starts unlocked
        vault.unlock_timestamp = 0; // Starts with no time lock

        msg!("Vault Initialized!");
        msg!("Vault Authority (Owner): {}", vault.authority);
        msg!("Vault Token Account: {}", vault.token_account);

        Ok(())
    }

    // Instruction 2: Deposit Tokens
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            // Note: .to_account_info() works fine even with Box<Account<...>>
            from: ctx.accounts.user_token_account.to_account_info(), 
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // Perform the CPI to transfer tokens
        token::transfer(cpi_ctx, amount)?;

        msg!("Deposited {} tokens into the vault.", amount);
        Ok(())
    }

    // Instruction 3: Withdraw Tokens (Conditional)
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let clock = Clock::get()?;

        // --- Security Check 1: Time Lock ---
        // Ensure vault is NOT locked OR that the lock time has expired
        require!(
            !vault.is_locked || clock.unix_timestamp >= vault.unlock_timestamp,
            VaultError::VaultStillLocked
        );

        // --- Security Check 2: Insufficient Funds (Best practice) ---
        // Note: .amount is accessed via the Boxed Account
        require!(
            ctx.accounts.vault_token_account.amount >= amount, 
            VaultError::InsufficientFunds
        );

        // 1. Setup the CPI accounts 
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(), // PDA is the authority
        };
        
        // 2. Setup the PDA signer seeds
        let vault_key = vault.key(); 
        let authority_seed = &[
            b"authority",
            vault_key.as_ref(),
            &[vault.authority_bump],
        ];
        let signer = &[&authority_seed[..]];

        // 3. Create the CPI context
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        // 4. Perform the transfer
        token::transfer(cpi_ctx, amount)?;

        msg!("Withdrew {} tokens from the vault.", amount);
        Ok(())
    }


    // Instruction 4: Lock the Vault with a Timestamp
    pub fn lock_vault(ctx: Context<LockVault>, unlock_timestamp: i64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        
        // --- Security Check 1: Ensure time is in the future ---
        let clock = Clock::get()?;
        require!(
            unlock_timestamp > clock.unix_timestamp,
            VaultError::InvalidUnlockTime
        );

        vault.is_locked = true;
        vault.unlock_timestamp = unlock_timestamp;
        
        msg!("Vault locked until timestamp: {}", unlock_timestamp);
        Ok(())
    }

    // Instruction 5: Unlock the Vault (Time-Based)
    pub fn unlock_vault(ctx: Context<UnlockVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?; // Get the current on-chain time

        // --- Security Check 1: Has enough time passed? ---
        require!(
            clock.unix_timestamp >= vault.unlock_timestamp,
            VaultError::VaultStillLocked
        );

        vault.is_locked = false;
        vault.unlock_timestamp = 0; // Reset timestamp
        
        msg!("Vault unlocked successfully at timestamp: {}", clock.unix_timestamp);
        Ok(())
    }
}

// --- Account Validation Structs ---

// Accounts for 'initialize_vault'
#[derive(Accounts)]
#[instruction(vault_bump: u8, authority_bump: u8)]
pub struct InitializeVault<'info> {
    // Vault PDA: Creates and funds the vault account
    #[account(
        init,
        payer = payer,
        seeds = [b"vault", payer.key().as_ref()], // Seeds: ["vault", payer_pubkey]
        bump,
        space = 8 + Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,

    // Vault Authority PDA: Owner of the vault's token account
    /// CHECK: This is safe because we derive it with PDA and only use it as an authority
    #[account(
        seeds = [b"authority", vault.key().as_ref()], // Seeds: ["authority", vault_pubkey]
        bump = authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    // Token Account: Holds the actual tokens, owned by the Vault Authority PDA
    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = vault_authority, // Owned by the PDA
    )]
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    pub token_account: Box<Account<'info, TokenAccount>>,
    
    // Other necessary accounts
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    pub mint: Box<Account<'info, Mint>>, 
    #[account(mut)]
    pub payer: Signer<'info>, // The wallet paying for the accounts and initializing the vault
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Accounts for 'deposit'
#[derive(Accounts)]
pub struct Deposit<'info> {
    // Vault PDA check: only the correct authority can deposit
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority // Ensures the vault is owned by the signer
    )]
    pub vault: Account<'info, Vault>,

    // User's token account (from)
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    #[account(mut, token::authority = authority)]
    pub user_token_account: Box<Account<'info, TokenAccount>>,
    
    // Vault's token account (to)
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    #[account(mut, address = vault.token_account)]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,
    
    pub authority: Signer<'info>, // The user depositing
    pub token_program: Program<'info, Token>,
}

// Accounts for 'withdraw'
#[derive(Accounts)]
pub struct Withdraw<'info> {
    // Vault PDA check: only the correct authority can withdraw
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,

    // Vault Authority PDA: The signer for the transfer out of the vault
    /// CHECK: This is safe because it is a verified PDA
    #[account(
        seeds = [b"authority", vault.key().as_ref()],
        bump = vault.authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    
    // User's token account (to)
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    #[account(mut, token::authority = authority)]
    pub user_token_account: Box<Account<'info, TokenAccount>>,
    
    // Vault's token account (from)
    // FIX: Box<Account<...>> used for large non-Anchor SPL account
    #[account(mut, address = vault.token_account)]
    pub vault_token_account: Box<Account<'info, TokenAccount>>,

    pub authority: Signer<'info>, // The user withdrawing
    pub token_program: Program<'info, Token>,
}

// Accounts for 'lock_vault'
#[derive(Accounts)]
pub struct LockVault<'info> {
    // Vault PDA check: Only the vault authority can lock it
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,
    
    pub authority: Signer<'info>, // The user locking
}

// Accounts for 'unlock_vault'
#[derive(Accounts)]
pub struct UnlockVault<'info> {
    // Vault PDA check: Only the vault authority can unlock it
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,
    
    pub authority: Signer<'info>, // The user unlocking
}


// --- Account Data Structure ---

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
    pub authority_bump: u8,
    pub is_locked: bool,
    pub unlock_timestamp: i64,
}


// --- Custom Errors ---

#[error_code]
pub enum VaultError {
    #[msg("Vault is still locked")]
    VaultStillLocked,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
    #[msg("The requested unlock time is not in the future")]
    InvalidUnlockTime,
}
