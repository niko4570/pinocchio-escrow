use pinocchio::{AccountView, Address, ProgramResult, cpi::{Seed,Signer}, error::ProgramError };
use pinocchio_token::{instructions::{Transfer,CloseAccount},state::TokenAccount};
use super::make::{MintInterface,SignerAccount,AssociatedTokenAccount,ProgramAccount};
use crate::state::Escrow;

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
    /// 3. vault:mint_a -> taker_ata_a
    /// 4. close vault
    /// 5. taker:mint_b -> maker_ata_b
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

        // check escrow is valid
        let data =self.accounts.escrow.try_borrow()?;
        let escrow=Escrow::load(&data)?;
        let (escrow_address,_)=Address::find_program_address(&[
            b"escrow",
            self.accounts.maker.address().as_ref(),
            &escrow.seed.to_le_bytes(),
            &escrow.bump,
        ],&crate::ID);
        if escrow_address!=*self.accounts.escrow.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        let seed_binding=escrow.seed.to_le_bytes();
        let bump_binding=escrow.bump;
        let seed=[
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.address().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];
        let signer=Signer::from(&seed);

        let amount=TokenAccount::from_account_view(self.accounts.vault)?.amount();
        
        // Transfer from vault to taker_ata_a
        // vault:mint_a -> taker_ata_a
        Transfer{
            from: self.accounts.vault,
            to: self.accounts.taker_ata_a,
            authority: self.accounts.escrow,
            amount,
        }.invoke_signed(&[signer.clone()])?;
        
        // After transfer, the vault is empty
        // Close the vault
        CloseAccount{
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }.invoke_signed(&[signer.clone()])?;
        
        // The vault is closing, so taker should tranfer mint_b to maker
        Transfer{
            from: self.accounts.taker_ata_b,
            to: self.accounts.maker_ata_b,
            authority: self.accounts.taker,
            amount,
        }.invoke()?;

        // Close the Escrow
        drop(data);
        ProgramAccount::close(self.accounts.escrow, self.accounts.taker)
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
