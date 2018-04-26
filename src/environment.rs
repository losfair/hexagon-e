use core::cell::Cell;
use tape::Tape;
use error::ExecuteResult;

pub trait Environment {
    fn get_memory(&self) -> &[u8];
    fn get_memory_mut(&mut self) -> &mut [u8];
    fn grow_memory(&mut self, len_inc: usize) -> ExecuteResult<()>;

    fn get_stack(&self) -> &Tape<Cell<i64>>;

    // Frame layout (from top to bottom):
    // - return_ip
    // - n_all_locals /* n_args + n_locals */
    // - [all_locals]
    fn get_call_stack(&self) -> &Tape<Cell<i64>>;
}
