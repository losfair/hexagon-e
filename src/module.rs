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
    Dup,
    Swap2,
    Select,

    Call,
    Return,
    Halt,

    GetLocal,
    SetLocal,
    TeeLocal,

    GetSlotIndirect,
    GetSlot,
    SetSlot,
    ResetSlots,

    NativeInvoke,

    CurrentMemory,
    GrowMemory,

    Nop,
    Unreachable,
    NotSupported,

    Jmp,
    JmpIf,
    JmpEither,
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

    I32Eq,
    I32Ne,
    I32LtU,
    I32LtS,
    I32LeU,
    I32LeS,
    I32GtU,
    I32GtS,
    I32GeU,
    I32GeS,

    I32WrapI64,

    I64Load,
    I64Load8U,
    I64Load8S,
    I64Load16U,
    I64Load16S,
    I64Load32U,
    I64Load32S,
    I64Store,
    I64Store8,
    I64Store16,
    I64Store32,

    I64Const,
    I64Ctz,
    I64Clz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivU,
    I64DivS,
    I64RemU,
    I64RemS,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrU,
    I64ShrS,
    I64Rotl,
    I64Rotr,

    I64Eq,
    I64Ne,
    I64LtU,
    I64LtS,
    I64LeU,
    I64LeS,
    I64GtU,
    I64GtS,
    I64GeU,
    I64GeS,

    I64ExtendI32U,
    I64ExtendI32S,

    Never
}

impl Opcode {
    #[inline]
    pub fn from_raw(v: u8) -> ExecuteResult<Opcode> {
        if v > 0 && v < Opcode::Never as u8 {
            Ok(unsafe { ::core::mem::transmute(v) })
        } else {
            Err(ExecuteError::IllegalOpcode)
        }
    }
}
