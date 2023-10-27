use anchor_lang::prelude::*;

pub const RESULT_BYTE_LEN: usize = 32;
pub const PROOF_BYTE_LEN: usize = 80;
pub const SEEDS_BYTE_LEN: usize = 32;

pub const VRF_RESULT_DISCRIMINATOR: [u8; 32] = [
    169, 181, 96, 37, 231, 213, 250, 114, 103, 201, 179, 141, 92, 38, 30, 87, 115, 210, 50, 29,
    136, 193, 41, 211, 45, 205, 112, 191, 205, 195, 2, 105,
];

#[event]
pub struct VrfRequestRandomness {
    pub vrf: Pubkey,
}

#[zero_copy]
#[repr(packed)]
pub struct AccountMetaPacked {
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

#[zero_copy]
#[repr(packed)]
pub struct VrfAccountData {
    pub result: crate::VrfResult,
    pub proof: [u8; PROOF_BYTE_LEN],
    pub seeds: [u8; SEEDS_BYTE_LEN],

    /// The unix timestamp when the VRF round was opened.
    pub request_timestamp: i64,

    /// The callback that is invoked when we fulfill the request.
    pub callback: CallbackPacked,
    /// Reserved for future info.
    pub _buf: [u8; 1024],
}

unsafe impl anchor_lang::__private::bytemuck::Pod for VrfAccountData {}
unsafe impl anchor_lang::__private::bytemuck::Zeroable for VrfAccountData {}

impl anchor_lang::Discriminator for VrfAccountData {
    const DISCRIMINATOR: [u8; 8] = [101, 35, 62, 239, 103, 151, 6, 18];
}
