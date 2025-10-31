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

impl From<u32> for SType {
    fn from(instruction: u32) -> Self {
        let opcode = (instruction & 0x7F) as u8;
        let funct3 = ((instruction >> 12) & 0x07) as u8;
        let rs1 = ((instruction >> 15) & 0x1F) as u8;
        let rs2 = ((instruction >> 20) & 0x1F) as u8;

        let imm = ((instruction as i32 >> 7) & 0x1F)
            | (instruction as i32 >> 20) & !0x1F;

        Self {
            opcode,
            funct3,
            rs1,
            rs2,
            imm,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BType {
    pub opcode: u8,
    pub funct3: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub imm: i32,
}

impl From<u32> for BType {
    fn from(instruction: u32) -> Self {
        let opcode = (instruction & 0x7F) as u8;
        let funct3 = ((instruction >> 12) & 0x07) as u8;
        let rs1 = ((instruction >> 15) & 0x1F) as u8;
        let rs2 = ((instruction >> 20) & 0x1F) as u8;

        let imm = decode_b_imm(instruction);

        Self {
            opcode: opcode as u8,
            funct3,
            rs1,
            rs2,
            imm
        }
    }
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

impl From<u32> for JType {
    fn from(instruction: u32) -> Self {
        let opcode = instruction & 0x7F;
        let rd = ((instruction >> 7) & 0x1F) as u8;

        Self {
            opcode: opcode as u8,
            rd,
            imm: decode_i_imm(instruction),
        }
    }
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
    Srai(IType),
    Srli(IType),
    Xori(IType),
    Ori(IType),
    Andi(IType),

    Addiw(IType),
    Slliw(IType),
    Srliw(IType),
    Addw(RType),

    // CSR
    Csrrw(IType),
    Csrrs(IType),
    Csrrc(IType),
    Csrrwi(IType),

    Mret(IType),
    Sret(IType),
    Wfi(IType),

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

    // Load/Store
    Load(IType),
    Store(SType),

    Ebreak,
    Ecall,

    // M
    Mul(RType),
    Mulh(RType),
    Div(RType),
    Divu(RType),
    Rem(RType),
    Remu(RType),

    // Jumps
    Jal(JType),
    Jalr(IType),

    // Branches
    Beq(BType),
    Bne(BType),
    Blt(BType),
    Bge(BType),
    Bltu(BType),
    Bgeu(BType),

    // A
    Amoswapw(RType),

    Fence,

    Invalid(u32),
}

fn decode_u_imm(instruction: u32) -> i32 {
    (instruction & 0xfffff800) as i32
}

fn decode_i_imm(instruction: u32) -> i32 {
    let insn = instruction as i32;

    ((insn & 0x80000000u32 as i32) >> 11)
        | ((insn >> 20) & 0x7FE)
        | ((insn >> 9) & 0x800)
        | insn & 0xff000
}

fn decode_b_imm(instruction: u32) -> i32 {
    let imm_12 = (instruction >> 31) & 1;
    let imm_11 = (instruction >> 7) & 1;
    let imm_10_5 = (instruction >> 25) & 0x3F;
    let imm_4_1 = (instruction >> 8) & 0xF;
    
    let immediate = ((imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1)) as i32;
    
    // Sign extend
    immediate << 19 >> 19
}

impl Instruction {
    pub fn decode(instruction: u32) -> Self {
        let opcode = instruction & 0x7F;

        match opcode {
            0x03 => Instruction::Load(IType::from(instruction)),
            0x0F => Instruction::Fence,
            0x13 => {
                let it = IType::from(instruction);
                match it.funct3 {
                    0 => Instruction::Addi(it),
                    1 => Instruction::Slli(it),
                    2 => Instruction::Slti(it),
                    3 => Instruction::Sltiu(it),
                    4 => Instruction::Xori(it),
                    5 => {
                        let imm = it.imm & 0x1F;
                        let shifttype = it.imm >> 5;

                        if shifttype == 0x20 {
                            Instruction::Srai(it)
                        } else {
                            Instruction::Srli(it)
                        }
                    },
                    6 => Instruction::Ori(it),
                    7 => Instruction::Andi(it),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, it),
                }
            }
            0x1b => {
                let it = IType::from(instruction);
                match it.funct3 {
                    0 => Instruction::Addiw(it),
                    1 => Instruction::Slliw(it),
                    5 => Instruction::Srliw(it),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, it),
                }
            }
            0x17 => Instruction::Auipc(UType::from(instruction)),
            0x23 => Instruction::Store(SType::from(instruction)),
            0x2F => {
                let rt = RType::from(instruction);
                
                match (rt.funct3, rt.funct7 >> 2) {
                    (2, 1) => Instruction::Amoswapw(rt),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, rt),
                }
            }
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
            0x3B => {
                let rt = RType::from(instruction);
                match (rt.funct3, rt.funct7) {
                    (0x0, 0x00) => Instruction::Addw(rt),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, rt),
                }
            }
            0x63 => {
                let bt = BType::from(instruction);
                match bt.funct3 {
                    0 => Instruction::Beq(bt),
                    1 => Instruction::Bne(bt),
                    4 => Instruction::Blt(bt),
                    5 => Instruction::Bge(bt),
                    6 => Instruction::Bltu(bt),
                    7 => Instruction::Bgeu(bt),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, bt),
                }
            }
            0x67 => Instruction::Jalr(IType::from(instruction)),
            0x6F => Instruction::Jal(JType::from(instruction)),
            0x73 => {
                let it = IType::from(instruction);
                match (it.funct3, it.imm) {
                    (0, 0x000) => Instruction::Ecall,
                    (0, 0x001) => Instruction::Ebreak,
                    (0, 0x102) => Instruction::Sret(it),
                    (0, 0x105) => Instruction::Wfi(it),
                    (0, 0x302) => Instruction::Mret(it),
                    (1, _) => Instruction::Csrrw(it),
                    (2, _) => Instruction::Csrrs(it),
                    (3, _) => Instruction::Csrrc(it),
                    (5, _) => Instruction::Csrrwi(it),
                    _ => unimplemented!("{:#010X} {:X?}", instruction, it),
                }
            }

            _ => unimplemented!("{:#010X} opcode={:02X}", instruction, opcode),
        }
    }
}
