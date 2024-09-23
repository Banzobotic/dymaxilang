use compiler::Compiler;
use vm::VM;

mod compiler;
mod vm;

fn main() {
    let mut compiler = Compiler::new(std::fs::read_to_string("test.dy").unwrap());
    let stack = compiler.compile();
    println!("{:?}", stack);
    let mut vm = VM::new(stack);
    vm.run();
}

