#[derive(Clone, Debug)]
pub enum ExecuteError {
    Generic,
    Bounds,
    IllegalOpcode
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;
