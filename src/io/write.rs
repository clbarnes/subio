use super::{map_seek_oob, seek_oob};
use std::{
    io::{self, Seek, SeekFrom, Write},
    ops::Deref,
};

/// Implements [Write] and [Seek] for a portion of the inner type,
/// where that inner type implements those traits.
#[derive(Debug, Clone)]
pub struct SubWriter<W> {
    inner: W,
    start: u64,
    end: u64,
    pos: u64,
    write_beyond: bool,
}

impl<W: Seek> SubWriter<W> {
    /// Creates a new SubWriter starting at the current position of `inner` and spanning `length` bytes.
    ///
    /// Still seeks to find the current position; see [SubWriter::new_unchecked] if you already know the position.
    pub fn new_from(inner: W, length: u64) -> io::Result<Self> {
        Self::new_seek(inner, SeekFrom::Current(0), length)
    }

    /// Creates a new SubWriter starting at the given seek position of `inner` and spanning `length` bytes.
    pub fn new_seek(mut inner: W, start: SeekFrom, length: u64) -> io::Result<Self> {
        let start = inner.seek(start)?;
        Ok(Self::new_unchecked(inner, start, length))
    }
}

impl<W: Write> Write for SubWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let bytes_written = if self.write_beyond {
            self.inner.write(buf)? as u64
        } else {
            if self.pos >= self.end {
                return Ok(0);
            }
            let max_write = (self.end - self.pos) as usize;
            let to_write = buf.len().min(max_write);
            let written = self.inner.write(&buf[..to_write])? as u64;
            if written + self.pos > self.end {
                self.end = self.pos;
            }
            written
        };
        self.pos += bytes_written;
        Ok(bytes_written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> Deref for SubWriter<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W> SubWriter<W> {
    /// Get a reference to the inner writer.
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Consume the SubWriter and return the inner writer.
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Zero-seek constructor; assumes that `pos` is the current position of `inner` and the desired start point.
    pub fn new_unchecked(inner: W, pos: u64, length: u64) -> Self {
        SubWriter {
            inner,
            start: pos,
            end: pos + length,
            pos,
            write_beyond: false,
        }
    }

    /// Defaults to false.
    pub fn write_beyond(mut self, allow: bool) -> Self {
        self.write_beyond = allow;
        self
    }

    /// Get the position of the inner writer.
    pub fn inner_stream_position(&self) -> u64 {
        self.pos
    }
}

impl<W: Seek> Seek for SubWriter<W> {
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
        self.inner.seek_relative(relative)?;
        self.pos = new_pos;
        Ok(self.pos - self.start)
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
    fn test_subwriter() {
        let data: Vec<u8> = (0..10).collect();
        let cursor = Cursor::new(data);
        let mut sub_writer = SubWriter::new_seek(cursor, SeekFrom::Start(5), 3).unwrap();

        assert_eq!(sub_writer.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap(), 3);
        assert_eq!(sub_writer.write(&[1, 2]).unwrap(), 0);

        let result = sub_writer.into_inner().into_inner();
        assert_eq!(&result, &[0, 1, 2, 3, 4, 0, 1, 2, 8, 9]);
    }
}
