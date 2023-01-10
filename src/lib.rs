//! This crate provides a zero-copy [`Read`] trait and a simplified [`Write`]
//! trait useful for possibly `no_std` environments.
//!
//! [`no_std`]: https://docs.rust-embedded.org/book/intro/no-std.html

// zc_io types in rustdoc of other crates get linked to here:
#![doc(html_root_url = "https://docs.rs/serde/1.0.152")]
// Enable https://doc.rust-lang.org/beta/unstable-book/language-features/doc-cfg.html:
#![cfg_attr(doc_cfg, feature(doc_cfg))]
// Support using zc_io without the standard library:
#![cfg_attr(not(feature = "std"), no_std)]
// Enable lints:
#![deny(clippy::pedantic, missing_docs)]

extern crate alloc;

#[macro_use]
mod error;

#[cfg(feature = "std")]
pub use error::ErrorKind;
pub use error::{Error, Result};

use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use core::{cmp, mem};
#[cfg(feature = "std")]
use std::{
    fmt,
    io::{self, IoSlice, IoSliceMut, SeekFrom},
    slice,
};

/// The `Read<'data>` trait allows for reading bytes with a lifetime of `'data`
/// from some source.
///
/// Implementors of the `Read<'data>` trait are called "zero-copy readers".
///
/// Note that not every source will enable zero-copy reads. This is evident by
/// [`read_slice()`] returning a [`Cow`] (such that it might return a [`Vec`] if
/// the bytes requested cannot be borrowed).
///
/// [`read_slice()`]: Read::read_slice
pub trait Read<'data> {
    /// Reads the next byte from the source.
    ///
    /// # Errors
    ///
    /// If this function encounters an error of the kind
    /// [`ErrorKind::Interrupted`] then the error is ignored and the operation
    /// will continue.
    ///
    /// An [`ErrorKind::UnexpectedEof`] error is returned if this reader has
    /// reached end-of-file before the call to this method.
    ///
    /// If any other read error is encountered then this function immediately
    /// returns.
    ///
    /// If this function returns an error, it is unspecified how many bytes got
    /// read.
    fn read_next(&mut self) -> Result<u8>;

    /// Reads `n` bytes from this reader, borrowing bytes if possible.
    ///
    /// As this trait is safe to implement, callers cannot rely on the number of
    /// returned bytes being equal to `n` for safety. Extra care needs to be
    /// taken when `unsafe` functions are used to access the read bytes. Callers
    /// have to ensure that no unchecked out-of-bounds accesses are possible
    /// even if the number of returned bytes are greater or less than `n`.
    ///
    /// # Errors
    ///
    /// If this function encounters an error of the kind
    /// [`ErrorKind::Interrupted`] then the error is ignored and the operation
    /// will continue.
    ///
    /// An [`ErrorKind::UnexpectedEof`] error is returned if this reader has
    /// reached end-of-file before the call to this method.
    ///
    /// If any other read error is encountered then this function immediately
    /// returns.
    ///
    /// If this function returns an error, it is unspecified how many bytes got
    /// read.
    fn read_slice(&mut self, n: usize) -> Result<Cow<'data, [u8]>>;

    /// Reads exactly `N` bytes from this reader.
    ///
    /// Note that because this method returns an array it will always copy the
    /// bytes from the source regardless if zero-copy reads are possible.
    ///
    /// # Errors
    ///
    /// If this function encounters an error of the kind
    /// [`ErrorKind::Interrupted`] then the error is ignored and the operation
    /// will continue.
    ///
    /// An [`ErrorKind::UnexpectedEof`] error is returned if this reader has
    /// reached end-of-file before the call to this method.
    ///
    /// If any other read error is encountered then this function immediately
    /// returns.
    ///
    /// If this function returns an error, it is unspecified how many bytes got
    /// read.
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]>;
}

