use super::chain_id::ChainId;
use crate::{models::BlockNumber, state::genesis::GenesisData};
use hex_literal::hex;
use std::collections::HashMap;

pub struct ChainsConfig(HashMap<String, ChainConfig>);

#[derive(Clone)]
pub struct ChainConfig {
    genesis: &'static GenesisData,
    genesis_block_hash: ethereum_types::H256,
}

impl ChainConfig {
    fn new(genesis: &'static GenesisData, genesis_block_hash: ethereum_types::H256) -> Self {
        // TODO: calculate hash from GenesisData
        // let genesis_header = genesis.header(&genesis.initial_state());
        // let genesis_block_hash = genesis_header.hash();
        Self {
            genesis,
            genesis_block_hash,
        }
    }

    pub fn id(&self) -> ChainId {
        self.genesis.config.chain_id
    }

    pub fn genesis_block_hash(&self) -> ethereum_types::H256 {
        self.genesis_block_hash
    }

    pub fn fork_block_numbers(&self) -> Vec<BlockNumber> {
        self.genesis.config.gather_forks().iter().cloned().collect()
    }
}

impl ChainsConfig {
    pub fn new() -> anyhow::Result<Self> {
        let mut configs = HashMap::<String, ChainConfig>::new();
        configs.insert(
            String::from("mainnet"),
            ChainConfig::new(
                &crate::res::genesis::MAINNET,
                ethereum_types::H256(hex!(
                    "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3"
                )),
            ),
        );
        configs.insert(
            String::from("ropsten"),
            ChainConfig::new(
                &crate::res::genesis::ROPSTEN,
                ethereum_types::H256(hex!(
                    "41941023680923e0fe4d74a34bdac8141f2540e3ae90623718e47d66d1ca4a2d"
                )),
            ),
        );
        Ok(ChainsConfig(configs))
    }

    pub fn get(&self, chain_name: &str) -> Option<&ChainConfig> {
        self.0.get(chain_name)
    }
}

impl ChainsConfig {
    pub fn chain_names(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect::<Vec<&str>>()
    }
}
