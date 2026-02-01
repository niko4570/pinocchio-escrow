use std::pin;

use pinocchio::{Address, AccountView, error::ProgramError, ProgramResult, cpi::{Seed, Signer}};
use pinocchio_token::{state::TokenAccount,instructions::Transfer};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use crate::state::Escrow;

/// Creates a new escrow account for token swapping.
/// 
/// This instruction initializes the escrow data and creates the associated vault
/// token account if it doesn't already exist.
/// 
/// # Accounts
/// - `maker`: Maker's wallet account (signer)
/// - `escrow`: Escrow account to be created
/// - `mint_a`: Mint of the token to be deposited
/// - `mint_b`: Mint of the token to be received
/// - `maker_ata_a`: Maker's associated token account for mint_a
/// - `vault`: Vault token account for holding deposited tokens
/// - `system_program`: System program
/// - `token_program`: Token program
pub struct Make<'a> {
    /// Accounts required for the make instruction
    pub accounts: MakeAccounts<'a>,
    /// Instruction data containing seed, receive amount, and send amount
    pub instruction_data: MakeInstructionData,
    /// Bump seed for program address derivation
    pub bump: u8,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for Make<'a> {
    type Error = ProgramError;

    /// Creates a `Make` instruction from account views and instruction data.
    /// 
    /// Validates that the provided escrow account matches the expected program-derived address.
    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = MakeAccounts::try_from(accounts)?;
        let instruction_data = MakeInstructionData::try_from(data)?;
        
        // Derive the expected escrow address using program address derivation
        let (escrow_address, bump) = Address::find_program_address(
            &[
                b"escrow",
                accounts.maker.address().as_ref(),
                &instruction_data.seed.to_le_bytes(),
            ],
            &crate::ID,
        );
        
        // Validate that the provided escrow account matches the expected address
        if accounts.escrow.address() != &escrow_address {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { accounts, instruction_data, bump })
    }
}

impl<'a> Make<'a> {
    /// Instruction discriminator for the Make instruction
    pub const DISCRIMINATOR: &'a u8 = &0;

    /// Processes the Make instruction.
    /// 
    /// This function:
    /// 1. Creates the escrow account with minimum balance
    /// 2. Initializes the escrow account data
    /// 3. Creates the vault token account if it doesn't exist
    pub fn process(&mut self) -> ProgramResult {
        let accounts = &self.accounts;
        let instruction_data = &self.instruction_data;
        
        // Prepare seeds for program-signed CPI
        let seed_binding = instruction_data.seed.to_le_bytes();
        let bump_binding = [self.bump];
        let seeds = [
            Seed::from(b"escrow"),
            Seed::from(accounts.maker.address().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];

        // Create the escrow account with minimum balance
        let signer = &[Signer::from(&seeds)];
        CreateAccount::with_minimum_balance(
            accounts.maker,
            accounts.escrow,
            Escrow::LEN as u64,
            &crate::ID,
            None,
        )?
        .invoke_signed(signer)?;
        
        // Initialize escrow account data
        let mut data = self.accounts.escrow.try_borrow_mut()?;
        let escrow = Escrow::load_mut(&mut data)?;
        escrow.set_inner(
            instruction_data.seed,
            accounts.maker.address().clone(),
            accounts.mint_a.address().clone(),
            accounts.mint_b.address().clone(),
            instruction_data.receive,
            [self.bump],
        );

        // Create the vault token account if it doesn't exist
        if accounts.vault.is_data_empty() {
            pinocchio_associated_token_account::instructions::Create {
                funding_account: accounts.maker,
                account: accounts.vault,
                wallet: accounts.escrow,
                mint: accounts.mint_a,
                system_program: accounts.system_program,
                token_program: accounts.token_program,
            }
            .invoke()?;
        }
        
        // Transfer tokens from maker to vault
        Transfer {
            from: accounts.maker_ata_a,
            to: accounts.vault,
            authority: accounts.maker,
            amount: instruction_data.amount,
        }   
        .invoke()?;

        Ok(())
    }
}

/// Accounts required for the Make instruction
pub struct MakeAccounts<'a> {
    /// Maker's wallet account (signer)
    pub maker: &'a AccountView,
    /// Escrow account to be created
    pub escrow: &'a AccountView,
    /// Mint of the token to be deposited
    pub mint_a: &'a AccountView,
    /// Mint of the token to be received
    pub mint_b: &'a AccountView,
    /// Maker's associated token account for mint_a
    pub maker_ata_a: &'a AccountView,
    /// Vault token account for holding deposited tokens
    pub vault: &'a AccountView,
    /// System program
    pub system_program: &'a AccountView,
    /// Token program
    pub token_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for MakeAccounts<'a> {
    type Error = ProgramError;

