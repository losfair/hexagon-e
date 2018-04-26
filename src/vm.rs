use environment::Environment;
use module::{Module, Opcode};
use tape::{Tape, TapeU8};
use byteorder::{LittleEndian, ByteOrder};
use error::*;

pub struct VirtualMachine<'a, E: Environment> {
    pub module: Module<'a>,
    pub env: E
}

#[derive(Copy, Clone, Debug, Default)]
pub struct ExecutionState {
    pub sp: usize,
    pub ip: usize
}

macro_rules! pop1 {
    ($env:expr) => {
        $env.get_stack().prev()?.get()
    }
}

macro_rules! pop2 {
    ($env:expr) => {
        {
            let stack = $env.get_stack();
            let b = stack.prev()?;
            let a = stack.prev()?;
            (a.get(), b.get())
        }
    }
}

macro_rules! pop3 {
    ($env:expr) => {
        {
            let stack = $env.get_stack();
            let c = stack.prev()?;
            let b = stack.prev()?;
            let a = stack.prev()?;
            (a.get(), b.get(), c.get())
        }
    }
}

macro_rules! push1 {
    ($env:expr, $v:expr) => {
        {
            let v = $v;

            let stack = $env.get_stack();
            let location = stack.next()?;
            location.set(v);
        }
    }
}

macro_rules! extract_locals {
    ($cs:expr) => {
        {
            let n_all_locals = $cs.tail_many(2)?[0].get() as usize;
            &$cs.tail_many(n_all_locals + 2)?[0..n_all_locals]
        }
    }
}

macro_rules! get_local {
    ($env:expr, $id:expr) => {
        {
            let id = $id;
            let cs = $env.get_call_stack();
            let locals = extract_locals!(cs);

            if id >= locals.len() {
                return Err(ExecuteError::Bounds);
            }

            push1!($env, locals[id].get());
        }
    }
}

macro_rules! set_local {
    ($env:expr, $id:expr) => {
        {
            let id = $id;
            let cs = $env.get_call_stack();
            let locals = extract_locals!(cs);

            if id >= locals.len() {
                return Err(ExecuteError::Bounds);
            }

            locals[id].set(pop1!($env));
        }
    }
}

macro_rules! tee_local {
    ($env:expr, $id:expr) => {
        {
            let id = $id;
            let cs = $env.get_call_stack();
            let locals = extract_locals!(cs);

            if id >= locals.len() {
                return Err(ExecuteError::Bounds);
            }

            locals[id].set($env.get_stack().tail_many(1)?[0].get());
        }
    }
}

macro_rules! load_val {
    ($env:expr, $code:expr, $t: ty, $read:ident) => {
        let offset = $code.next_u32()? as usize;
        let addr = pop1!($env) as usize;

        let real_addr = offset + addr;
        let val = $env.get_memory().$read(real_addr)? as $t;
        push1!($env, val as u64 as _);
    }
}

macro_rules! store_val {
    ($env:expr, $code:expr, $t:ty, $write:ident) => {
        let offset = $code.next_u32()? as usize;
        let val = pop1!($env) as u64 as $t;
        let addr = pop1!($env) as usize;

        let real_addr = offset + addr;
        $env.get_memory_mut().$write(real_addr, val)?;
    }
}

