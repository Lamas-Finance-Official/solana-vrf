use std::{cell::Ref, ops::RangeInclusive};

use crate::{ErrorCode, VrfStatus};
use anchor_lang::{prelude::*, Discriminator};
use num_traits::{AsPrimitive, PrimInt};

#[zero_copy]
#[repr(packed)]
pub struct AccountMetaPacked {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AccountMetaBorsh {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[zero_copy]
#[repr(packed)]
pub struct CallbackPacked {
    /// Program ID of the callback program being invoked.
    pub program_id: Pubkey,
    /// The accounts being used in the callback instruction.
    pub accounts: [AccountMetaPacked; 32],
    /// The number of accounts used in the callback.
    pub accounts_len: u32,
    /// The serialized instruction data.
    pub ix_data: [u8; 1024],
    /// The number of serialized bytes in the instruction data.
    pub ix_data_len: u32,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Callback {
    /// The program ID of the callback program being invoked.
    pub program_id: Pubkey,
    /// The accounts being used in the callback instruction.
    pub accounts: Vec<AccountMetaBorsh>,
    /// The serialized instruction data.
    pub ix_data: Vec<u8>,
}

#[account(zero_copy)]
#[repr(C, packed)]
pub struct VrfAccountData {
    pub status: VrfStatus,

    // Pubkey of the offchain oracle that can fulfill this request
    pub signer: Pubkey,

    pub result: [u8; VrfAccountData::RESULT_BYTE_LEN],
    pub proof: [u8; VrfAccountData::PROOF_BYTE_LEN],
    pub seeds: [u8; VrfAccountData::SEEDS_BYTE_LEN],

    /// The unix timestamp when the VRF round was opened.
    pub request_timestamp: i64,

    /// The callback that is invoked when we fulfill the request.
    pub callback: CallbackPacked,
    /// Reserved for future info.
    pub _buf: [u8; 1024],
}

impl Default for VrfAccountData {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl VrfAccountData {
    pub const RESULT_BYTE_LEN: usize = 32;
    pub const PROOF_BYTE_LEN: usize = 80;
    pub const SEEDS_BYTE_LEN: usize = 32;

    pub fn new<'info>(account_info: &'info AccountInfo) -> anchor_lang::Result<Ref<'info, Self>> {
        let data = account_info.try_borrow_data()?;
        if data.len() < Self::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != Self::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..std::mem::size_of::<Self>() + 8])
        }))
    }

    pub fn from_bytes(data: &[u8]) -> anchor_lang::Result<&Self> {
        if data.len() < Self::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != Self::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(bytemuck::from_bytes(
            &data[8..std::mem::size_of::<Self>() + 8],
        ))
    }

    pub fn bounded_result<T>(self, range: RangeInclusive<T>) -> T
    where
        T: PrimInt + AsPrimitive<i128>,
        i128: AsPrimitive<T>,
    {
        let v = i128::from_be_bytes(self.result[0..16].try_into().unwrap());
        let bound: i128 = (*range.end() - *range.start()).as_();
        ((v % bound) + range.start().as_()).as_()
    }
}

#[test]
fn size() {
    println!(
        "sizeof VrfAccountData: {}",
        std::mem::size_of::<VrfAccountData>()
    );
}
