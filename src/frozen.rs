use core::{alloc::Layout, ptr::NonNull};
use std::fmt::Debug;

pub const RANDOM_STATE_TYPE_NAME: &str = "std::collections::hash::map::RandomState";
pub const GLOBAL_ALLOC_TYPE_NAME: &str = "alloc::alloc::Global";

#[derive(Clone)]
pub struct FrozenHashMap<S = RandomState> {
    pub table_layout: TableLayout,
    pub hashmap: HashMap<S>,
    pub memory: Vec<u8>,
}

impl<S: Debug> Debug for FrozenHashMap<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrozenHashMap")
            .field("table_layout", &self.table_layout)
            .field("hashmap", &self.hashmap)
            .field(
                "memory",
                &format!("<binary data of size {}>", self.memory.len()),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct HashMap<S = RandomState> {
    pub hash_builder: S,
    pub table: RawTable,
}

#[derive(Debug, Clone)]
pub struct RandomState {
    pub k0: u64,
    pub k1: u64,
}

#[derive(Debug, Clone)]
pub struct RawTable {
    pub table: RawTableInner,
}

#[derive(Debug, Clone)]
pub struct RawTableInner {
    pub bucket_mask: usize,
    pub ctrl: NonNull<u8>,
    pub growth_left: usize,
    pub items: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct TableLayout {
    pub size: usize,
    pub ctrl_align: usize,
}

impl TableLayout {
    pub fn new(layout: Layout) -> Self {
        Self {
            size: layout.size(),
            ctrl_align: if layout.align() > crate::Group::WIDTH {
                layout.align()
            } else {
                crate::Group::WIDTH
            },
        }
    }

    pub fn calculate_layout_for(&self, buckets: usize) -> Option<(Layout, usize)> {
        assert!(buckets.is_power_of_two());

        let TableLayout { size, ctrl_align } = *self;
        // Manual layout calculation since Layout methods are not yet stable.
        let ctrl_offset =
            size.checked_mul(buckets)?.checked_add(ctrl_align - 1)? & !(ctrl_align - 1);
        let len = ctrl_offset.checked_add(buckets + crate::Group::WIDTH)?;

        Some((
            unsafe { Layout::from_size_align_unchecked(len, ctrl_align) },
            ctrl_offset,
        ))
    }
}

impl RawTableInner {
    pub fn allocation(&self, table_layout: &TableLayout) -> Option<(*const u8, Layout)> {
        if self.is_empty_singleton() {
            None
        } else {
            let (layout, ctrl_offset) = table_layout.calculate_layout_for(self.buckets())?;
            Some((unsafe { self.ctrl.as_ptr().sub(ctrl_offset) }, layout))
        }
    }

    pub fn reallocation(&self, table_layout: &TableLayout) -> Option<(usize, Layout)> {
        if self.is_empty_singleton() {
            None
        } else {
            let (layout, ctrl_offset) = table_layout.calculate_layout_for(self.buckets())?;
            Some((ctrl_offset, layout))
        }
    }

    fn buckets(&self) -> usize {
        self.bucket_mask + 1
    }

    fn is_empty_singleton(&self) -> bool {
        self.bucket_mask == 0
    }
}

impl FrozenHashMap<RandomState> {
    pub fn construct<K, V>(hashmap: &std::collections::HashMap<K, V>) -> Self {
        Self::construct_with(
            unsafe {
                core::slice::from_raw_parts(
                    std::mem::transmute(hashmap as *const _),
                    std::mem::size_of::<std::collections::HashMap<K, V>>(),
                )
            },
            TableLayout::new(Layout::new::<(K, V)>()),
        )
    }

    pub fn construct_with(hashmap: &[u8], table_layout: TableLayout) -> Self {
        assert_eq!(std::mem::size_of::<HashMap<RandomState>>(), hashmap.len());
        let hashmap: HashMap<RandomState> =
            unsafe { std::ptr::read_unaligned(hashmap.as_ptr() as *const _) };
        let memory = if let Some((location, layout)) = hashmap.table.table.allocation(&table_layout)
        {
            let location: &[u8] =
                unsafe { core::slice::from_raw_parts(location as *const u8, layout.size()) };
            location.to_vec()
        } else {
            vec![]
        };
        Self {
            table_layout,
            hashmap,
            memory,
        }
    }

    pub fn reconstruct<K, V>(&mut self) -> Option<&std::collections::HashMap<K, V>> {
        assert_eq!(
            std::mem::size_of::<HashMap<RandomState>>(),
            std::mem::size_of::<std::collections::HashMap<K, V>>()
        );
        if self.memory.is_empty() {
            return None;
        }
        if let Some((offset, layout)) = self.hashmap.table.table.reallocation(&self.table_layout) {
            assert_eq!(layout.size(), self.memory.len());
            let address = self.memory.as_ptr() as usize + offset;
            if address == 0 {
                return None;
            }
            self.hashmap.table.table.ctrl = unsafe { NonNull::new_unchecked(address as *mut u8) };
            unsafe {
                // this is the crazy part
                Some(std::mem::transmute(&self.hashmap))
            }
        } else {
            None
        }
    }

    pub fn store(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                std::mem::transmute(&self.table_layout as *const _),
                std::mem::size_of::<TableLayout>(),
            )
        });
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                std::mem::transmute(&self.hashmap as *const _),
                std::mem::size_of::<HashMap<RandomState>>(),
            )
        });
        bytes.extend_from_slice(&self.memory.len().to_ne_bytes());
        bytes.extend_from_slice(&self.memory);
        bytes
    }

    /// None means failed to load
    pub fn load(bytes: &[u8]) -> Option<Self> {
        let mut cursor = 0;
        let chunk = std::mem::size_of::<TableLayout>();
        if cursor + chunk > bytes.len() {
            return None;
        }
        let table_layout: TableLayout =
            unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const _) };
        cursor += chunk;
        let chunk = std::mem::size_of::<HashMap<RandomState>>();
        if cursor + chunk > bytes.len() {
            return None;
        }
        let hashmap: HashMap<RandomState> =
            unsafe { std::ptr::read_unaligned(bytes.as_ptr().add(cursor) as *const _) };
        cursor += chunk;
        let chunk = 8;
        if cursor + chunk > bytes.len() {
            return None;
        }
        let ll = [
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ];
        cursor += chunk;
        let length = usize::from_ne_bytes(ll);
        if cursor + length != bytes.len() {
            return None;
        }
        let memory = bytes[cursor..].to_vec();
        Some(Self {
            table_layout,
            hashmap,
            memory,
        })
    }

    pub fn len(&self) -> usize {
        self.hashmap.len()
    }
}

impl<S> HashMap<S> {
    pub fn len(&self) -> usize {
        self.table.table.items
    }
}
