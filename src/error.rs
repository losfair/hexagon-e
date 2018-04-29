#[derive(Clone, Debug)]
pub enum ExecuteError {
    Generic,
    Bounds,
    Unreachable,
    IllegalOpcode,
    InvalidNativeInvoke
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;
