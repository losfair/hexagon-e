#[derive(Clone, Debug)]
pub enum ExecuteError {
    Generic,
    Bounds,
    Unreachable,
    IllegalOpcode
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;
