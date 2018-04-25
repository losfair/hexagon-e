use environment::Environment;
use module::{Module, Opcode};
use tape::{Tape, TapeU8};
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
            let stack = $env.get_stack();
            let location = stack.next()?;
            location.set($v);
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

    pub fn run(&mut self, state: &mut ExecutionState) -> ExecuteResult<()> {
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
                Opcode::SaveSp => {
                    let sp = state.sp;
                    push1!(self.env, sp as _);

                    let new_sp = self.env.get_stack().get_pos();
                    state.sp = new_sp;
                },
                Opcode::RestoreSp => {
                    let sp = state.sp;
                    self.env.get_stack().set_pos(sp)?;

                    let old_sp = pop1!(self.env);
                    state.sp = old_sp as _;
                },
                Opcode::SaveIp => {
                    let ip = state.ip;
                    push1!(self.env, ip as _);

                    let new_ip = code.get_pos();
                    state.ip = new_ip;
                },
                Opcode::RestoreIp => {
                    // state.ip usually points at the `Jmp` opcode now.
                    // So we need to move it forward by one.
                    let ip = state.ip + 1;
                    code.set_pos(ip)?;

                    let old_ip = pop1!(self.env);
                    state.ip = old_ip as _;
                },
                Opcode::ReserveStack => {
                    let n = code.next_u32()? as usize;
                    let stack = self.env.get_stack();
                    let area = stack.next_many(n)?;

                    for i in 0..n {
                        area[i].set(0);
                    }
                },
                Opcode::GetStack => {
                    let n = code.next_u32()? as usize;
                    let stack = self.env.get_stack();
                    let sp = state.sp;
                    let v = stack.at(sp + n)?.get();
                    stack.next()?.set(v);
                },
                Opcode::SetStack => {
                    let n = code.next_u32()? as usize;
                    let stack = self.env.get_stack();
                    let sp = state.sp;
                    let v = stack.prev()?.get();
                    stack.at(sp + n)?.set(v);
                }
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
                Opcode::JmpIndirect => {
                    let target = pop1!(self.env);
                    code.set_pos(target as _)?;
                },
                Opcode::JmpIndirectIf => {
                    let (cond, target) = pop2!(self.env);
                    if cond != 0 {
                        code.set_pos(target as _)?;
                    }
                },
                Opcode::JmpTable => {
                    let cond = pop1!(self.env) as usize;
                    let default_target = code.next_u32()? as usize;

                    let table_len = code.next_u32()? as usize;
                    let table = code.next_many(table_len * 4)?; // 32-bit

                    use byteorder::{LittleEndian, ByteOrder};

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
