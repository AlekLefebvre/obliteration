// This file contains error codes used in a PS4 system. The value of each error must be the same as
// the PS4.
use std::error::Error;
use std::num::NonZeroI32;

pub const EPERM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(1) };
pub const ENOENT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(2) };
pub const ESRCH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(3) };
pub const E2BIG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(7) };
pub const ENOEXEC: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(8) };
pub const ENOMEM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(12) };
pub const EFAULT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(14) };
pub const EINVAL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(22) };
pub const EAGAIN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(35) };
pub const ENAMETOOLONG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(63) };

/// An object that is mappable to PS4 errno.
pub trait Errno: Error {
    fn errno(&self) -> NonZeroI32;
}

/// Get human readable text.
pub fn strerror(num: NonZeroI32) -> &'static str {
    match num {
        EPERM => "operation not permitted",
        ENOENT => "no such file or directory",
        ESRCH => "no such process",
        E2BIG => "argument list too long",
        ENOEXEC => "exec format error",
        ENOMEM => "cannot allocate memory",
        EFAULT => "bad address",
        EINVAL => "invalid argument",
        EAGAIN => "resource temporarily unavailable",
        ENAMETOOLONG => "file name too long",
        v => todo!("strerror {v}"),
    }
}
