use std::ops::{DerefMut, RangeInclusive};

use anchor_lang::{prelude::*, InstructionData, ToAccountMetas, ZeroCopy};
use num_traits::{AsPrimitive, PrimInt};

pub mod vrf;
pub use vrf_sdk_macro::declare_vrf_state;

/// Hidden, to be used by proc-macro declare_vrf_state
#[doc(hidden)]
pub mod __private {
    pub use anchor_lang::{
        prelude::Pubkey,
        AccountDeserialize, Discriminator, Owner, ZeroCopy,
        __private::bytemuck::{from_bytes, Pod, Zeroable},
        error, Result,
    };
}

/// Account size use when init a new [VrfState](`declare_vrf_state`)
///
/// Example
/// ```ignore
/// #[derive(Accounts)]
/// pub struct RequestRandomness<'info> {
///     #[account(
/// 		init,
/// 		payer = payer,
/// 		space = vrf_sdk::ACCOUNT_SIZE,
/// 		seeds = [
/// 			// Specify PDA seeds
/// 		],
/// 		bump,
/// 	)]
///     vrf: AccountLoader<'info, VrfState>,
///     system_program: Program<'info, System>,
/// }
/// ```
pub const ACCOUNT_SIZE: usize = std::mem::size_of::<vrf::VrfAccountData>() + /* DISCRIMINATOR */ 8;

/// Request a new randomness value.
/// The supplied `VrfState` should be created seperately for each request
///
/// Example
/// ```ignore
/// vrf_sdk::request_randomness(
/// 	&seeds,
/// 	&ctx.accounts.vrf,
/// 	// This struct will be auto generated by anchor
/// 	// if you has an struct like
/// 	//
/// 	// #[derive(Accounts)]
/// 	// struct OnRandomnessResponse
/// 	accounts::OnRandomnessResponse {
/// 		vrf: ctx.accounts.vrf.key(),
/// 	},
/// 	// This struct will be auto generated by anchor
/// 	// if you has an instruction call: on_randomness_response
/// 	instruction::OnRandomnessResponse {
///     	params: params,
/// 	},
/// )?;
/// ```
pub fn request_randomness<VRF, CB, IX>(
    seeds: &[u8],
    vrf: &AccountLoader<'_, VRF>,
    callback: CB,
    callback_ix_data: IX,
) -> anchor_lang::Result<()>
where
    VRF: DerefMut<Target = vrf::VrfAccountData> + ZeroCopy + Owner,
    CB: ToAccountMetas,
    IX: InstructionData,
{
    if seeds.is_empty() || seeds.iter().map(|v| *v as u32).sum::<u32>() == 0 {
        return Err(Error::AnchorError(AnchorError {
            error_name: "Vrf seeds is zeroed or empty".to_owned(),
            error_code_number: ErrorCode::ConstraintSeeds.into(),
            error_msg: "Vrf seeds is zeroed or empty".to_owned(),
            error_origin: None,
            compared_values: None,
        }));
    }

    let vrf_pubkey = vrf.key();
    let vrf = &mut vrf.load_init()?;
    let vrf = vrf.deref_mut();

    vrf.seeds[0..seeds.len().min(vrf::SEEDS_BYTE_LEN)]
        .copy_from_slice(&seeds[0..seeds.len().min(vrf::SEEDS_BYTE_LEN)]);

    vrf.request_timestamp = Clock::get()?.unix_timestamp;
    vrf.callback.program_id = VRF::owner();

    let metas = callback.to_account_metas(None);
    vrf.callback.accounts_len = metas.len() as u32;
    for (i, meta) in metas.iter().enumerate() {
        vrf.callback.accounts[i] = vrf::AccountMetaPacked {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        };
    }

    let ix_data = callback_ix_data.data();
    vrf.callback.ix_data_len = ix_data.len() as u32;
    vrf.callback.ix_data[0..ix_data.len()].copy_from_slice(&ix_data);

    emit!(vrf::VrfRequestRandomness { vrf: vrf_pubkey });
    Ok(())
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize)]
#[repr(packed)]
pub struct VrfResult {
    pub result: [u8; vrf::RESULT_BYTE_LEN],
}

impl Default for VrfResult {
    fn default() -> Self {
        Self {
            result: vrf::VRF_RESULT_DISCRIMINATOR,
        }
    }
}

impl VrfResult {
    /// Generate a random number from the `VrfState`
    /// that satisfy the provided range.
    ///
    /// Example
    /// ```ignore
    ///		let result = vrf_result.random(0..=100)?;
    /// 	assert!(0 <= result && result <= 100);
    /// ```
    pub fn random<Int>(self, range: RangeInclusive<Int>) -> anchor_lang::Result<Int>
    where
        Int: PrimInt + AsPrimitive<i128>,
        i128: AsPrimitive<Int>,
    {
        // compile time assertion that `vrf::VrfAccountData::RESULT_BYTE_LEN`
        // must contains at least 16 bytes
        const _: [(); 0 - !(vrf::RESULT_BYTE_LEN >= 16) as usize] = [];

        // ensure that the vrf has completed
        if &self.result == &[0u8; vrf::RESULT_BYTE_LEN]
            || &self.result == &vrf::VRF_RESULT_DISCRIMINATOR
        {
            return Err(Error::AnchorError(AnchorError {
                error_name: "VrfNotFulfilled".to_owned(),
                error_code_number: 7777,
                error_msg: "vrf_sdk::random() called on an empty VrfState".to_owned(),
                error_origin: None,
                compared_values: None,
            }));
        }

        // convert the first 16 byte from the result to an i128
        // we assert at compile time that the result contains at least 16 bytes, so unwrap is ok
        let rand = i128::from_be_bytes(self.result[0..16].try_into().unwrap());

        // apply the required range
        let bound: i128 = (*range.end() - *range.start()).as_();
        let out = ((rand % bound) + range.start().as_()).as_();
        Ok(out)
    }
}