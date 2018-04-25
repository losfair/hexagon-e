use error::*;
use core::cell::Cell;

pub struct Tape<'a, T: 'static> {
    data: &'a [T],
    pos: Cell<usize>
}

impl<'a, T: 'static> From<&'a [T]> for Tape<'a, T> {
    fn from(other: &'a [T]) -> Tape<'a, T> {
        Tape {
            data: other,
            pos: Cell::new(0)
        }
    }
}

impl<'a, T: 'static> Tape<'a, T> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos.get()
    }

    pub fn next(&self) -> ExecuteResult<&T> {
        if self.remaining() < 1 {
            Err(ExecuteError::Bounds)
        } else {
            let pos = self.pos.get();
            let v = &self.data[pos];
            self.pos.set(pos + 1);

            Ok(v)
        }
    }

    pub fn next_many(&self, n: usize) -> ExecuteResult<&[T]> {
        if self.remaining() < n {
            Err(ExecuteError::Bounds)
        } else {
            let pos = self.pos.get();
            let data = &self.data[pos..pos + n];
            self.pos.set(pos + n);

            Ok(data)
        }
    }

    pub fn prev(&self) -> ExecuteResult<&T> {
        if self.pos.get() == 0 {
            Err(ExecuteError::Bounds)
        } else {
            let pos = self.pos.get() - 1;
            let v = &self.data[pos];
            self.pos.set(pos);

            Ok(v)
        }
    }

    pub fn current(&self) -> ExecuteResult<&T> {
        if self.pos.get() < self.data.len() {
            Ok(&self.data[self.pos.get()])
        } else {
            Err(ExecuteError::Bounds)
        }
    }

    pub fn at(&self, at: usize) -> ExecuteResult<&T> {
        if at < self.data.len() {
            Ok(&self.data[at])
        } else {
            Err(ExecuteError::Bounds)
        }
    }

    pub fn get_pos(&self) -> usize {
        self.pos.get()
    }

    pub fn set_pos(&self, pos: usize) -> ExecuteResult<()> {
        if pos <= self.data.len() {
            self.pos.set(pos);
            Ok(())
        } else {
            Err(ExecuteError::Bounds)
        }
    }
}

pub trait TapeU8 {
    fn next_u32(&self) -> ExecuteResult<u32>;
}

impl<'a> TapeU8 for Tape<'a, u8> {
    fn next_u32(&self) -> ExecuteResult<u32> {
        if self.remaining() < 4 {
            Err(ExecuteError::Bounds)
        } else {
            use byteorder::{LittleEndian, ByteOrder};

            let pos = self.pos.get();
            let v = LittleEndian::read_u32(&self.data[pos..pos + 4]);
            self.pos.set(pos + 4);

            Ok(v)
        }
    }
}
