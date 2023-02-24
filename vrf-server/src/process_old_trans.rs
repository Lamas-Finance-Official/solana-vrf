use std::{str::FromStr, sync::Arc};

use anchor_client::{
    solana_client::{
        nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
        rpc_response::RpcConfirmedTransactionStatusWithSignature,
    },
    solana_sdk::{commitment_config::CommitmentConfig, signature::Signature},
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, UiTransactionEncoding, UiTransactionStatusMeta,
};

use crate::config::VrfConfig;

pub async fn process_old_transaction(config: Arc<VrfConfig>, rpc_client: Arc<RpcClient>) {
    let programs = config
        .program_ids
        .iter()
        .map(|program_id| (program_id, program_id.to_string()))
        .collect::<Vec<_>>();

    for (program_pubkey, program_id) in programs.iter() {
        if let Ok(signatures) = rpc_client
            .get_signatures_for_address_with_config(
                program_pubkey,
                GetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: None,
                    commitment: Some(CommitmentConfig::finalized()),
                },
            )
            .await
        {
            let fetched_len = signatures.len();
            let signatures = signatures
                .into_iter()
                .filter(|sig| sig.err.is_none())
                .collect::<Vec<_>>();

            if signatures.is_empty() {
                continue;
            }

            tracing::info!(
                "Process old transaction: processing {} in {} fetched transactions",
                signatures.len(),
                fetched_len
            );

            for trans_sig in signatures {
                let RpcConfirmedTransactionStatusWithSignature { signature, .. } = trans_sig;

                if let Ok(encoded_transaction) = rpc_client
                    .get_transaction(
                        &Signature::from_str(&signature)
                            .expect("invalid signature return from get_signatures"),
                        UiTransactionEncoding::Json,
                    )
                    .await
                {
                    if let Some(UiTransactionStatusMeta {
                        err: None,
                        log_messages: OptionSerializer::Some(logs),
                        ..
                    }) = encoded_transaction.transaction.meta
                    {
                        let span = tracing::info_span!(
                            "Process old transaction",
                            program_id,
                            transaction = signature
                        );

                        if let Err(err) =
                            crate::process(&config, &rpc_client, program_pubkey, &span, &logs).await
                        {
                            span.in_scope(|| {
                                tracing::error!("Error processing old transaction:\n{err:#}")
                            });
                        }

                        span.in_scope(|| tracing::info!("Finished!"));
                    }
                }
            }
        }
    }
}
