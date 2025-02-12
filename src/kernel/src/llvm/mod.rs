use self::module::LlvmModule;
use llvm_sys::core::{LLVMContextCreate, LLVMContextDispose, LLVMModuleCreateWithNameInContext};
use llvm_sys::prelude::LLVMContextRef;
use std::ffi::{c_char, CStr, CString};
use std::fmt::Display;
use std::sync::Mutex;

pub mod module;

/// A LLVM wrapper for thread-safe.
pub struct Llvm {
    context: Mutex<LLVMContextRef>,
}

impl Llvm {
    pub(super) fn new() -> Self {
        let context = unsafe { LLVMContextCreate() };

        Self {
            context: Mutex::new(context),
        }
    }

    pub fn create_module(&self, name: &str) -> LlvmModule<'_> {
        let context = self.context.lock().unwrap();
        let name = CString::new(name).unwrap();
        let module = unsafe { LLVMModuleCreateWithNameInContext(name.as_ptr(), *context) };

        LlvmModule::new(self, module)
    }

    fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(LLVMContextRef) -> R,
    {
        f(*self.context.lock().unwrap())
    }
}

impl Drop for Llvm {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(*self.context.get_mut().unwrap()) };
    }
}

unsafe impl Sync for Llvm {}

/// A wrapper on LLVM error.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    /// # Safety
    /// `message` must be pointed to a null-terminated string allocated with `malloc` or a
    /// compatible funtion because this method will free it with `free`.
    unsafe fn new(message: *mut c_char) -> Self {
        let owned = CStr::from_ptr(message)
            .to_string_lossy()
            .trim_end_matches('.')
            .to_owned();

        libc::free(message as _);

        Self { message: owned }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}
