use crate::Result;
use std::io::{Cursor, Seek};
use byteorder::{LittleEndian, ReadBytesExt};
use tracing::error;

pub fn get_chunk_index(chunk_idx_buffer: &[u8], is_headers: bool) -> Result<usize> {
    let mut cursor = Cursor::new(chunk_idx_buffer);
    let index = if is_headers {
        cursor.read_u32::<LittleEndian>()
    } else {
        cursor.seek(std::io::SeekFrom::Start(4))?;
        cursor.read_u32::<LittleEndian>()
    };

    match index {
        Ok(idx) => Ok(idx as usize),
        Err(e) => {
            error!(target: "bluetooth", "Failed to read chunk index: {}", e);
            Ok(0) // Default to 0 if we can't read the index
        }
    }
}