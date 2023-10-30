//! Frozen version of Rust standard library's [hashbrown](https://github.com/rust-lang/hashbrown).
//!
//! # What is this about
//!
//! 1. Allow you to dump the memory of a `std::collections::HashMap` into a blob
//! 2. Load the blob and re-construct the hashmap
//! 3. Such that we can iterate through the data!
//!
//! # What is this for
//!
//! 1. It's used in FireDBG to allow us to capture and render `HashMap`
//! 2. It could also probably allow us to extract `HashMap` from `coredump`
//!
//! # How it works
//!
//! Online
//!
//! 1. Construct `TableLayout` for `(K, V)`
//! 2. Extract `ctrl` and `bucket_mask`
//! 3. `calculate_layout_for(buckets)` and calculate the address `NonNull<u8>` and `Layout`
//! 4. Dump the memory into a blob
//!
//! Offline
//!
//! 1. Load the blob into memory
//! 2. Re-construct `hashbrown::map::HashMap` for `(K, V)`
//! 3. Ready to serve
//!
//! # Why does it work
//!
//! 1. The `HashMap` in Rust's standard library is a flat hashmap. Meaning it's only backed by a single contiguous piece of memory.
//! 2. It's dense for small maps and is very memory efficient
//! 3. It's more like a glorified `Vec<(K, V)>` with an index to assist hash key lookup
//!
//! # How to use
//!
//! ```rust
//! use frozen_hashbrown::FrozenHashMap;
//! use std::collections::HashMap;
//!
//! let map: HashMap<char, i32> = [('a', 1), ('b', 2), ('c', 3), ('d', 4)]
//!     .into_iter()
//!     .collect();
//! let snapshot = format!("{map:?}");
//!
//! let frozen = FrozenHashMap::construct(&map);
//! std::mem::drop(map);
//! let frozen: Vec<u8> = frozen.store();
//!
//! let mut unfrozen = FrozenHashMap::load(&frozen).expect("Failed to load");
//! let unfrozen = unfrozen
//!     .reconstruct::<char, i32>()
//!     .expect("Failed to reconstruct");
//! let unfrozen_snapshot = format!("{unfrozen:?}");
//!
//! // even the "random" iteration order holds
//! assert_eq!(snapshot, unfrozen_snapshot);
//! ```
//!
//! More examples in https://github.com/SeaQL/frozen-hashbrown/blob/main/frozen-hashbrown/tests/unfreeze.rs
//!
//! #

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Only support 64-bit platforms");

mod frozen;
mod iter;

pub use frozen::*;
pub use iter::*;

pub struct Group {}

cfg_if::cfg_if! {
    if #[cfg(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri)
    ))] {
        impl Group {
            pub const WIDTH: usize = 16;
        }
    } else if #[cfg(all(target_arch = "aarch64", target_feature = "neon"))] {
        impl Group {
            pub const WIDTH: usize = 8;
        }
    } else {
        // generic
        impl Group {
            pub const WIDTH: usize = 8;
        }
    }
}
