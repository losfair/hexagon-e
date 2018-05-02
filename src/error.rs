#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum ExecuteError {
    Generic = 1,
    Bounds,
    Unreachable,
    IllegalOpcode,
    InvalidNativeInvoke,
    NotSupported,
    InvalidInput,
    ExecutionLimit,
    MemoryLimit,
    SlotLimit,
    FatalSignal,
    Fuse,
    DivideByZero
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

impl ExecuteError {
    pub fn status(&self) -> i32 {
        -(*self as u8 as i32)
    }
}
