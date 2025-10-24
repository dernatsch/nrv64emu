

#[derive(Debug, Copy, Clone)]
pub struct RType {
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub funct7: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct IType {
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub imm: i32,
}

#[derive(Debug, Copy, Clone)]
pub struct SType {
    pub opcode: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub imm: i32,
}

#[derive(Debug, Copy, Clone)]
pub struct BType {
    pub opcode: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub imm: i32,
}

#[derive(Debug, Copy, Clone)]
pub struct UType {
    pub opcode: u8,
    pub rd: u8,
    pub imm: i32,
}

impl From<u32> for UType {
    fn from(instruction: u32) -> Self {
        let opcode = instruction & 0x7F;
        let rd = ((instruction >> 7) & 0x1F) as u8;

        Self {
            opcode: opcode as u8,
            rd,
            imm: decode_u_imm(instruction),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct JType {
    pub opcode: u8,
    pub rd: u8,
    pub imm: i32,
}

#[derive(Debug, Copy, Clone)]
pub enum Instruction {
    Auipc(UType),
    Lui(UType),
    Invalid(u32),
}

fn decode_u_imm(instruction: u32) -> i32 {
    (instruction & 0xfffff800) as i32
}

impl Instruction {
    pub fn decode(instruction: u32) -> Self {
        let opcode = instruction & 0x7F;

        match opcode {
            0x17 => Instruction::Auipc(UType::from(instruction)),
            0x37 => Instruction::Lui(UType::from(instruction)),
            _ => unimplemented!("{:#010X} opcode={:02X}", instruction, opcode),
        }
    }

}
