use std::cell::RefCell;

use anchor_client::{
    anchor_lang::{AnchorDeserialize, Discriminator},
    solana_client::{
        client_error::ClientErrorKind,
        nonblocking::rpc_client::RpcClient,
        rpc_request::{RpcError, RpcResponseErrorData},
        rpc_response::RpcSimulateTransactionResult,
    },
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::Signer,
        transaction::{Transaction, TransactionError},
    },
};
use anyhow::Context;
use backoff::{backoff::Backoff, ExponentialBackoff};
use once_cell::unsync::Lazy;
use vrf::{
    openssl::{CipherSuite, ECVRF},
    VRF,
};
use vrf_sdk::{
    __private::Pubkey,
    vrf::{VrfAccountData, VrfRequestRandomness, RESULT_BYTE_LEN},
};

use crate::{config::VrfConfig, parse_logs::parse_logs};

thread_local! {
    static VRF: RefCell<Lazy<ECVRF>> = RefCell::new(Lazy::new(|| {
        ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap()
    }));
}

pub struct VrfResponse {
    pub response_transaction: String,
    pub seeds: Vec<u8>,
    pub proof: Vec<u8>,
}

pub async fn process<S: AsRef<str>>(
    config: &VrfConfig,
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    span: &tracing::Span,
    logs: &[S],
) -> anyhow::Result<Option<VrfResponse>> {
    let (events, errors) = parse_logs(&logs, &config.program_ids);

    if !errors.is_empty() {
        return Err(anyhow::Error::msg(
            errors
                .into_iter()
                .map(|err| format!("{err:?}\n"))
                .collect::<String>(),
        ));
    }

    let event = {
        let event = events
            .into_iter()
            .filter(|event| VrfRequestRandomness::discriminator() == event.data[0..8])
            .next();

        match event {
            Some(event) => event,
            None => return Ok(None),
        }
    };

    if &event.program_id != program_id {
        return Err(anyhow::anyhow!("program_id not match"));
    }

    let request_vrf = VrfRequestRandomness::deserialize(&mut &event.data[8..])
        .context("Deserialize RequestVrf Event")?;

    let vrf_account_data = rpc_client.get_account_data(&request_vrf.vrf).await?;
    if vrf_account_data[0..8] != VrfAccountData::DISCRIMINATOR {
        return Err(anyhow::anyhow!("invalid discriminator"));
    }

    let vrf_account_data: &VrfAccountData =
        bytemuck::from_bytes(&vrf_account_data[8..std::mem::size_of::<VrfAccountData>() + 8]);

    let (proof, random) = {
        let (proof, hash) = VRF.with(|vrf| {
            let mut vrf = vrf.borrow_mut();
            let proof = vrf
                .prove(&config.vrf_secret, &vrf_account_data.seeds)
                .unwrap();
            let hash = vrf.proof_to_hash(&proof).unwrap();
            (proof, hash)
        });

        let mut random = [0u8; RESULT_BYTE_LEN];
        random.copy_from_slice(&hash[..RESULT_BYTE_LEN]);
        (proof, random)
    };

    span.in_scope(|| tracing::info!("Random value: {:?}", &random));

    let mut trans = {
        let cb = vrf_account_data.callback;

        let mut ix_data = cb.ix_data[0..cb.ix_data_len as usize].to_vec();
        if let Some((offset, _)) = ix_data
            .windows(RESULT_BYTE_LEN)
            .enumerate()
            .find(|(_, slice)| slice == &vrf_sdk::vrf::VRF_RESULT_DISCRIMINATOR)
        {
            if offset != 8 {
                span.in_scope(|| {
                    tracing::warn!(
                        "VrfResult maybe not the first parameters, offset={}",
                        offset
                    )
                });
            }

            ix_data[offset..offset + RESULT_BYTE_LEN].copy_from_slice(&random);
        } else {
            return Err(anyhow::anyhow!("cannot found VrfResult in ix_data"));
        }

        let instruction = Instruction {
            program_id: *program_id,
            data: ix_data,
            accounts: cb.accounts[0..cb.accounts_len as usize]
                .iter()
                .map(|acc| AccountMeta {
                    pubkey: acc.pubkey,
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
                .collect(),
        };

        let latest_hash = rpc_client.get_latest_blockhash().await?;
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&config.signer.pubkey()),
            &[&config.signer],
            latest_hash,
        )
    };

    let mut backoff = ExponentialBackoff::default();
    loop {
        span.in_scope(|| tracing::info!("Sending request..."));
        match rpc_client.send_and_confirm_transaction(&trans).await {
            Ok(signature) => {
                return Ok(Some(VrfResponse {
                    response_transaction: signature.to_string(),
                    seeds: vrf_account_data.seeds.to_vec(),
                    proof: proof.clone(),
                }))
            }
            Err(err) => match err.kind() {
                ClientErrorKind::RpcError(RpcError::RpcResponseError { data, .. }) => {
                    if let RpcResponseErrorData::SendTransactionPreflightFailure(
                        RpcSimulateTransactionResult {
                            logs: Some(logs), ..
                        },
                    ) = data
                    {
                        let mut errors = "Simulation error logs:".to_string();
                        for log in logs {
                            errors.push('\t');
                            errors.push_str(log);
                            errors.push('\n');
                        }

                        return Err(err).context(errors);
                    }

                    Err(err)?
                }
                ClientErrorKind::TransactionError(TransactionError::BlockhashNotFound)
                | ClientErrorKind::TransactionError(TransactionError::AlreadyProcessed) => {
                    let new_blockhash = rpc_client
                        .get_new_latest_blockhash(&trans.message.recent_blockhash)
                        .await;

                    if let Ok(new_blockhash) = new_blockhash {
                        trans.message.recent_blockhash = new_blockhash;
                    }
                }
                _ => return Err(err)?,
            },
        }

        match backoff.next_backoff() {
            Some(duration) => tokio::time::sleep(duration).await,
            None => return Err(anyhow::anyhow!("Send transaction failed!")),
        }
    }
}