    /// Creates `MakeAccounts` from a slice of `AccountView`s.
    /// 
    /// Validates that all required accounts are present and have correct properties.
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        // Extract accounts from the slice
        let [maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Validate that the maker account is a signer
        SignerAccount::check(maker)?;
        
        // Validate that mint accounts are owned by the system program
        if !mint_a.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if !mint_b.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        
        // Validate that the maker's ATA is correctly configured
        AssociatedTokenAccount::check(maker_ata_a, maker, mint_a, token_program)?;

        Ok(Self {
            maker,
            escrow,
            mint_a,
            mint_b,
            maker_ata_a,
            vault,
            system_program,
            token_program
        })
    }
}

/// Instruction data for the Make instruction
pub struct MakeInstructionData {
    /// Unique seed for escrow account derivation
    pub seed: u64,
    /// Amount of mint_b tokens to receive
    pub receive: u64,
    /// Amount of mint_a tokens to deposit
    pub amount: u64,
}

impl<'a> TryFrom<&'a [u8]> for MakeInstructionData {
    type Error = ProgramError;

    /// Creates `MakeInstructionData` from raw bytes.
    /// 
    /// Validates that:
    /// 1. The data length is correct (24 bytes)
    /// 2. The amount is non-zero
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Validate data length
        if data.len() != core::mem::size_of::<u64>() * 3 {
            return Err(ProgramError::InvalidInstructionData);
        }
        
        // Parse instruction data
        let seed = u64::from_le_bytes(data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?);
        let receive = u64::from_le_bytes(data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?);
        let amount = u64::from_le_bytes(data[16..24].try_into().map_err(|_| ProgramError::InvalidInstructionData)?);
        
        // Validate that amount is non-zero
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        
        Ok(Self { seed, receive, amount })
    }
}

// Account validation utilities

/// Validator for signer accounts
pub struct SignerAccount;

impl SignerAccount {
    /// Validates that the account is a signer
    pub fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.is_signer() {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(())
    }
}

/// Validator for mint accounts
pub struct MintInterface;

impl MintInterface {
    /// Validates that the account is owned by the token program
    pub fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }
}

/// Validator for associated token accounts
pub struct AssociatedTokenAccount;

impl AssociatedTokenAccount {
    /// Validates that an associated token account is correctly configured
    /// 
    /// Validates:
    /// 1. The account is owned by the token program
    /// 2. The account has the correct data length
    /// 3. The account's mint matches the provided mint
    /// 4. The account's owner matches the provided authority
    pub fn check(
        ata: &AccountView,
        authority: &AccountView,
        mint: &AccountView,
        token_program: &AccountView,
    ) -> Result<(), ProgramError> {
        // Validate ownership by token program
        if !ata.owned_by(token_program.address()) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate data length
        if ata.data_len() != TokenAccount::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Validate token account data
        let token_account = TokenAccount::from_account_view(ata)?;
        if token_account.mint() != mint.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_account.owner() != authority.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
    pub fn init_if_needed(
        ata: &AccountView,
        mint: &AccountView,
        authority: &AccountView,
        payer: &AccountView,
        system_program: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult {
        CreateIdempotent{
            funding_account: payer,
            account: ata,
            wallet: authority,
            mint,
            system_program,
            token_program,
        }
        .invoke()?;
        Self::check(ata, authority, mint, token_program)
    }
}

pub struct ProgramAccount;

impl ProgramAccount{
        
    /// 1. Owner matches the program ID
    /// 2. account is not the signer
    /// 3. data can't be empty
    pub fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.is_signer() {
            return Err(ProgramError::InvalidInstructionData);
        }
        if account.data_len().eq(&0) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}
