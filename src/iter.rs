use crate::FrozenHashMap;

/// An iterator that yields raw pointers to buckets
pub struct RawBucketIter<'a> {
    base: *const u8,
    cur: *const u8,
    end: *const u8,
    bucket_size: usize,
    items: usize,
    _memory: &'a [u8],
}

impl<S> FrozenHashMap<S> {
    pub fn raw_iter(&self) -> Option<RawBucketIter<'_>> {
        if let Some((offset, layout)) = self.hashmap.table.table.reallocation(&self.table_layout) {
            if self.memory.is_empty() {
                return None;
            }
            if layout.size() != self.memory.len() {
                return None;
            }
            let base = unsafe { self.memory.as_ptr().add(offset) };
            Some(RawBucketIter {
                base,
                cur: base,
                end: unsafe { self.memory.as_ptr().add(self.memory.len()) },
                bucket_size: self.table_layout.size,
                items: self.hashmap.table.table.items,
                _memory: &self.memory,
            })
        } else {
            None
        }
    }
}

impl<'a> Iterator for RawBucketIter<'a> {
    /// memory address of the bucket
    type Item = *const u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items == 0 {
            return None;
        }
        while self.cur < self.end {
            // most significant bit = 0 means bucket is full
            let full = (unsafe { *self.cur } & 0x80) == 0;
            self.cur = unsafe { self.cur.add(1) };
            if full {
                let offset = unsafe { self.cur.offset_from(self.base) } * self.bucket_size as isize;
                assert!(offset >= 0);
                self.items -= 1;
                return Some(unsafe { self.base.sub(offset as usize) });
            }
        }
        return None;
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items, Some(self.items))
    }
}
