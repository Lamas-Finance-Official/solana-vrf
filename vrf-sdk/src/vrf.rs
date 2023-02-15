use anchor_lang::prelude::*;

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
    pub result: [u8; Self::RESULT_BYTE_LEN],
    pub proof: [u8; Self::PROOF_BYTE_LEN],
    pub seeds: [u8; Self::SEEDS_BYTE_LEN],

    /// The unix timestamp when the VRF round was opened.
    pub request_timestamp: i64,

    /// The callback that is invoked when we fulfill the request.
    pub callback: CallbackPacked,
    /// Reserved for future info.
    pub _buf: [u8; 1024],
}

impl anchor_lang::Discriminator for VrfAccountData {
    const DISCRIMINATOR: [u8; 8] = [101, 35, 62, 239, 103, 151, 6, 18];
}

impl VrfAccountData {
    pub const RESULT_BYTE_LEN: usize = 32;
    pub const PROOF_BYTE_LEN: usize = 80;
    pub const SEEDS_BYTE_LEN: usize = 32;
}
