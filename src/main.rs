use std::{env, process};

use compiler::Compiler;

mod compiler;
mod vm;

fn main() {
    let mut args = env::args();
    let Some(file) = args.nth(1) else {
        eprintln!("\x1b[91merror\x1b[0m: need to provide path to source file");
        process::exit(1);
    };
    let Ok(source) = std::fs::read_to_string(file) else {
        eprintln!("\x1b[91merror\x1b[0m: source file not found");
        process::exit(1);
    };
    let compiler = Compiler::new(source);
    let mut vm = compiler.compile();
    vm.run();
}
