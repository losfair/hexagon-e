use core::cell::Cell;
use tape::Tape;

pub trait Environment {
    fn get_memory(&self) -> &[u8];
    fn get_memory_mut(&mut self) -> &mut [u8];
    fn grow_memory(&mut self, len_inc: usize);

    fn get_stack(&self) -> &Tape<Cell<i64>>;
}
