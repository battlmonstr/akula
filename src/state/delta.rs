use super::{intra_block_state::IntraBlockState, object::Object};
use crate::{State, Storage};
use ethereum_types::*;
use std::fmt::Debug;

/// Reversible change made to `IntraBlockState`.
#[derive(Debug)]
pub enum Delta {
    Create {
        address: Address,
    },
    Update {
        address: Address,
        previous: Object,
    },
    UpdateBalance {
        address: Address,
        previous: U256,
    },
    Selfdestruct {
        address: Address,
    },
    Touch {
        address: Address,
    },
    StorageChange {
        address: Address,
        key: H256,
        previous: H256,
    },
    StorageWipe {
        address: Address,
        storage: Storage,
    },
    StorageCreate {
        address: Address,
    },
    StorageAccess {
        address: Address,
        key: H256,
    },
    AccountAccess {
        address: Address,
    },
}

impl Delta {
    pub fn revert<R>(self, state: &mut IntraBlockState<'_, R>)
    where
        R: State,
    {
        match self {
            Delta::Create { address } => {
                state.objects.remove(&address);
            }
            Delta::Update { address, previous } => {
                state.objects.insert(address, previous);
            }
            Delta::UpdateBalance { address, previous } => {
                state
                    .objects
                    .get_mut(&address)
                    .unwrap()
                    .current
                    .as_mut()
                    .unwrap()
                    .balance = previous;
            }
            Delta::Selfdestruct { address } => {
                state.self_destructs.remove(&address);
            }
            Delta::Touch { address } => {
                state.touched.remove(&address);
            }
            Delta::StorageChange {
                address,
                key,
                previous,
            } => {
                state
                    .storage
                    .entry(address)
                    .or_default()
                    .current
                    .insert(key, previous);
            }
            Delta::StorageWipe { address, storage } => {
                state.storage.insert(address, storage);
            }
            Delta::StorageCreate { address } => {
                state.storage.remove(&address);
            }
            Delta::StorageAccess { address, key } => {
                state
                    .accessed_storage_keys
                    .entry(address)
                    .or_default()
                    .remove(&key);
            }
            Delta::AccountAccess { address } => {
                state.accessed_addresses.remove(&address);
            }
        }
    }
}
