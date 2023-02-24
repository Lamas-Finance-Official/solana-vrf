use anchor_client::{
    solana_sdk::{
        commitment_config::{CommitmentConfig, CommitmentLevel},
        pubkey::Pubkey,
        signature::Keypair,
    },
    Cluster,
};
use anyhow::Context;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, serde::Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(rename = "signer-private-key")]
    signer: Vec<u8>,
    #[serde(rename = "vrf-private-key")]
    vrf_secret: Vec<u8>,
    #[serde_as(as = "DisplayFromStr")]
    cluster: Cluster,
    #[serde_as(as = "DisplayFromStr")]
    commitment: CommitmentLevel,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    program_ids: Vec<Pubkey>,
}

#[derive(Debug)]
pub struct VrfConfig {
    pub signer: Keypair,
    pub vrf_secret: Vec<u8>,
    pub cluster: Cluster,
    pub commitment: CommitmentConfig,
    pub program_ids: Vec<Pubkey>,
}

impl TryFrom<Config> for VrfConfig {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let owner =
            Keypair::from_bytes(&config.signer).context("recover owner Keypair from bytes")?;

        let commitment = CommitmentConfig {
            commitment: config.commitment,
        };

        Ok(Self {
            signer: owner,
            vrf_secret: config.vrf_secret,
            cluster: config.cluster,
            commitment,
            program_ids: config.program_ids,
        })
    }
}
