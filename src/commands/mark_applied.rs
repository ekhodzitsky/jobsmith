//! Mark a tracked application as submitted.

use crate::error::Result;
use crate::profile::store::ProfileStore;

/// Run the mark-applied command.
pub async fn run(store: &ProfileStore, id: i64) -> Result<()> {
    store.mark_applied(id).await?;
    println!("✓ Application {} marked as applied.", id);
    Ok(())
}
