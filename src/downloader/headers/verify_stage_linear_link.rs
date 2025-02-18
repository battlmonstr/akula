use crate::{
    downloader::headers::{
        header_slice_status_watch::HeaderSliceStatusWatch,
        header_slice_verifier,
        header_slices::{HeaderSlice, HeaderSliceStatus, HeaderSlices},
    },
    models::{BlockHeader, BlockNumber},
};
use anyhow::anyhow;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::{ops::DerefMut, sync::Arc, time::SystemTime};
use tracing::*;

/// Verifies the sequence rules to link the slices with the last known verified header and sets Verified status.
pub struct VerifyStageLinearLink {
    header_slices: Arc<HeaderSlices>,
    start_block_num: BlockNumber,
    start_block_hash: ethereum_types::H256,
    last_verified_header: Option<BlockHeader>,
    pending_watch: HeaderSliceStatusWatch,
    remaining_count: usize,
}

impl VerifyStageLinearLink {
    pub fn new(
        header_slices: Arc<HeaderSlices>,
        start_block_num: BlockNumber,
        start_block_hash: ethereum_types::H256,
    ) -> Self {
        Self {
            header_slices: header_slices.clone(),
            start_block_num,
            start_block_hash,
            last_verified_header: None,
            pending_watch: HeaderSliceStatusWatch::new(
                HeaderSliceStatus::VerifiedInternally,
                header_slices,
                "VerifyStageLinearLink",
            ),
            remaining_count: 0,
        }
    }

    pub async fn execute(&mut self) -> anyhow::Result<()> {
        debug!("VerifyStageLinearLink: start");

        // initially remaining_count = 0, so we wait for any internally verified slices to try to link them
        // since we want to link sequentially, there might be some remaining slices
        // in this case we wait until some more slices become internally verified
        // hopefully its the slices at the front so that we can link them too
        self.pending_watch.wait_while(self.remaining_count).await?;

        let pending_count = self.pending_watch.pending_count();

        debug!("VerifyStageLinearLink: verifying {} slices", pending_count);
        let updated_count = self.verify_pending_monotonic(pending_count)?;
        debug!("VerifyStageLinearLink: updated {} slices", updated_count);

        self.remaining_count = pending_count - updated_count;

        debug!("VerifyStageLinearLink: done");
        Ok(())
    }

    fn verify_pending_monotonic(&mut self, pending_count: usize) -> anyhow::Result<usize> {
        let mut updated_count: usize = 0;
        for i in 0..pending_count {
            let slice_lock = self.header_slices.find_by_index(i).ok_or_else(|| {
                anyhow!(
                    "VerifyStageLinearLink: inconsistent state - less pending slices than expected"
                )
            })?;
            let is_updated = self.verify_pending_slice(slice_lock);
            if is_updated.is_some() {
                updated_count += 1;
            }
            let is_verified = is_updated.unwrap_or(false);
            if !is_verified {
                break;
            }
        }
        Ok(updated_count)
    }

    fn verify_pending_slice(&mut self, slice_lock: Arc<RwLock<HeaderSlice>>) -> Option<bool> {
        let slice = slice_lock.upgradable_read();
        let is_verified_internally = slice.status == HeaderSliceStatus::VerifiedInternally;
        if !is_verified_internally {
            return None;
        }

        let is_verified = self.verify_slice_link(&slice, &self.last_verified_header);

        let mut slice = RwLockUpgradableReadGuard::upgrade(slice);
        if is_verified {
            self.header_slices
                .set_slice_status(slice.deref_mut(), HeaderSliceStatus::Verified);
            if let Some(last_verified_header) = slice.headers.as_ref().unwrap().iter().last() {
                self.last_verified_header = Some(last_verified_header.clone());
            }
        } else {
            self.header_slices
                .set_slice_status(slice.deref_mut(), HeaderSliceStatus::Invalid);
        }

        Some(is_verified)
    }

    fn now_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn verify_slice_link(&self, slice: &HeaderSlice, parent: &Option<BlockHeader>) -> bool {
        if slice.headers.is_none() {
            return false;
        }
        let headers = slice.headers.as_ref().unwrap();
        if headers.is_empty() {
            return false;
        }
        let child = &headers[0];

        // for the start header we just verify its hash
        if child.number == self.start_block_num {
            return child.hash() == self.start_block_hash;
        }
        // otherwise we expect that we have a verified parent
        if parent.is_none() {
            return false;
        }
        let parent = parent.as_ref().unwrap();

        header_slice_verifier::verify_link_by_parent_hash(child, parent)
            && header_slice_verifier::verify_link_block_nums(child, parent)
            && header_slice_verifier::verify_link_timestamps(child, parent)
            && header_slice_verifier::verify_link_difficulties(child, parent)
            && header_slice_verifier::verify_link_pow(child, parent)
    }
}

#[async_trait::async_trait]
impl super::stage::Stage for VerifyStageLinearLink {
    async fn execute(&mut self) -> anyhow::Result<()> {
        VerifyStageLinearLink::execute(self).await
    }
}
