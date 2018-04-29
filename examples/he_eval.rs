extern crate hexagon_e;

use std::fs::File;
use std::io::Read;
use std::env;
use std::cell::Cell;

use hexagon_e::environment::Environment;
use hexagon_e::tape::Tape;
use hexagon_e::error::ExecuteResult;
use hexagon_e::module::Opcode;

struct ResourceHolder {
    mem: Vec<u8>,
    stack: Vec<Cell<i64>>,
    call_stack: Vec<Cell<i64>>
}

struct ExecutionEnv<'a> {
    mem: &'a mut Vec<u8>,
    stack: Tape<'a, Cell<i64>>,
    call_stack: Tape<'a, Cell<i64>>,
}

impl<'a> ExecutionEnv<'a> {
    fn new(rh: &'a mut ResourceHolder) -> ExecutionEnv<'a> {
        ExecutionEnv {
            mem: &mut rh.mem,
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

    fn trace_opcode(&self, op: &Opcode) {
        println!("{:?}", op);
    }

    fn trace_call(&self, target: usize) {
        println!("call {}", target);
    }
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
        stack: vec! [ Cell::new(0); 1024 ],
        call_stack: vec! [ Cell::new(0); 1024 ]
    };
    let env = ExecutionEnv::new(&mut rh);

    let mut vm = hexagon_e::vm::VirtualMachine::new(&module, env);
    vm.run_memory_initializers().unwrap();
    vm.run().unwrap();
}