impl<'data, R> Read<'data> for &mut R
where
    R: Read<'data>,
{
    #[inline]
    fn read_next(&mut self) -> Result<u8> {
        (**self).read_next()
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Result<Cow<'data, [u8]>> {
        (**self).read_slice(len)
    }

    #[inline]
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        (**self).read_array()
    }
}

impl<'data, R> Read<'data> for Box<R>
where
    R: Read<'data>,
{
    #[inline]
    fn read_next(&mut self) -> Result<u8> {
        (**self).read_next()
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Result<Cow<'data, [u8]>> {
        (**self).read_slice(len)
    }

    #[inline]
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        (**self).read_array()
    }
}

impl<'data> Read<'data> for &'data [u8] {
    #[inline]
    fn read_next(&mut self) -> Result<u8> {
        if let Some((&byte, rest)) = self.split_first() {
            *self = rest;
            return Ok(byte);
        }

        Err(error!(UnexpectedEof, "failed to read byte"))
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Result<Cow<'data, [u8]>> {
        if self.len() < len {
            return Err(error!(UnexpectedEof, "failed to read slice"));
        }

        let (slice, rest) = self.split_at(len);
        *self = rest;
        Ok(Cow::Borrowed(slice))
    }

    #[inline]
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.len() < N {
            return Err(error!(UnexpectedEof, "failed to read array"));
        }

        let (array, rest) = self.split_at(N);
        *self = rest;
        // SAFETY: a slice of bytes whose length is `N` is identical to
        // `[u8; N]`.
        Ok(unsafe { *array.as_ptr().cast::<[u8; N]>() })
    }
}

/// The `IoReader<R>` struct implements [`Read<'data>`] to any reader.
///
/// Due to the interface of [`io::Read`], an `IoReader<R>` will never support
/// zero-copy operations, meaning that [`read_slice`] will always return an
/// [`Owned`] value.
///
/// [`Read<'data>`]: Read
/// [`read_slice`]: Read::read_slice
/// [`Owned`]: Cow::Owned
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
pub struct IoReader<R> {
    inner: R,
}

#[cfg(feature = "std")]
impl<R> IoReader<R>
where
    R: io::Read,
{
    /// Creates a new `IoReader<R>` from some reader.
    #[must_use]
    #[inline]
    pub fn new(reader: R) -> Self {
        IoReader { inner: reader }
    }

    /// Gets a reference to the underlying reader.
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Unwraps the `IoReader<R>`, returning the underlying reader.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<'data, R> Read<'data> for IoReader<R>
where
    R: io::Read,
{
    #[inline]
    fn read_next(&mut self) -> Result<u8> {
        let mut byte = 0;
        self.inner.read_exact(slice::from_mut(&mut byte))?;
        Ok(byte)
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Result<Cow<'data, [u8]>> {
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf)?;
        Ok(Cow::Owned(buf))
    }

    #[inline]
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut array = [0; N];
        self.inner.read_exact(&mut array)?;
        Ok(array)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<R> io::Read for IoReader<R>
where
    R: io::Read,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<R> io::BufRead for IoReader<R>
where
    R: io::BufRead,
{
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }

    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_until(byte, buf)
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner.read_line(buf)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<R> io::Seek for IoReader<R>
where
    R: io::Seek,
{
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }

    #[inline]
    fn rewind(&mut self) -> io::Result<()> {
        self.inner.rewind()
    }

    #[inline]
    fn stream_position(&mut self) -> io::Result<u64> {
        self.inner.stream_position()
    }
}

