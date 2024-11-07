use compiler::Compiler;

mod compiler;
mod vm;

fn main() {
    let compiler = Compiler::new(std::fs::read_to_string("test.dy").unwrap());
    let mut vm = compiler.compile();
    vm.run();
}
