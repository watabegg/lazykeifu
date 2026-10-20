//! Git extension configuration for libgit2.

use anyhow::{Context, Result};

/// Configure extra repository extensions that libgit2 should accept, and
/// skip re-hashing every object read (git doesn't either; the SHA-1
/// collision-detecting hash was ~50% of the time spent loading commit diffs).
///
/// This must run before opening repositories.
pub fn configure_git_extensions() -> Result<()> {
    git2::opts::strict_hash_verification(false);
    unsafe { git2::opts::set_extensions(&["relativeworktrees"]) }.context(crate::tr!(
        "failed to configure libgit2 supported extensions (relativeworktrees)"
    ))
}
