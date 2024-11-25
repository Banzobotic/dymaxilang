use super::value::Value;

#[repr(u8)]
#[derive(Debug)]
pub enum OpCode {
    LoadConstant,
    LoadConstantExt,
    Null,
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Not,
    Negate,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    GetMap,
    SetMap,
    #[cfg(feature = "local_map_scopes")]
    PushMap,
    #[cfg(feature = "local_map_scopes")]
    PopMap,
    Jump,
    JumpIfFalse,
    JumpIfFalseNoPop,
    JumpIfTrueNoPop,
    Call,
    Return,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    code: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn code_ptr(&self) -> *const u8 {
        self.code.as_ptr()
    }

    pub fn size(&self) -> usize {
        self.code.len() + self.constants.len() * size_of::<Value>()
    }

    #[cfg(feature = "local_map_scopes")]
    pub fn push_map(&mut self, target: usize) {
        self.code.insert(target, OpCode::PushMap as u8);
        self.push_opcode(OpCode::PopMap);
    }

    fn add_constant(&mut self, constant: Value) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }

    pub fn push_constant(&mut self, constant: Value) {
        let idx = self.add_constant(constant);
        if idx <= u8::MAX as usize {
            self.push_opcode(OpCode::LoadConstant);
            self.push_byte(idx as u8);
        } else {
            self.push_opcode(OpCode::LoadConstantExt);
            self.push_byte(((idx >> 16) & 0xFF) as u8);
            self.push_byte(((idx >> 8) & 0xFF) as u8);
            self.push_byte((idx & 0xFF) as u8);
        }
    }

    pub fn push_jump(&mut self, opcode: OpCode) -> usize {
        self.push_opcode(opcode);
        self.push_byte(0xFF);
        self.jump_target() - 1
    }

    pub fn push_loop(&mut self, target: usize) {
        let offset = (target as isize - self.code.len() as isize - 2) as u8;
        self.push_opcode(OpCode::Jump);
        self.push_byte(offset);
    }

    pub fn patch_jump(&mut self, jump_idx: usize) {
        let offset = (self.code.len() as isize - jump_idx as isize - 1) as u8;
        self.code[jump_idx] = offset;
    }

    pub fn jump_target(&self) -> usize {
        self.code.len()
    }

    pub fn push_opcode(&mut self, opcode: OpCode) {
        self.push_byte(opcode as u8);
    }

    pub fn push_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    #[cfg(any(feature = "decompile", feature = "trace_execution"))]
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04X} ", offset);

        use OpCode as Op;
        match unsafe { std::mem::transmute::<u8, OpCode>(self.code[offset]) } {
            op @ (Op::Null
            | Op::Pop
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Equal
            | Op::NotEqual
            | Op::Greater
            | Op::GreaterEqual
            | Op::Less
            | Op::LessEqual
            | Op::Not
            | Op::Negate
            | Op::GetMap
            | Op::SetMap
            | Op::Return) => {
                println!("{:?}", op);
                offset + 1
            }
            #[cfg(feature = "local_map_scopes")]
            op @ (Op::PushMap | Op::PopMap) => {
                println!("{:?}", op);
                offset + 1
            }
            op @ (Op::DefineGlobal
            | Op::GetGlobal
            | Op::SetGlobal
            | Op::GetLocal
            | Op::SetLocal
            | Op::Jump
            | Op::JumpIfFalse
            | Op::JumpIfFalseNoPop
            | Op::JumpIfTrueNoPop
            | Op::Call) => {
                let constant = self.code[offset + 1];
                println!("{:16} {:04X}", format!("{:?}", op), constant);
                offset + 2
            }
            op @ Op::LoadConstant => {
                let idx = self.code[offset + 1];
                println!("{:16} {:04X} {}", format!("{:?}", op), idx, self.constants[idx as usize]);
                offset + 2
            }
            op @ Op::LoadConstantExt => {
                println!("{} {} {}", self.code[offset + 1], self.code[offset + 2], self.code[offset + 3]);
                let idx = (self.code[offset + 1] as usize) << 16 | (self.code[offset + 2] as usize) << 8 | self.code[offset + 3] as usize;
                println!("{:16} {:04X} {}", format!("{:?}", op), idx, self.constants[idx]);
                offset + 4
            }
        }
    }

    #[cfg(feature = "decompile")]
    pub fn disassemble(&self) {
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }
}
