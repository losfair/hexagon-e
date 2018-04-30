use environment::Environment;
use module::{Module, Opcode};
use tape::{Tape, TapeU8};
use byteorder::{LittleEndian, ByteOrder};
use error::*;

pub struct VirtualMachine<'a, E: Environment> {
    pub module: Module<'a>,
    pub env: E,

    reset_slots_fuse: bool
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
    ($env:expr, $code:expr, $t1: ty, $t2: ty, $read:ident) => {
        let offset = $code.next_u32()? as usize;
        let addr = pop1!($env) as u32 as usize;

        let real_addr = offset + addr;
        let val = $env.get_memory().$read(real_addr)? as $t1 as $t2;
        $env.trace_load(offset, addr, val as u64);
        push1!($env, val as u64 as _);
    }
}

macro_rules! store_val {
    ($env:expr, $code:expr, $write:ident) => {
        let offset = $code.next_u32()? as usize;
        let val = pop1!($env) as u64 as _;
        let addr = pop1!($env) as u32 as usize;

        let real_addr = offset + addr;
        $env.get_memory_mut().$write(real_addr, val)?;
    }
}

macro_rules! run_unop {
    ($env:expr, $t:ty, $body:expr) => {
        {
            let v = pop1!($env);
            let result = ($body)(v as $t) as $t;
            push1!($env, result as u64 as i64);
        }
    }
}

macro_rules! run_binop {
    ($env:expr, $t:ty, $body:expr) => {
        {
            let (left, right) = pop2!($env);
            let result = ($body)(left as $t, right as $t) as $t;
            push1!($env, result as u64 as i64);
        }
    }
}

macro_rules! run_relop {
    ($env:expr, $t:ty, $body:expr) => {
        {
            let (left, right) = pop2!($env);
            let result = ($body)(left as $t, right as $t);
            push1!($env, if result == true { 1 } else { 0 });
        }
    }
}