impl<'a, E: Environment> VirtualMachine<'a, E> {
    pub fn new(
        module: &Module<'a>,
        env: E
    ) -> VirtualMachine<'a, E> {
        VirtualMachine {
            module: *module,
            env: env
        }
    }

    pub fn run_memory_initializers(&mut self) -> ExecuteResult<()> {
        let mem = self.env.get_memory_mut();
        let mi = Tape::from(self.module.memory_initializers);

        loop {
            let addr = match mi.next_u32() {
                Ok(v) => v,
                Err(_) => break
            } as usize;

            let data_len = mi.next_u32()? as usize;
            let data = mi.next_many(data_len)?;

            if addr >= mem.len() || addr + data_len > mem.len() {
                return Err(ExecuteError::Bounds);
            }

            mem[addr..addr + data_len].copy_from_slice(data);
        }

        Ok(())
    }

    pub fn run(&mut self) -> ExecuteResult<()> {
        let code = Tape::from(self.module.code);
        loop {
            let op = Opcode::from_raw(*(code.next()?))?;

            match op {
                Opcode::Drop => {
                    pop1!(self.env);
                },
                Opcode::Select => {
                    let (cond, val1, val2) = pop3!(self.env);
                    if cond != 0 {
                        push1!(self.env, val1);
                    } else {
                        push1!(self.env, val2);
                    }
                },
                Opcode::Call => {
                    let n_args = code.next_u32()? as usize;
                    let n_locals = code.next_u32()? as usize;

                    let vs = self.env.get_stack();
                    let cs = self.env.get_call_stack();

                    let target = vs.prev()?.get();

                    // [all_locals]
                    for arg in vs.prev_many(n_args)? {
                        cs.next()?.set(arg.get());
                    }
                    for _ in 0..n_locals {
                        cs.next()?.set(0);
                    }

                    // n_all_locals
                    cs.next()?.set((n_args + n_locals) as _);

                    // return_ip
                    cs.next()?.set(code.get_pos() as _);

                    // Jump!
                    code.set_pos(target as usize)?;
                },
                Opcode::Return => {
                    let cs = self.env.get_call_stack();

                    let return_ip = cs.prev()?.get();
                    let n_all_locals = cs.prev()?.get();

                    cs.prev_many(n_all_locals as _)?;

                    code.set_pos(return_ip as _)?;
                },
                Opcode::Halt => {
                    return Ok(());
                },
                Opcode::GetLocal => {
                    let id = code.next_u32()? as usize;
                    get_local!(self.env, id);
                },
                Opcode::SetLocal => {
                    let id = code.next_u32()? as usize;
                    set_local!(self.env, id);
                },
                Opcode::TeeLocal => {
                    let id = code.next_u32()? as usize;
                    tee_local!(self.env, id);
                },
                Opcode::CurrentMemory => {
                    let len = self.env.get_memory().len();
                    push1!(self.env, len as _);
                },
                Opcode::GrowMemory => {
                    let len_inc = pop1!(self.env);

                    let len = self.env.get_memory().len();
                    push1!(self.env, len as _);

                    self.env.grow_memory(len_inc as usize)?;
                },
                Opcode::Nop => {},
                Opcode::Unreachable => {
                    return Err(ExecuteError::Unreachable);
                },
                Opcode::Jmp => {
                    let target = code.next_u32()? as usize;
                    code.set_pos(target)?;
                },
                Opcode::JmpIf => {
                    let target = code.next_u32()? as usize;
                    let cond = pop1!(self.env);
                    if cond != 0 {
                        code.set_pos(target)?;
                    }
                },
                Opcode::JmpTable => {
                    let cond = pop1!(self.env) as usize;
                    let default_target = code.next_u32()? as usize;

                    let table_len = code.next_u32()? as usize;
                    let table = code.next_many(table_len * 4)?; // 32-bit

                    if cond >= table_len {
                        code.set_pos(default_target as _)?;
                    } else {
                        // cond < table_len
                        // => cond + 1 <= table_len
                        // => cond * 4 + 4 <= table_len * 4
                        // table.len() == table_len * 4
                        let target = LittleEndian::read_u32(&table[cond * 4 .. cond * 4 + 4]) as usize;
                        code.set_pos(target)?;
                    }
                },
                Opcode::I32Load => {
                    load_val!(self.env, code, u32, read_u32);
                },
                Opcode::I32Store => {
                    store_val!(self.env, code, u32, write_u32);
                },
                Opcode::I32Const => {
                    let v = code.next_u32()?;
                    push1!(self.env, v as i64);
                },
                Opcode::I32Add => {
                    let (a, b) = pop2!(self.env);
                    push1!(self.env, ((a as i32) + (b as i32)) as u64 as i64);
                }
                Opcode::Never => {
                    return Err(ExecuteError::IllegalOpcode)
                }
            }
        }
    }
}

