use pinocchio::{AccountView,ProgramResult,error::ProgramError,cpi::{Seed,Signer} };
use pinocchio_token::{instructions::Transfer,state::{Mint, TokenAccount}};
use super::make::{MintInterface,SignerAccount,AssociatedTokenAccount,ProgramAccount};

pub struct Take<'a> {
    pub accounts: TakeAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountView]> for Take<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {

        Ok(Self{
            accounts: TakeAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> Take<'a> {
    pub const DISCRIMINATOR: &'a u8=&1;
    
    /// 1. receive / pay ATA is existed
    /// 2. escrow is valid
    /// 3. vault:mint_b -> taker_ata_b
    /// 4. vault:mint_a -> taker_ata_a
    /// 5. close vault
    /// 6. close escrow
    pub fn process(&self) -> ProgramResult {
        
        AssociatedTokenAccount::init_if_needed(
            self.accounts.taker_ata_a,
            self.accounts.taker,
            self.accounts.mint_a,
            self.accounts.system_program,
            self.accounts.system_program,
            self.accounts.token_program,
        )?;

        AssociatedTokenAccount::init_if_needed(
            self.accounts.maker_ata_b,
            self.accounts.mint_b,
            self.accounts.taker,                
            self.accounts.maker,
            self.accounts.system_program,
            self.accounts.token_program,
        )?;
        Ok(())
    }
}

pub struct TakeAccounts<'a> {
    pub taker: &'a AccountView,
    pub maker: &'a AccountView,
    pub escrow: &'a AccountView,
    pub mint_a: &'a AccountView,
    pub mint_b: &'a AccountView,
    pub vault: &'a AccountView,
    pub taker_ata_a: &'a AccountView,
    pub taker_ata_b: &'a AccountView,
    pub maker_ata_b: &'a AccountView,
    pub system_program: &'a AccountView,
    pub token_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for TakeAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [taker, maker, escrow, mint_a, mint_b, vault, taker_ata_a, taker_ata_b, maker_ata_b, system_program, token_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(taker)?;
        ProgramAccount::check(escrow)?;
        MintInterface::check(mint_a)?;
        MintInterface::check(mint_b)?;
        AssociatedTokenAccount::check(taker_ata_b,taker,mint_b,token_program)?;
        AssociatedTokenAccount::check(vault,escrow,mint_a,token_program)?;

        Ok(Self {
            taker,
            maker,
            escrow,
            mint_a,
            mint_b,
            vault,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            system_program,
            token_program,
        })
    }
}
