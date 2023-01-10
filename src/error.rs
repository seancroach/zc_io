#![allow(clippy::module_name_repetitions)]

use core::{
    fmt::{self, Formatter},
    result,
};

#[cfg(feature = "std")]
use std::{
    error,
    ffi::NulError,
    io::{self, IntoInnerError},
};

/// A convenient alias for [`io::ErrorKind`].
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
pub type ErrorKind = io::ErrorKind;

/// A specialized [`Result`] type for [`zc_io`].
///
/// This type is used across [`zc_io`] for any operation which may produce an
/// error.
///
/// This typedef is generally used to avoid writing out [`zc_io::Error`]
/// directly and is otherwise a direct mapping to [`Result`].
///
/// While usual Rust style is to import types directly, aliases of [`Result`]
/// often are not, to make it easier to distinguish between them. [`Result`] is
/// generally assumed to be [`core::result::Result`][`Result`], and so users of
/// this alias will generally use `zc_io::Result` instead of shadowing the
/// [prelude]'s import of [`core::result::Result`][`Result`].
///
/// [`Result`]: result::Result
/// [`zc_io`]: crate
/// [`zc_io::Error`]: Error
/// [prelude]: core::prelude
pub type Result<T> = result::Result<T, Error>;

/// The error type for zero-copy I/O operations of the [`Read`] and [`Write`]
/// traits.
///
/// When `std` is enabled this becomes a zero-cost abstraction over
/// [`io::Error`]. In `no_std` environments, error messages are preserved while
/// [`io::ErrorKind`] information is stripped.
///
/// If you are working in an environment that may be `no_std`, and you need to
/// create an [`Error`] yourself, use the [`error!`] macro.
///
/// [`Read`]: crate::Read
/// [`Write`]: crate::Write
/// [`error!`]: crate::error!
pub struct Error {
    #[cfg(feature = "std")]
    inner: io::Error,
    #[cfg(not(feature = "std"))]
    pub(crate) inner: &'static str,
}

/// Constructs a new [`Error`] from an [`io::ErrorKind`] variant identifier and
/// a [`&'static str`].
///
/// In a `std` environment, this macro delegates to [`Error::new`]; in a
/// `no_std` environment, the error kind is elided but the corresponding
/// message is preserved.
///
/// [`&'static str`]: prim@str
///
/// # Examples
///
/// In a `std` environment:
///
/// ```
/// use zc_io::error;
///
/// let my_error = error!(Other, "miscellaneous user error");
///
/// // ErrorKind inspection only works when `std` is available:
/// #[cfg(feature = "std")]
/// assert_eq!(my_error.kind(), zc_io::ErrorKind::Other);
///
/// assert_eq!(my_error.to_string(), "miscellaneous user error");
/// ```
#[macro_export]
macro_rules! error {
    ($variant:ident, $message:literal) => {
        $crate::__error_impl!($variant, $message)
    };
}

#[cfg(feature = "std")]
#[doc(hidden)]
#[macro_export]
macro_rules! __error_impl {
    ($variant:ident, $message:literal) => {
        $crate::Error::new($crate::ErrorKind::$variant, $message)
    };
}

#[cfg(not(feature = "std"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __error_impl {
    ($variant:ident, $message:literal) => {
        $crate::Error::__const_error($message)
    };
}

#[cfg(not(feature = "std"))]
impl Error {
    #[doc(hidden)]
    #[must_use]
    pub const fn __const_error(message: &'static str) -> Error {
        Error { inner: message }
    }
}

#[cfg(feature = "std")]
impl Error {
    /// Creates a new I/O error from a known kind of error as well as an
    /// arbitrary error payload.
    ///
    /// For more information, refer to [`io::Error::new`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        io::Error::new(kind, error).into()
    }

    /// Creates a new I/O error from an arbitrary error payload.
    ///
    /// For more information, refer to [`io::Error::other`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn other<E>(error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        io::Error::new(ErrorKind::Other, error).into()
    }

    /// Returns an error representing the last OS error which occurred.
    ///
    /// For more information, refer to [`io::Error::last_os_error`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn last_os_error() -> Error {
        io::Error::last_os_error().into()
    }

    /// Creates a new instance of an Error from a particular OS error code.
    ///
    /// For more information, refer to [`io::Error::from_raw_os_error`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn from_raw_os_error(code: i32) -> Error {
        let inner = io::Error::from_raw_os_error(code);
        Error { inner }
    }

    /// Returns the OS error that this error represents (if any).
    ///
    /// For more information, refer to [`io::Error::raw_os_error`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.inner.raw_os_error()
    }

    /// Returns a reference to the inner error wrapped by this error (if any).
    ///
    /// For more information, refer to [`io::Error::get_ref`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        self.inner.get_ref()
    }

    /// Returns a mutable reference to the inner error wrapped by this error
    /// (if any).
    ///
    /// For more information, refer to [`io::Error::get_mut`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        self.inner.get_mut()
    }

    /// Consumes the Error, returning its inner error (if any).
    ///
    /// For more information, refer to [`io::Error::into_inner`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline]
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        self.inner.into_inner()
    }

    /// Returns the corresponding [`ErrorKind`] for this error.
    ///
    /// For more information, refer to [`io::Error::kind`].
    #[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
    #[must_use]
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.inner.kind()
    }
}

impl fmt::Debug for Error {
    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.inner)
            .finish()
    }

    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Display for Error {
    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.inner)
    }

    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.inner.source()
    }
}

/// Converts an [`io::Error`] to a [`zc_io::Error`](Error).
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl From<io::Error> for Error {
    fn from(inner: io::Error) -> Self {
        Error { inner }
    }
}

/// Converts a [`zc_io::Error`](Error) to an [`io::Error`].
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl From<Error> for io::Error {
    #[inline]
    fn from(error: Error) -> Self {
        error.inner
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        io::Error::from(kind).into()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl<W> From<IntoInnerError<W>> for Error {
    #[inline]
    fn from(error: IntoInnerError<W>) -> Self {
        io::Error::from(error).into()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl From<NulError> for Error {
    #[inline]
    fn from(error: NulError) -> Self {
        io::Error::from(error).into()
    }
}
