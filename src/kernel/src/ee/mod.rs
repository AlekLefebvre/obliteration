use std::error::Error;

pub mod llvm;
#[cfg(target_arch = "x86_64")]
pub mod native;

/// An object to execute the PS4 binary.
pub trait ExecutionEngine: Sync {
    /// All execution must be stopped when this method return.
    fn run(&mut self) -> Result<(), Box<dyn Error>>;
}
