use crate::*;
use solana_program::sysvar;

#[derive(Accounts)]
#[instruction(params: RequestRandomnessParams)]
pub struct RequestRandomness<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
		init,
		payer = payer,
		space = std::mem::size_of::<VrfAccountData>(),
		seeds = [
			b"VrfAccountData",
			&params.vrf_pda_seed[..],
		],
		bump
	)]
    pub vrf: AccountLoader<'info, VrfAccountData>,

    /// CHECK:
    #[account(address = sysvar::recent_blockhashes::ID)]
    pub recent_blockhashes: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct RequestRandomnessParams {
    callback: Callback,
    vrf_pda_seed: Vec<u8>,
}

impl RequestRandomness<'_> {
    pub fn execute(
        ctx: &Context<Self>,
        params: &RequestRandomnessParams,
    ) -> anchor_lang::Result<()> {
        let RequestRandomnessParams { callback, .. } = params;

        require!(
            callback.accounts.len() <= 32,
            VrfError::CallbackTooManyAccounts
        );

        require!(
            callback.ix_data.len() <= 1024,
            VrfError::CallbackTooManyInstructionData
        );

        let vrf = &mut ctx.accounts.vrf.load_init()?;
        vrf.status = VrfStatus::Requesting;
        vrf.signer = ctx.accounts.signer.key();
        vrf.request_timestamp = Clock::get()?.unix_timestamp;

        #[allow(deprecated)]
        {
            let recent_blockhashes =
                solana_program::sysvar::recent_blockhashes::RecentBlockhashes::from_account_info(
                    &ctx.accounts.recent_blockhashes,
                )?;

            let most_recent = recent_blockhashes
                .first()
                .expect("Recent block hashes: no recent block hashes");

            vrf.seeds.copy_from_slice(most_recent.blockhash.as_ref());
        }

        vrf.callback.program_id = callback.program_id;
        vrf.callback.accounts_len = callback.accounts.len() as u32;
        for (i, account) in callback.accounts.iter().enumerate() {
            vrf.callback.accounts[i] = AccountMetaPacked {
                pubkey: account.pubkey,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            };
        }

        vrf.callback.ix_data_len = callback.ix_data.len() as u32;
        vrf.callback.ix_data[0..callback.ix_data.len()].copy_from_slice(&callback.ix_data);

        emit!(RequestRandomnessEvent {
            vrf: ctx.accounts.vrf.key()
        });
        Ok(())
    }
}
