use std::{str::FromStr, sync::Arc};

use anchor_client::solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::RpcLogsResponse,
};
use backoff::ExponentialBackoff;
use futures_util::StreamExt;
use vrf_sdk::__private::Pubkey;

use crate::{process::process, process_old_trans::process_old_transaction};

mod config;
mod parse_logs;
mod process;
mod process_old_trans;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config: crate::config::Config = ::config::Config::builder()
        .add_source(::config::File::with_name("vrf-server.toml"))
        .add_source(::config::Environment::with_prefix("VRF"))
        .build()?
        .try_deserialize()?;

    let config = Arc::new(crate::config::VrfConfig::try_from(config)?);

    println!("---");
    println!("Running VRF handler with:");
    println!("Cluster: ({}) {}", &config.cluster, config.cluster.url());
    println!("Commitment: {}", &config.commitment.commitment);
    println!("---");

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        config.cluster.url().to_string(),
        config.commitment,
    ));

    tokio::spawn(process_old_transaction(config.clone(), rpc_client.clone()));

    let handles = config
        .program_ids
        .iter()
        .map(|program_id| {
            tokio::spawn(logs_subscribe(
                config.clone(),
                Arc::new(program_id.to_string()),
                rpc_client.clone(),
            ))
        })
        .collect::<Vec<_>>();

    futures_util::future::join_all(handles).await;
    Ok(())
}

pub async fn logs_subscribe(
    config: Arc<crate::config::VrfConfig>,
    program_id: Arc<String>,
    rpc_client: Arc<RpcClient>,
) -> ! {
    let program_id_pubkey =
        Pubkey::from_str(&program_id).expect(&format!("invalid program id: {}", &program_id));

    loop {
        let _ = backoff::future::retry::<Result<(), backoff::Error<()>>, _, _, _, _>(
            ExponentialBackoff::default(),
            || async {
                let pubsub_client = PubsubClient::new(config.cluster.ws_url()).await.unwrap();

                let mut recv_stream = if let Ok((stream, _)) = pubsub_client
                    .logs_subscribe(
                        RpcTransactionLogsFilter::Mentions(vec![(*program_id).clone()]),
                        RpcTransactionLogsConfig {
                            commitment: Some(config.commitment),
                        },
                    )
                    .await
                {
                    stream
                } else {
                    return Err(backoff::Error::Permanent(()));
                };

                tracing::info!("Listening for logs from: {}", &program_id);
                while let Some(response) = recv_stream.next().await {
                    let config = config.clone();
                    let rpc_client = rpc_client.clone();
                    let program_id = program_id.clone();

                    // Spawn a new task to handle the transaction
                    tokio::spawn(async move {
                        let program_id: &str = &program_id;

                        let RpcLogsResponse {
                            signature,
                            err,
                            logs,
                        } = response.value;

                        let span = tracing::info_span!(
                            "Process transaction",
                            program_id,
                            transaction = signature
                        );

                        if let Some(err) = err {
                            span.in_scope(|| {
                                tracing::info!("Skipping error transaction:\n{err:#}")
                            });
                            return;
                        }

                        span.in_scope(|| tracing::info!("Start processing"));
                        match process(&config, &rpc_client, &program_id_pubkey, &span, &logs).await
                        {
                            Ok(_) => {
                                // TODO
                                span.in_scope(|| tracing::info!("Finished!"));
                            }
                            Err(err) => {
                                span.in_scope(|| {
                                    tracing::error!("Error processing transaction:\n{err:#}")
                                });
                            }
                        }
                    });
                }

                tracing::warn!(
                    "Logs subscribe stream ({}) stopped, retrying...",
                    &program_id
                );

                Err(backoff::Error::Transient {
                    err: (),
                    retry_after: None,
                })
            },
        )
        .await;
    }
}
