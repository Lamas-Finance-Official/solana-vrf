use crate::*;
use solana_program::{instruction::Instruction, program::invoke};

#[derive(Accounts)]
pub struct FulfillRandomness<'info> {
    #[account(address = vrf.load()?.signer)]
    pub signer: Signer<'info>,

    #[account(constraint = *vrf.to_account_info().owner == crate::id())]
    pub vrf: AccountLoader<'info, VrfAccountData>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct FulfillRandomnessParams {
    pub result: Vec<u8>,
    pub proof: Vec<u8>,
}

impl FulfillRandomness<'_> {
    pub fn execute<'info>(
        ctx: &Context<'_, '_, '_, 'info, FulfillRandomness<'info>>,
        params: &FulfillRandomnessParams,
    ) -> anchor_lang::Result<()> {
        let signer_account_info: AccountInfo<'info> = ctx.accounts.signer.to_account_info();
        let vrf_account_info: AccountInfo<'info> = ctx.accounts.vrf.to_account_info();
        let vrf = &mut ctx.accounts.vrf.load_mut()?;

        require!(
            ctx.accounts.signer.key() == vrf.signer,
            VrfError::InvalidSigner
        );

        vrf.proof.copy_from_slice(&params.proof);
        vrf.result.copy_from_slice(&params.result);

        let instruction = Instruction::new_with_bytes(
            vrf.callback.program_id,
            &vrf.callback.ix_data[0..vrf.callback.ix_data_len as usize],
            vrf.callback.accounts[0..vrf.callback.accounts_len as usize]
                .iter()
                .map(|meta| AccountMeta {
                    pubkey: meta.pubkey,
                    is_signer: meta.is_signer,
                    is_writable: meta.is_writable,
                })
                .collect(),
        );

        let account_infos = [
            &[signer_account_info, vrf_account_info],
            ctx.remaining_accounts,
        ]
        .concat();

        drop(vrf);
        invoke(&instruction, &account_infos)?;
        Ok(())
    }
}
