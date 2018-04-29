# Syscalls

This document specifies how virtual "system calls" should be implemented by users of HexagonE.

All system call numbers within the range of u16 (0-65535 inclusive) are reserved, and some of them are defined here to be included in the "standard" set of syscalls.

Syscall numbers greater than or equal to 65536 can be freely used for user-specified purposes.

### The standard set

Here are all the standard system calls that should be implemented by all users if possible.

- (0) log

**Parameters:**

- level: i32
- text_base: i32 (pointer)
- text_len: i32

**Returns:** `none`

**Semantics:**

Writes `text` to the environment-provided logger.

The text must be valid UTF-8 or otherwise the behavior is implementation-defined.

`level` can be one of:

- 1: Error
- 3: Warning
- 6: Info
