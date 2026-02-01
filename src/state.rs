use pinocchio::{error::ProgramError,Address};
use core::mem::size_of;
#[repr(C)]

pub struct Escrow {
    pub seed: u64,
    pub maker: Address,
    pub mint_a: Address,
    pub mint_b: Address,
    pub receive: u64,
    pub bump: [u8;1],
}

impl Escrow {
    pub const LEN: usize=size_of::<u64>()+size_of::<Address>()*3+size_of::<u64>()+size_of::<[u8;1]>();

    #[inline(always)]
    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self,ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok( unsafe{&mut *core::mem::transmute::<*mut u8, *mut Self>(bytes.as_mut_ptr())})
    }

    #[inline(always)]
    pub fn load(bytes: &[u8]) -> Result<&Self,ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok( unsafe{& *core::mem::transmute::<*const u8, *const Self>(bytes.as_ptr())})
    }

    #[inline(always)]
    pub fn set_seeds(&mut self,seed: u64) {
        self.seed = seed;
    }
    #[inline(always)]
    pub fn set_maker(&mut self,maker: Address) {
        self.maker = maker;
    }
    #[inline(always)]
    pub fn set_mint_a(&mut self,mint_a: Address) {
        self.mint_a = mint_a;
    }
    #[inline(always)]
    pub fn set_mint_b(&mut self,mint_b: Address) {
        self.mint_b = mint_b;
    }
    #[inline(always)]
    pub fn set_receive(&mut self, receive: u64) {
        self.receive = receive;
    }
    #[inline(always)]
    pub fn set_bump(&mut self, bump: [u8;1]) {
        self.bump = bump;
    }

    #[inline(always)]
    pub fn set_inner(&mut self, seed: u64, maker: Address, mint_a: Address, mint_b: Address, receive: u64, bump: [u8;1]) {
        self.set_seeds(seed);
        self.set_maker(maker);
        self.set_mint_a(mint_a);
        self.set_mint_b(mint_b);
        self.set_receive(receive);
        self.set_bump(bump);
    }
}