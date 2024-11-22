use std::env;

use compiler::Compiler;

mod compiler;
mod vm;

fn main() {
    let mut args = env::args();
    let file = args.nth(1).unwrap_or(String::from("test.dy"));
    let compiler = Compiler::new(
        std::fs::read_to_string(file).unwrap(),
    );
    let mut vm = compiler.compile();
    vm.run();
}
