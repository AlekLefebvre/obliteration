use self::cpu::{CpuState, ValueState};
use crate::rtld::Memory;
use iced_x86::{Code, Decoder, DecoderOptions, OpKind, Register};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

pub mod cpu;

/// Contains state of module disassemble.
pub(super) struct Disassembler<'a, 'b: 'a> {
    module: &'a Memory<'b>,
    functions: HashMap<usize, Function>, // Key is the offset in the mapped memory.
}

impl<'a, 'b: 'a> Disassembler<'a, 'b> {
    pub fn new(module: &'a Memory<'b>) -> Self {
        Self {
            module,
            functions: HashMap::new(),
        }
    }

    /// `offset` is an offset of the target **function** in the mapped memory.
    pub fn disassemble(&mut self, offset: usize) -> Result<(), DisassembleError> {
        let mut jobs = VecDeque::from([offset]); // Function offset.

        while let Some(job) = jobs.pop_front() {
            // Check if the offset is already disassembled.
            if self.functions.contains_key(&job) {
                continue;
            }

            // Disassemble.
            let func = self.disassemble_single(job)?;

            jobs.extend(&func.calls);

            if self.functions.insert(job, func).is_some() {
                panic!("Function {job} is already disassembled.");
            }
        }

        Ok(())
    }

    pub fn fixup(&mut self) {
        // TODO: Fixup all disassembled function.
    }

    pub fn get(&self, offset: usize) -> Option<&Function> {
        self.functions.get(&offset)
    }

    fn disassemble_single(&mut self, offset: usize) -> Result<Function, DisassembleError> {
        // Setup the decoder.
        let module = self.module.as_ref();
        let base = module.as_ptr() as u64;
        let decoder = Decoder::with_ip(
            64,
            &module[offset..],
            base + (offset as u64),
            DecoderOptions::AMD,
        );

        // Decode the whole function.
        let mut cpu = CpuState::new();
        let mut func = Function {
            params: Vec::new(),
            returns: Vec::new(),
            instructions: Vec::new(),
            calls: Vec::new(),
            refs: Vec::new(),
        };

        for i in decoder {
            // If the instruction is not valid that mean it is (likely) the end of function.
            if i.is_invalid() {
                break;
            }

            // Parse the instruction.
            let offset = (i.ip() - base) as usize;

            match i.code() {
                Code::Mov_rm64_r64 => self.disassemble_mov(&i, &mut func, &mut cpu),
                Code::Sub_rm64_imm8 => self.disassemble_sub(&i),
                Code::Xor_rm64_r64 => self.disassemble_xor(&i, &mut func, &mut cpu),
                _ => {
                    let opcode = &module[offset..(offset + i.len())];

                    return Err(DisassembleError::UnknownInstruction(
                        offset,
                        opcode.into(),
                        i,
                    ));
                }
            }
        }

        Ok(func)
    }

    fn disassemble_mov(&self, i: &iced_x86::Instruction, f: &mut Function, c: &mut CpuState) {
        if i.op0_kind() == OpKind::Memory {
            panic!("MOV with the first operand is a memory is not supported yet.");
        } else if i.op1_kind() == OpKind::Memory {
            panic!("MOV with the second operand is a memory is not supported yet.");
        } else {
            let dst = i.op0_register();
            let src = i.op1_register();

            // Check the second operand.
            let src: Operand = match c.register(src) {
                ValueState::FromCaller => {
                    let i = f.params.len();

                    f.params.push(src.into());
                    c.set_register(src, ValueState::Param(i));

                    Operand::Param(i)
                }
                ValueState::Param(i) => Operand::Param(*i),
                ValueState::Local => src.into(),
            };

            // Set destination state.
            c.set_register(dst, ValueState::Local);
            f.instructions.push(Instruction::Mov(dst.into(), src));
        }
    }

    fn disassemble_sub(&self, i: &iced_x86::Instruction) {
        if i.op0_kind() == OpKind::Memory {
            if i.has_lock_prefix() {
                panic!("SUB with LOCK prefix is not supported yet.");
            } else {
                panic!("SUB with the first operand is a memory is not supported yet.");
            }
        } else if i.op0_register() == Register::RSP {
            // This SUB is a stack allocation. We don't need to add any instructions to the function
            // here because we don't control a stack allocation on the codegen side.
            if i.op1_kind() != OpKind::Immediate8to64 {
                panic!("SUB RSP with non-immediate value is not supported yet.");
            }

            return;
        } else {
            panic!(
                "SUB with the first operand as the other regiser than RSP is not supported yet."
            );
        }
    }

    fn disassemble_xor(&self, i: &iced_x86::Instruction, f: &mut Function, c: &mut CpuState) {
        let i = if i.op0_kind() == OpKind::Memory {
            if i.has_lock_prefix() {
                panic!("XOR with LOCK prefix is not supported yet.");
            } else {
                panic!("XOR with the first operand as memory is not supported yet.");
            }
        } else {
            // The first operand is a register.
            let dst = i.op0_register();

            match i.op1_kind() {
                OpKind::Register => {
                    // Check if source and destination is the same register.
                    let src = i.op1_register();

                    if dst == src {
                        c.set_register(dst, ValueState::Local);
                        Instruction::Zero(dst.into())
                    } else {
                        panic!("XOR with different registers is not supported yet.");
                    }
                }
                v => panic!("XOR with the second operand is {v:?} is not supported yet."),
            }
        };

        f.instructions.push(i);
    }
}

