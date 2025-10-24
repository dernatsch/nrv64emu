#[derive(Debug, Copy, Clone)]
pub struct RType {
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub funct7: u8,
}

impl From<u32> for RType {
    fn from(instruction: u32) -> Self {
        let opcode = (instruction & 0x7F) as u8;
        let rd = ((instruction >> 7) & 0x1F) as u8;
        let funct3 = ((instruction >> 12) & 0x07) as u8;
        let rs1 = ((instruction >> 15) & 0x1F) as u8;
        let rs2 = ((instruction >> 20) & 0x1F) as u8;
        let funct7 = (instruction >> 25) as u8;

        Self {
            opcode,
            rd,
            funct3,
            rs1,
            rs2,
            funct7,
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct IType {
    pub opcode: u8,
    pub rd: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub imm: i32,
}

impl From<u32> for IType {
    fn from(instruction: u32) -> Self {
        let opcode = (instruction & 0x7F) as u8;
        let rd = ((instruction >> 7) & 0x1F) as u8;
        let funct3 = ((instruction >> 12) & 0x07) as u8;
        let rs1 = ((instruction >> 15) & 0x1F) as u8;
        let imm = (instruction as i32) >> 20;

        Self {
            opcode,
            rd,
            funct3,
            rs1,
            imm,
        }
    }
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

    // Immediates
    Addi(IType),
    Slli(IType),
    Slti(IType),
    Sltiu(IType),
    Xori(IType),
    Ori(IType),
    Andi(IType),

    // CSR
    Csrrw(IType),
    Csrrs(IType),
    Csrrc(IType),

    // OP
    Add(RType),
    Sub(RType),
    Sll(RType),
    Slt(RType),
    Sltu(RType),
    Xor(RType),
    Srl(RType),
    Sra(RType),
    Or(RType),
    And(RType),

    // M
    Mul(RType),
    Mulh(RType),
    Div(RType),
    Divu(RType),
    Rem(RType),
    Remu(RType),

    Invalid(u32),
}

fn decode_u_imm(instruction: u32) -> i32 {
    (instruction & 0xfffff800) as i32
}

impl Instruction {
    pub fn decode(instruction: u32) -> Self {
        let opcode = instruction & 0x7F;

        match opcode {
            0x13 => {
                let it = IType::from(instruction);
                match it.funct3 {
                    0 => Instruction::Addi(it),
                    1 => Instruction::Slli(it),
                    2 => Instruction::Slti(it),
                    3 => Instruction::Sltiu(it),
                    4 => Instruction::Xori(it),
                    6 => Instruction::Ori(it),
                    7 => Instruction::Andi(it),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, it),
                }
            }
            0x17 => Instruction::Auipc(UType::from(instruction)),
            0x33 => {
                let rt = RType::from(instruction);

                match (rt.funct7, rt.funct3) {
                    (0x00, 0x0) => Instruction::Add(rt),
                    (0x00, 0x1) => Instruction::Sll(rt),
                    (0x00, 0x2) => Instruction::Slt(rt),
                    (0x00, 0x3) => Instruction::Sltu(rt),
                    (0x00, 0x4) => Instruction::Xor(rt),
                    (0x00, 0x5) => Instruction::Srl(rt),
                    (0x00, 0x6) => Instruction::Or(rt),
                    (0x00, 0x7) => Instruction::And(rt),

                    (0x20, 0x0) => Instruction::Sub(rt),
                    (0x20, 0x5) => Instruction::Sra(rt),

                    (0x01, 0x0) => Instruction::Mul(rt),
                    (0x01, 0x1) => Instruction::Mulh(rt),
                    (0x01, 0x4) => Instruction::Div(rt),
                    (0x01, 0x5) => Instruction::Divu(rt),
                    (0x01, 0x6) => Instruction::Rem(rt),
                    (0x01, 0x7) => Instruction::Remu(rt),
                    _ => Instruction::Invalid(instruction),
                }
            }
            0x37 => Instruction::Lui(UType::from(instruction)),
            0x73 => {
                let it = IType::from(instruction);
                match it.funct3 {
                    1 => Instruction::Csrrw(it),
                    2 => Instruction::Csrrs(it),
                    3 => Instruction::Csrrc(it),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, it),
                }
            }

            _ => unimplemented!("{:#010X} opcode={:02X}", instruction, opcode),
        }
    }
}
