#[derive(Clone, Debug)]
pub enum ExecuteError {
    Generic,
    Bounds,
    Unreachable,
    IllegalOpcode(u8),
    InvalidNativeInvoke,
    NotSupported
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;
