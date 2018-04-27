use error::*;
use byteorder::{LittleEndian, ByteOrder};

#[derive(Copy, Clone, Debug)]
pub struct Module<'a> {
    pub memory_initializers: &'a [u8], // Serialized
    pub code: &'a [u8] // Raw opcodes & immediates
}

impl<'a> Module<'a> {
    pub fn from_raw(mut s: &'a [u8]) -> ExecuteResult<Module<'a>> {
        if s.len() < 4 {
            return Err(ExecuteError::Bounds);
        }
        let initializers_len = LittleEndian::read_u32(s) as usize;
        s = &s[4..];

        if s.len() < initializers_len {
            return Err(ExecuteError::Bounds);
        }
        let memory_initializers = &s[0..initializers_len];
        s = &s[initializers_len..];

        let code = s;

        Ok(Module {
            memory_initializers: memory_initializers,
            code: code
        })
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Opcode {
    Drop = 1,
    Select,

    Call,
    Return,
    Halt,

    GetLocal,
    SetLocal,
    TeeLocal,

    CurrentMemory,
    GrowMemory,

    Nop,
    Unreachable,

    Jmp,
    JmpIf,
    JmpTable,

    I32Load,
    I32Load8U,
    I32Load8S,
    I32Load16U,
    I32Load16S,
    I32Store,
    I32Store8,
    I32Store16,

    I32Const,
    I32Ctz,
    I32Clz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivU,
    I32DivS,
    I32RemU,
    I32RemS,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrU,
    I32ShrS,
    I32Rotl,
    I32Rotr,

    Never
}

impl Opcode {
    pub fn from_raw(v: u8) -> ExecuteResult<Opcode> {
        if v > 0 && v < Opcode::Never as u8 {
            Ok(unsafe { ::core::mem::transmute(v) })
        } else {
            Err(ExecuteError::IllegalOpcode)
        }
    }
}