/// Represents a disassembled function.
pub(super) struct Function {
    params: Vec<Param>,
    returns: Vec<iced_x86::Register>,
    instructions: Vec<Instruction>,
    calls: Vec<usize>,
    refs: Vec<usize>,
}

impl Function {
    pub fn instructions(&self) -> &[Instruction] {
        self.instructions.as_ref()
    }

    /// Gets a slice of the offset this function call to.
    pub fn calls(&self) -> &[usize] {
        self.calls.as_ref()
    }

    /// Gets a slice of the offset whose calling this function.
    pub fn refs(&self) -> &[usize] {
        self.refs.as_ref()
    }
}

/// Represents a function parameter.
pub(super) enum Param {
    Int(usize),
}

impl From<iced_x86::Register> for Param {
    fn from(value: Register) -> Self {
        match value {
            Register::RDI => Self::Int(64),
            Register::RSI => Self::Int(64),
            v => panic!("Register {v:?} is not supported yet."),
        }
    }
}

/// Represents a normalized CPU instruction.
pub(super) enum Instruction {
    Mov(Operand, Operand),
    Zero(Operand),
}

/// Represents the operand of the instruction.
pub(super) enum Operand {
    Param(usize),
    Rax(usize),
    Rbp(usize),
    Rbx(usize),
    Rcx(usize),
    Rdi(usize),
    Rdx(usize),
    Rsi(usize),
    R8(usize),
    R9(usize),
    R10(usize),
    R11(usize),
    R12(usize),
    R13(usize),
    R14(usize),
    R15(usize),
}

impl From<iced_x86::Register> for Operand {
    fn from(value: Register) -> Self {
        match value {
            Register::RAX => Self::Rax(64),
            Register::RBP => Self::Rbp(64),
            Register::RBX => Self::Rbx(64),
            Register::RCX => Self::Rcx(64),
            Register::RDI => Self::Rdi(64),
            Register::RDX => Self::Rdx(64),
            Register::RSI => Self::Rsi(64),
            Register::R8 => Self::R8(64),
            Register::R9 => Self::R9(64),
            Register::R10 => Self::R10(64),
            Register::R11 => Self::R11(64),
            Register::R12 => Self::R12(64),
            Register::R13 => Self::R13(64),
            Register::R14 => Self::R14(64),
            Register::R15 => Self::R15(64),
            v => panic!("Register {v:?} is not supported yet."),
        }
    }
}

/// Represents an error for [`Disassembler::disassemble()`].
#[derive(Debug, Error)]
pub enum DisassembleError {
    #[error("unknown instruction '{2}' ({1:02x?}) at {0:#018x}")]
    UnknownInstruction(usize, Vec<u8>, iced_x86::Instruction),
}