/// A simplified facade of [`io::Write`] for easier use in possibly [`no_std`]
/// environments.
///
/// [`no_std`]: https://docs.rust-embedded.org/book/intro/no-std.html
pub trait Write {
    /// Write a buffer into this writer, returning how many bytes were written.
    ///
    /// This function will attempt to write the entire contents of `buf`, but
    /// the entire write might not succeed, or the write may also generate an
    /// error. A call to `write` represents *at most one* attempt to write to
    /// any wrapped object.
    ///
    /// Calls to `write` are not guaranteed to block waiting for data to be
    /// written, and a write which would otherwise block can be indicated through
    /// an [`Err`] variant.
    ///
    /// If the return value is [`Ok(n)`] then it must be guaranteed that
    /// `n <= buf.len()`. A return value of `0` typically means that the
    /// underlying object is no longer able to accept bytes and will likely not
    /// be able to in the future as well, or that the buffer provided is empty.
    ///
    /// # Errors
    ///
    /// Each call to `write` may generate an I/O error indicating that the
    /// operation could not be completed. If an error is returned then no bytes
    /// in the buffer were written to this writer.
    ///
    /// It is **not** considered an error if the entire buffer could not be
    /// written to this writer.
    ///
    /// An error of the [`ErrorKind::Interrupted`] kind is non-fatal and the
    /// write operation should be retried if there is nothing else to do.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// # Errors
    ///
    /// It is considered an error if not all bytes could be written due to I/O
    /// errors or EOF getting reached.
    fn flush(&mut self) -> Result<()>;

    /// Attempts to write an entire buffer into this writer.
    ///
    /// This method will continuously call [`write`] until there is no more data
    /// to be written or an error of non-[`ErrorKind::Interrupted`] kind is
    /// returned. This method will not return until the entire buffer has been
    /// successfully written or such an error occurs. The first error that is
    /// not of [`ErrorKind::Interrupted`] kind generated from this method will
    /// be returned.
    ///
    /// If the buffer contains no data, this will never call [`write`].
    ///
    /// # Errors
    ///
    /// This function will return the first error of
    /// non-[`ErrorKind::Interrupted`] kind that [`write`] returns.
    ///
    /// [`write`]: Write::write
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(error!(WriteZero, "failed to write whole buffer")),
                Ok(n) => buf = &buf[n..],
                #[cfg(feature = "std")]
                Err(ref error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

impl<W> Write for &mut W
where
    W: Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }
}

impl<W> Write for Box<W>
where
    W: Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }
}

/// Write is implemented for `&mut [u8]` by copying into the slice, overwriting
/// its data.
///
/// Note that writing updates the slice to point to the yet unwritten part.
/// The slice will be empty when it has been completely overwritten.
///
/// If the number of bytes to be written exceeds the size of the slice, write
/// operations will return short writes: ultimately, `Ok(0)`; in this situation,
/// `write_all` returns an error of kind `ErrorKind::WriteZero`.
impl Write for &mut [u8] {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let amount = cmp::min(buf.len(), self.len());
        let (data, tail) = mem::take(self).split_at_mut(amount);
        data.copy_from_slice(&buf[..amount]);
        *self = tail;
        Ok(amount)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        if self.write(buf)? == buf.len() {
            Ok(())
        } else {
            Err(error!(WriteZero, "failed to write whole buffer"))
        }
    }
}

/// Write is implemented for `Vec<u8>` by appending to the vector. The vector
/// will grow as needed.
impl Write for Vec<u8> {
    #[inline]
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.extend_from_slice(data);
        Ok(data.len())
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.extend_from_slice(data);
        Ok(())
    }
}

/// The `IoWriter<W>` struct implements [`Write`] to any I/O writer.
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
pub struct IoWriter<W> {
    inner: W,
}

#[cfg(feature = "std")]
impl<W> IoWriter<W>
where
    W: io::Write,
{
    /// Creates a new `IoWriter<W>` from some writer.
    #[must_use]
    #[inline]
    pub fn new(writer: W) -> Self {
        IoWriter { inner: writer }
    }

    /// Gets a reference to the underlying writer.
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Gets a mutable reference to the underlying writer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Unwraps the `IoWriter<W>`, returning the underlying writer.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<W> Write for IoWriter<W>
where
    W: io::Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let amount = self.inner.write(buf)?;
        Ok(amount)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()?;
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<W> io::Write for IoWriter<W>
where
    W: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.inner.write_fmt(fmt)
    }
}
