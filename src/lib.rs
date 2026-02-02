use pinocchio::{AccountView,error::ProgramError,ProgramResult,entrypoint,Address};
//use solana_address::declare_id;
entrypoint!(process_instructions);
mod state;
mod instructions;
pub use instructions::*;

// declare_id!("22222222222222222222222222222222222222222222");
pub const ID: Address = Address::new_from_array([
    0x0f, 0x1e, 0x6b, 0x14, 0x21, 0xc0, 0x4a, 0x07,
    0x04, 0x31, 0x26, 0x5c, 0x19, 0xc5, 0xbb, 0xee,
    0x19, 0x92, 0xba, 0xe8, 0xaf, 0xd1, 0xcd, 0x07,
    0x8e, 0xf8, 0xaf, 0x70, 0x47, 0xdc, 0x11, 0xf7,
]);
pub fn process_instructions(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8]
) -> ProgramResult{
    match instruction_data.split_first() {
        Some((Make::DISCRIMINATOR,data)) => make::Make::try_from((accounts,data))?.process(),
        Some((Take::DISCRIMINATOR,_)) => take::Take::try_from(accounts)?.process(),
    }
}

