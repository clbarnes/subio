# subio

Implementing IO traits on regions of IO-compatible types.

## Use case

It is often convenient to read, write, and seek within a portion of a file as if it were itself a standalone file.
For example, a tar or zip archive, which are a number of file entries concatenated together with some metadata.

`subio` does the (trivial) bookkeeping to allow that to happen, for standard traits [std::io::Read], [std::io::Write], [std::io::Seek] etc..

Note that the `Read` side of this crate is not much different to [std::io::Take]; you should probably use that.

## Extension

The provided types could be extended to support other IO traits like those provided by various async runtimes and io_uring.