trait Memory {
    fn read_u8(&self, ra: usize) -> ExecuteResult<u8>;
    fn read_u16(&self, ra: usize) -> ExecuteResult<u16>;
    fn read_u32(&self, ra: usize) -> ExecuteResult<u32>;
    fn read_u64(&self, ra: usize) -> ExecuteResult<u64>;

    fn write_u8(&mut self, ra: usize, v: u8) -> ExecuteResult<()>;
    fn write_u16(&mut self, ra: usize, v: u16) -> ExecuteResult<()>;
    fn write_u32(&mut self, ra: usize, v: u32) -> ExecuteResult<()>;
    fn write_u64(&mut self, ra: usize, v: u64) -> ExecuteResult<()>;
}

fn bounds_check<T>(target: &[T], start: usize, len: usize) -> ExecuteResult<()> {
    if start >= target.len() || start + len > target.len() {
        Err(ExecuteError::Bounds)
    } else {
        Ok(())
    }
}

impl Memory for [u8] {
    fn read_u8(&self, ra: usize) -> ExecuteResult<u8> {
        bounds_check(self, ra, 1)?;
        Ok(self[0])
    }

    fn read_u16(&self, ra: usize) -> ExecuteResult<u16> {
        bounds_check(self, ra, 2)?;
        Ok(LittleEndian::read_u16(self))
    }

    fn read_u32(&self, ra: usize) -> ExecuteResult<u32> {
        bounds_check(self, ra, 4)?;
        Ok(LittleEndian::read_u32(self))
    }

    fn read_u64(&self, ra: usize) -> ExecuteResult<u64> {
        bounds_check(self, ra, 8)?;
        Ok(LittleEndian::read_u64(self))
    }

    fn write_u8(&mut self, ra: usize, v: u8) -> ExecuteResult<()> {
        bounds_check(self, ra, 1)?;
        self[0] = v;
        Ok(())
    }

    fn write_u16(&mut self, ra: usize, v: u16) -> ExecuteResult<()> {
        bounds_check(self, ra, 2)?;
        LittleEndian::write_u16(self, v);
        Ok(())
    }

    fn write_u32(&mut self, ra: usize, v: u32) -> ExecuteResult<()> {
        bounds_check(self, ra, 4)?;
        LittleEndian::write_u32(self, v);
        Ok(())
    }

    fn write_u64(&mut self, ra: usize, v: u64) -> ExecuteResult<()> {
        bounds_check(self, ra, 8)?;
        LittleEndian::write_u64(self, v);
        Ok(())
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    struct TestEnv<'a> {
        mem: [u8; 512],
        stack: Tape<'a, Cell<i64>>
    }

    impl<'a> TestEnv<'a> {
        fn new(stack: &'a [Cell<i64>]) -> TestEnv<'a> {
            TestEnv {
                mem: [0; 512],
                stack: Tape::from(stack)
            }
        }
    }

    impl<'a> Environment for TestEnv<'a> {
        fn get_memory(&self) -> &[u8] {
            &self.mem
        }

        fn get_memory_mut(&mut self) -> &mut [u8] {
            &mut self.mem
        }

        fn grow_memory(&mut self, len_inc: usize) {
            unimplemented!()
        }

        fn get_stack(&self) -> &Tape<Cell<i64>> {
            &self.stack
        }
    }

    fn build_stack_mem() -> [Cell<i64>; 512] {
        unsafe { ::core::mem::zeroed() }
    }

    #[test]
    fn test_basic() {
        let mem = build_stack_mem();
        let env = TestEnv::new(&mem);
    }
}
*/
