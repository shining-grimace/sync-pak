//! Portable definitions and device bindings. Transfer code receives resolved snapshots.
mod model;
mod paths;
mod storage;
mod transfer;
mod validation;

pub use model::*;
pub use paths::{default_local_root, local_path, relative_path};
pub(crate) use storage::{decode, encode};
pub use transfer::{ConflictChoice, export_list, import_list, portable, provider_candidates};

#[cfg(test)]
pub(crate) mod tests;
