use core::cell::Cell;
use tape::Tape;
use module::Opcode;
use error::*;

pub trait Environment {
    fn get_memory(&self) -> &[u8];
    fn get_memory_mut(&mut self) -> &mut [u8];
    fn grow_memory(&mut self, len_inc: usize) -> ExecuteResult<()>;

    fn get_slots(&self) -> &[i64];
    fn get_slots_mut(&mut self) -> &mut [i64];
    fn reset_slots(&mut self, len: usize) -> ExecuteResult<()>;

    fn get_stack(&self) -> &Tape<Cell<i64>>;

    // Frame layout (from top to bottom):
    // - return_ip
    // - n_all_locals /* n_args + n_locals */
    // - [all_locals]
    fn get_call_stack(&self) -> &Tape<Cell<i64>>;

    fn do_native_invoke(&mut self, _id: usize) -> ExecuteResult<Option<i64>> {
        Err(ExecuteError::InvalidNativeInvoke)
    }

    fn trace_mem_init(&self, _start: usize, _data: &[u8]) {}
    fn trace_opcode(&self, _op: &Opcode) -> ExecuteResult<()> { Ok(()) }
    fn trace_call(&self, _target: usize, _n_locals: usize) {}
    fn trace_load(&self, _offset: usize, _addr: usize, _val: u64) {}
}
