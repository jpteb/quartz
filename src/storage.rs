use std::ptr::NonNull;

#[derive(Debug)]
pub(crate) struct BlobVec {
    data: NonNull<u8>,
}

impl BlobVec {
    pub fn new() -> Self {
        Self {
            data: NonNull::dangling(),
        }
    }
}