impl<'a, E: Environment> VirtualMachine<'a, E> {
    pub fn new(
        module: &Module<'a>,
        env: E
    ) -> VirtualMachine<'a, E> {
        VirtualMachine {
            module: *module,
            env: env,
            reset_slots_fuse: false
        }
    }

    pub fn run_memory_initializers(&mut self) -> ExecuteResult<()> {
        let mi = Tape::from(self.module.memory_initializers);

        loop {
            let addr = match mi.next_u32() {
                Ok(v) => v,
                Err(_) => break
            } as usize;

            let data_len = mi.next_u32()? as usize;
            let data = mi.next_many(data_len)?;

            {
                let mem = self.env.get_memory_mut();

                if addr >= mem.len() || addr + data_len > mem.len() {
                    return Err(ExecuteError::Bounds);
                }

                mem[addr..addr + data_len].copy_from_slice(data);
            }

            self.env.trace_mem_init(addr as usize, data);
        }

        Ok(())
    }

    pub fn run(&mut self) -> ExecuteResult<()> {
        let code = Tape::from(self.module.code);
        loop {
            let op = Opcode::from_raw(*(code.next()?))?;
            self.env.trace_opcode(&op)?;

            match op {
                Opcode::Drop => {
                    pop1!(self.env);
                },
                Opcode::Dup => {
                    let stack = self.env.get_stack();
                    let val = stack.tail_many(1)?[0].get();
                    stack.next()?.set(val);
                },
                Opcode::Swap2 => {
                    let stack = self.env.get_stack();
                    let tail = stack.tail_many(2)?;
                    let a = tail[0].get();
                    let b = tail[1].get();
                    tail[0].set(b);
                    tail[1].set(a);
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

                    let vs = self.env.get_stack();
                    let cs = self.env.get_call_stack();

                    let n_locals = vs.prev()?.get() as usize;
                    let target = vs.prev()?.get() as usize;

                    self.env.trace_call(target, n_locals);

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
                    code.set_pos(target)?;
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
                Opcode::GetSlotIndirect => {
                    let id = pop1!(self.env) as usize;

                    let slots = self.env.get_slots();
                    bounds_check(slots, id, 1)?;

                    let val = slots[id];
                    push1!(self.env, val);
                },
                Opcode::GetSlot => {
                    let id = code.next_u32()? as usize;

                    let slots = self.env.get_slots();
                    bounds_check(slots, id, 1)?;

                    let val = slots[id];
                    push1!(self.env, val);
                },
                Opcode::SetSlot => {
                    let id = code.next_u32()? as usize;
                    let val = pop1!(self.env);

                    let slots = self.env.get_slots_mut();
                    bounds_check(slots, id, 1)?;

                    slots[id] = val;
                },
                Opcode::ResetSlots => {
                    let n = code.next_u32()? as usize;

                    if self.reset_slots_fuse {
                        return Err(ExecuteError::Fuse);
                    }
                    self.reset_slots_fuse = true;

                    self.env.reset_slots(n)?;
                },
                Opcode::NativeInvoke => {
                    let id = code.next_u32()? as usize;
                    let ret = self.env.do_native_invoke(id)?;
                    if let Some(v) = ret {
                        push1!(self.env, v);
                    }
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
                Opcode::NotSupported => {
                    return Err(ExecuteError::NotSupported);
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
                Opcode::JmpEither => {
                    let target_a = code.next_u32()? as usize;
                    let target_b = code.next_u32()? as usize;
                    let cond = pop1!(self.env);
                    if cond != 0 {
                        code.set_pos(target_a)?;
                    } else {
                        code.set_pos(target_b)?;
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
                    load_val!(self.env, code, u32, u32, read_u32);
                },
                Opcode::I32Load8U => {
                    load_val!(self.env, code, u8, u32, read_u8);
                },
                Opcode::I32Load8S => {
                    load_val!(self.env, code, i8, i32, read_u8);
                },
                Opcode::I32Load16U => {
                    load_val!(self.env, code, u16, u32, read_u16);
                },
                Opcode::I32Load16S => {
                    load_val!(self.env, code, i16, i32, read_u16);
                },
                Opcode::I32Store => {
                    store_val!(self.env, code, write_u32);
                },
                Opcode::I32Store8 => {
                    store_val!(self.env, code, write_u8);
                },
                Opcode::I32Store16 => {
                    store_val!(self.env, code, write_u16);
                },
                Opcode::I32Const => {
                    let v = code.next_u32()?;
                    push1!(self.env, v as i64);
                },
                Opcode::I32Clz => run_unop!(self.env, i32, |v| unsafe { ::core::intrinsics::ctlz(v) }),
                Opcode::I32Ctz => run_unop!(self.env, i32, |v| unsafe { ::core::intrinsics::cttz(v) }),
                Opcode::I32Popcnt => run_unop!(self.env, i32, |v| unsafe { ::core::intrinsics::ctpop(v) }),
                Opcode::I32Add => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_add(b)),
                Opcode::I32Sub => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_sub(b)),
                Opcode::I32Mul => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_mul(b)),
                Opcode::I32DivU => run_binop!(self.env, u32, |a: u32, b: u32| a.wrapping_div(b)),
                Opcode::I32DivS => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_div(b)),
                Opcode::I32RemU => run_binop!(self.env, u32, |a: u32, b: u32| a.wrapping_rem(b)),
                Opcode::I32RemS => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_rem(b)),
                Opcode::I32And => run_binop!(self.env, u32, |a: u32, b: u32| a & b),
                Opcode::I32Or => run_binop!(self.env, u32, |a: u32, b: u32| a | b),
                Opcode::I32Xor => run_binop!(self.env, u32, |a: u32, b: u32| a ^ b),
                Opcode::I32Shl => run_binop!(self.env, u32, |a: u32, b: u32| a.wrapping_shl(b)),
                Opcode::I32ShrU => run_binop!(self.env, u32, |a: u32, b: u32| a.wrapping_shr(b)),
                Opcode::I32ShrS => run_binop!(self.env, i32, |a: i32, b: i32| a.wrapping_shr(b as u32)),
                Opcode::I32Rotl => run_binop!(self.env, u32, |a: u32, b: u32| a.rotate_left(b)),
                Opcode::I32Rotr => run_binop!(self.env, u32, |a: u32, b: u32| a.rotate_right(b)),
                Opcode::I32Eq => run_relop!(self.env, u32, |a: u32, b: u32| a == b),
                Opcode::I32Ne => run_relop!(self.env, u32, |a: u32, b: u32| a != b),
                Opcode::I32LtU => run_relop!(self.env, u32, |a: u32, b: u32| a < b),
                Opcode::I32LtS => run_relop!(self.env, i32, |a: i32, b: i32| a < b),
                Opcode::I32LeU => run_relop!(self.env, u32, |a: u32, b: u32| a <= b),
                Opcode::I32LeS => run_relop!(self.env, i32, |a: i32, b: i32| a <= b),
                Opcode::I32GtU => run_relop!(self.env, u32, |a: u32, b: u32| a > b),
                Opcode::I32GtS => run_relop!(self.env, i32, |a: i32, b: i32| a > b),
                Opcode::I32GeU => run_relop!(self.env, u32, |a: u32, b: u32| a >= b),
                Opcode::I32GeS => run_relop!(self.env, i32, |a: i32, b: i32| a >= b),

                Opcode::I32WrapI64 => run_unop!(self.env, u32, |v: u32| v),

                Opcode::I64Load => {
                    load_val!(self.env, code, u64, u64, read_u64);
                },
                Opcode::I64Load8U => {
                    load_val!(self.env, code, u8, u64, read_u8);
                },
                Opcode::I64Load8S => {
                    load_val!(self.env, code, i8, i64, read_u8);
                },
                Opcode::I64Load16U => {
                    load_val!(self.env, code, u16, u64, read_u16);
                },
                Opcode::I64Load16S => {
                    load_val!(self.env, code, i16, i64, read_u16);
                },
                Opcode::I64Load32U => {
                    load_val!(self.env, code, u32, u64, read_u32);
                },
                Opcode::I64Load32S => {
                    load_val!(self.env, code, i32, i64, read_u32);
                },
                Opcode::I64Store => {
                    store_val!(self.env, code, write_u64);
                },
                Opcode::I64Store8 => {
                    store_val!(self.env, code, write_u8);
                },
                Opcode::I64Store16 => {
                    store_val!(self.env, code, write_u16);
                },
                Opcode::I64Store32 => {
                    store_val!(self.env, code, write_u32);
                },
                Opcode::I64Const => {
                    let v = code.next_u64()?;
                    push1!(self.env, v as i64);
                },
                Opcode::I64Clz => run_unop!(self.env, i64, |v| unsafe { ::core::intrinsics::ctlz(v) }),
                Opcode::I64Ctz => run_unop!(self.env, i64, |v| unsafe { ::core::intrinsics::cttz(v) }),
                Opcode::I64Popcnt => run_unop!(self.env, i64, |v| unsafe { ::core::intrinsics::ctpop(v) }),
                Opcode::I64Add => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_add(b)),
                Opcode::I64Sub => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_sub(b)),
                Opcode::I64Mul => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_mul(b)),
                Opcode::I64DivU => run_binop!(self.env, u64, |a: u64, b: u64| a.wrapping_div(b)),
                Opcode::I64DivS => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_div(b)),
                Opcode::I64RemU => run_binop!(self.env, u64, |a: u64, b: u64| a.wrapping_rem(b)),
                Opcode::I64RemS => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_rem(b)),
                Opcode::I64And => run_binop!(self.env, u64, |a: u64, b: u64| a & b),
                Opcode::I64Or => run_binop!(self.env, u64, |a: u64, b: u64| a | b),
                Opcode::I64Xor => run_binop!(self.env, u64, |a: u64, b: u64| a ^ b),
                Opcode::I64Shl => run_binop!(self.env, u64, |a: u64, b: u64| a.wrapping_shl(b as u32)),
                Opcode::I64ShrU => run_binop!(self.env, u64, |a: u64, b: u64| a.wrapping_shr(b as u32)),
                Opcode::I64ShrS => run_binop!(self.env, i64, |a: i64, b: i64| a.wrapping_shr(b as u32)),
                Opcode::I64Rotl => run_binop!(self.env, u64, |a: u64, b: u64| a.rotate_left(b as u32)),
                Opcode::I64Rotr => run_binop!(self.env, u64, |a: u64, b: u64| a.rotate_right(b as u32)),
                Opcode::I64Eq => run_relop!(self.env, u64, |a: u64, b: u64| a == b),
                Opcode::I64Ne => run_relop!(self.env, u64, |a: u64, b: u64| a != b),
                Opcode::I64LtU => run_relop!(self.env, u64, |a: u64, b: u64| a < b),
                Opcode::I64LtS => run_relop!(self.env, i64, |a: i64, b: i64| a < b),
                Opcode::I64LeU => run_relop!(self.env, u64, |a: u64, b: u64| a <= b),
                Opcode::I64LeS => run_relop!(self.env, i64, |a: i64, b: i64| a <= b),
                Opcode::I64GtU => run_relop!(self.env, u64, |a: u64, b: u64| a > b),
                Opcode::I64GtS => run_relop!(self.env, i64, |a: i64, b: i64| a > b),
                Opcode::I64GeU => run_relop!(self.env, u64, |a: u64, b: u64| a >= b),
                Opcode::I64GeS => run_relop!(self.env, i64, |a: i64, b: i64| a >= b),
                Opcode::I64ExtendI32U => run_unop!(self.env, u64, |v: u64| v as u32 as u64),
                Opcode::I64ExtendI32S => run_unop!(self.env, u64, |v: u64| v as u32 as i32 as i64 as u64),
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
        Ok(self[ra])
    }

    fn read_u16(&self, ra: usize) -> ExecuteResult<u16> {
        bounds_check(self, ra, 2)?;
        Ok(LittleEndian::read_u16(&self[ra..]))
    }

    fn read_u32(&self, ra: usize) -> ExecuteResult<u32> {
        bounds_check(self, ra, 4)?;
        Ok(LittleEndian::read_u32(&self[ra..]))
    }

    fn read_u64(&self, ra: usize) -> ExecuteResult<u64> {
        bounds_check(self, ra, 8)?;
        Ok(LittleEndian::read_u64(&self[ra..]))
    }

    fn write_u8(&mut self, ra: usize, v: u8) -> ExecuteResult<()> {
        bounds_check(self, ra, 1)?;
        self[ra] = v;
        Ok(())
    }

    fn write_u16(&mut self, ra: usize, v: u16) -> ExecuteResult<()> {
        bounds_check(self, ra, 2)?;
        LittleEndian::write_u16(&mut self[ra..], v);
        Ok(())
    }

    fn write_u32(&mut self, ra: usize, v: u32) -> ExecuteResult<()> {
        bounds_check(self, ra, 4)?;
        LittleEndian::write_u32(&mut self[ra..], v);
        Ok(())
    }

    fn write_u64(&mut self, ra: usize, v: u64) -> ExecuteResult<()> {
        bounds_check(self, ra, 8)?;
        LittleEndian::write_u64(&mut self[ra..], v);
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
