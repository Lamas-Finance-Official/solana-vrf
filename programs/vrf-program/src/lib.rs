pub use crate::{
    action_fulfill_randomness::*,
    action_request_randomness::*,
    error::VrfError,
    vrf::{AccountMetaPacked, Callback, CallbackPacked, VrfAccountData},
};
use anchor_lang::prelude::*;

mod action_fulfill_randomness;
mod action_request_randomness;
mod error;
mod vrf;

declare_id!("DEoxdV1CCWvbeGp8PpwkUifmm3pV5AgtFwFaS4P7qZeZ");

#[program]
pub mod anchor_vrf_program {
    use super::*;

    pub fn request_randomness(
        ctx: Context<RequestRandomness>,
        params: RequestRandomnessParams,
    ) -> anchor_lang::Result<()> {
        RequestRandomness::execute(&ctx, &params)
    }

    pub fn fulfill_randomness<'info>(
        ctx: Context<'_, '_, '_, 'info, FulfillRandomness<'info>>,
        params: FulfillRandomnessParams,
    ) -> anchor_lang::Result<()> {
        FulfillRandomness::execute(&ctx, &params)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum VrfStatus {
    Requesting,
    Generating,
    Success,
}

#[event]
pub struct RequestRandomnessEvent {
    vrf: Pubkey,
}
