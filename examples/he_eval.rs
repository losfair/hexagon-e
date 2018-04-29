extern crate hexagon_e;

use std::fs::File;
use std::io::Read;
use std::env;
use std::cell::Cell;

use hexagon_e::environment::Environment;
use hexagon_e::tape::Tape;
use hexagon_e::error::ExecuteResult;
//use hexagon_e::module::Opcode;

struct ResourceHolder {
    mem: Vec<u8>,
    slots: Vec<i64>,
    stack: Vec<Cell<i64>>,
    call_stack: Vec<Cell<i64>>
}

struct ExecutionEnv<'a> {
    mem: &'a mut Vec<u8>,
    slots: &'a mut Vec<i64>,
    stack: Tape<'a, Cell<i64>>,
    call_stack: Tape<'a, Cell<i64>>,
}

impl<'a> ExecutionEnv<'a> {
    fn new(rh: &'a mut ResourceHolder) -> ExecutionEnv<'a> {
        ExecutionEnv {
            mem: &mut rh.mem,
            slots: &mut rh.slots,
            stack: Tape::from(rh.stack.as_slice()),
            call_stack: Tape::from(rh.call_stack.as_slice())
        }
    }
}

impl<'a> Environment for ExecutionEnv<'a> {
    fn get_memory(&self) -> &[u8] {
        &self.mem
    }

    fn get_memory_mut(&mut self) -> &mut [u8] {
        &mut self.mem
    }

    fn get_slots(&self) -> &[i64] {
        &self.slots
    }

    fn get_slots_mut(&mut self) -> &mut [i64] {
        &mut self.slots
    }

    fn reset_slots(&mut self, len: usize) -> ExecuteResult<()> {
        *self.slots = vec! [ 0; len ];
        Ok(())
    }

    fn grow_memory(&mut self, len_inc: usize) -> ExecuteResult<()> {
        self.mem.extend((0..len_inc).map(|_| 0));
        Ok(())
    }

    fn get_stack(&self) -> &Tape<Cell<i64>> {
        &self.stack
    }

    fn get_call_stack(&self) -> &Tape<Cell<i64>> {
        &self.call_stack
    }

/*
    fn trace_opcode(&self, op: &Opcode) {
        println!("{:?}", op);
    }

    fn trace_call(&self, target: usize, n_locals: usize) {
        println!("call {} (n_locals = {})", target, n_locals);
    }*/
/*
    fn trace_load(&self, offset: usize, addr: usize, val: u64) {
        println!("load {} + {} -> {}", offset, addr, val);
    }

    fn trace_mem_init(&self, start: usize, data: &[u8]) {
        println!("mem_init {}, len = {}", start, data.len());
        if data.len() < 1024 {
            println!("data = {:?}", data);
        }
    }
*/
}

fn main() {
    let mut f = File::open(env::args()
        .nth(1)
        .expect("Path expected")
    ).expect("Unable to open code file");

    let mut code: Vec<u8> = Vec::new();
    f.read_to_end(&mut code).unwrap();

    let module = hexagon_e::module::Module::from_raw(&code).unwrap();

    let mut rh = ResourceHolder {
        mem: vec! [ 0; 1048576 ],
        slots: vec! [ 0; 65536 ],
        stack: vec! [ Cell::new(0); 1024 ],
        call_stack: vec! [ Cell::new(0); 1024 ]
    };
    let env = ExecutionEnv::new(&mut rh);

    let mut vm = hexagon_e::vm::VirtualMachine::new(&module, env);
    vm.run_memory_initializers().unwrap();
    vm.run().unwrap();
}
