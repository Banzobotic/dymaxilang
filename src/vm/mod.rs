use chunk::Chunk;
use stack::Stack;

pub mod chunk;
pub mod stack;
pub mod value;

struct VM {
    chunk: Chunk,
    stack: Stack,
}
