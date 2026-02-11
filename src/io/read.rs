use super::{map_seek_oob, seek_oob};
use std::{
    io::{self, BufRead, Read, Seek, SeekFrom},
    ops::Deref,
};

/// Implements [Read] and [Seek] for a portion of the inner type,
/// where that inner type implements those traits.
///
/// Note that while [BufRead] is also supported,
/// users should prefer to wrap a SubReader in a [std::io::BufReader] rather than the other way round,
/// to avoid inconsistent behaviour when the inner buffer reads beyond the end of the subreader.
#[derive(Debug, Clone)]
pub struct SubReader<R> {
    inner: R,
    start: u64,
    end: u64,
    pos: u64,
}

impl<R: Read> Read for SubReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.end {
            return Ok(0);
        }
        let max_read = (self.end - self.pos) as usize;
        let to_read = std::cmp::min(buf.len(), max_read);
        let bytes_read = self.inner.read(&mut buf[..to_read])?;
        self.pos += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl<R> Deref for SubReader<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R> SubReader<R> {
    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn inner_stream_position(&self) -> u64 {
        self.pos
    }

    pub fn new_unchecked(inner: R, pos: u64, length: u64) -> Self {
        let start = pos;
        let end = start + length;
        SubReader {
            inner,
            start,
            end,
            pos,
        }
    }
}

impl<R: Read + Seek> SubReader<R> {
    /// Creates a new SubReader starting at the current position of `inner` and spanning `length` bytes.
    ///
    /// Still seeks to find the current position.
    pub fn new_from(inner: R, length: u64) -> io::Result<Self> {
        Self::new_seek(inner, SeekFrom::Current(0), length)
    }

    /// Creates a new SubReader starting at the give seek position of `inner` and spanning `length` bytes.
    pub fn new_seek(mut inner: R, start: SeekFrom, length: u64) -> io::Result<Self> {
        let start = inner.seek(start)?;
        Ok(Self::new_unchecked(inner, start, length))
    }
}

impl<R: BufRead> BufRead for SubReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.end {
            return Ok(&[]);
        }
        let buf = self.inner.fill_buf()?;
        let max_len = (self.end - self.pos) as usize;
        Ok(&buf[..std::cmp::min(buf.len(), max_len)])
    }

    fn consume(&mut self, amt: usize) {
        let remaining = self.end.saturating_sub(self.pos);
        let to_read = amt.min(remaining as usize);
        self.inner.consume(to_read);
        self.pos += to_read as u64;
    }
}

impl<R: Seek> Seek for SubReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => self.start + offset,
            SeekFrom::End(offset) => map_seek_oob(self.end.checked_add_signed(offset))?,
            SeekFrom::Current(offset) => map_seek_oob(self.pos.checked_add_signed(offset))?,
        };
        if new_pos < self.start {
            seek_oob()?;
        }
        let Some(relative) = (new_pos as i64).checked_sub(self.pos as i64) else {
            return seek_oob();
        };
        // delegating to relative improves performance for BufReader
        self.inner.seek_relative(relative)?;
        self.pos = new_pos;
        self.stream_position()
    }

    /// Infallible.
    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self.pos - self.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_subreader() {
        let data: Vec<u8> = (0..10).collect();
        let cursor = Cursor::new(data);
        let mut subreader = SubReader::new_seek(cursor, SeekFrom::Start(3), 5).unwrap();
        let mut buf = Vec::default();
        subreader.read_to_end(&mut buf).unwrap();
        assert_eq!(&buf, &[3, 4, 5, 6, 7]);
    }
}
