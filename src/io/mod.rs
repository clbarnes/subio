//! Sub-readers and writers for [std::io].
mod read;
mod write;
use std::io;

pub use read::SubReader;
pub use write::SubWriter;

fn map_seek_oob(maybe_pos: Option<u64>) -> io::Result<u64> {
    match maybe_pos {
        Some(pos) => Ok(pos),
        None => seek_oob(),
    }
}

fn seek_oob<T>() -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "Seek position out of bounds",
    ))
}
