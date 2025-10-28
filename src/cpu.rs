use crate::decoder::Instruction;

const CLINT_BASE: u64 = 0x02000000;
const PLIC_BASE: u64 = 0x0C000000;
const UART_BASE: u64 = 0x10000000;
const VIRTIO_DISK_BASE: u64 = 0x10001000;
const VIRTIO_NET_BASE: u64 = 0x10001000; //XXX: enough distance?

fn rtc_time() -> u64 {
    let ts = std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap();
    (ts.as_nanos() / 100) as u64
}

pub struct Cpu {
    ram_base: u64,
    ram: Vec<u8>, //TODO: bus abstraction

    // Interal state
    pc: u64,
    regs: [u64; 32],

    pmpcfg: [u64; 4],
    pmpaddr: [u64; 64],

    mstatus: u64,
    misa: u64,
    medeleg: u64,
    mideleg: u64,
    mie: u64,
    mtvec: u64,
    mcounteren: u64,
    menvcfg: u64,
    mepc: u64,

    satp: u64,
}

impl std::fmt::Debug for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cpu")
            .field("pc", &self.pc)
            .field("regs", &self.regs)
            .finish()
    }
}

const MISA_RV64G: u64 = (2 << 62) // XLEN=64
    | (1<<18)   // S
    | (1<<20)   // U
    | (1<<8)    // I
    | (1<<12)   // M
    | (1<<0)    // A
    // | (1<<5)    // F
    // | (1<<3)    // D
    | 1<<2;   // C

impl Cpu {
    pub fn new() -> Self {
        let mut ram = Vec::new();
        ram.resize(128 * 1024 * 1024, 0xAA);

        Cpu {
            ram_base: 0x80000000,
            ram: ram,

            pc: 0x80000000,
            regs: [0; 32],

            pmpcfg: [0; 4],
            pmpaddr: [0; 64],

            mstatus: 0,
            misa: 0,
            medeleg: 0,
            mideleg: 0,
            mie: 0,
            mtvec: 0,
            mcounteren: 0,
            menvcfg: 0,
            mepc: 0,

            satp: 0,
        }
    }

    pub fn load_bytes(&mut self, offset: u64, bytes: &[u8]) {
        let ram_off = (offset - self.ram_base) as usize;
        self.ram[ram_off..][..bytes.len()].copy_from_slice(bytes);
    }

    fn read_csr(&mut self, csr: u32) -> u64 {
        match csr {
            0x104 => self.mie & self.mideleg, // sie
            0x180 => self.satp,
            0x300 => self.mstatus,
            0x301 => self.misa,
            0x302 => self.medeleg,
            0x303 => self.mideleg,
            0x304 => self.mie,
            0x305 => self.mtvec,
            0x306 => self.mcounteren,
            0x30a => self.menvcfg,
            0x3a0..=0x3a3 => self.pmpcfg[(csr & 0x0f) as usize],

            0x3b0..=0x3ff => self.pmpaddr[(csr & 0x3f) as usize],
            0x341 => self.mepc,
            0xC01 => rtc_time(),
            0xF14 => 0, // mhartid
            _ => unimplemented!("csr {:03X} read, cause exception!", csr),
        }
    }

    fn write_csr(&mut self, csr: u32, val: u64) {}

    fn store_u8(&mut self, address: u64, value: u8) -> bool { unimplemented!("store_u8") }
    fn store_u16(&mut self, address: u64, value: u16) -> bool { unimplemented!("store_u16") }
    fn store_u32(&mut self, address: u64, value: u32) -> bool { unimplemented!("store_u32") }

    fn store_u64(&mut self, address: u64, value: u64) -> bool{
        // alignment
        if address % 8 != 0 {
            return false;
        }

        // bounds
        //TODO: handle MMIO
        if address < self.ram_base || address >= (self.ram_base + self.ram.len() as u64) {
            return false;
        }

        let ram_off = (address - self.ram_base) as usize;
        self.ram[ram_off..][..8].copy_from_slice(&value.to_le_bytes());

        true
    }

    fn load_u8(&mut self, address: u64) -> Option<u64> { unimplemented!("load") }
    fn load_u16(&mut self, address: u64) -> Option<u64> { unimplemented!("load") }
    fn load_u32(&mut self, address: u64) -> Option<u64> { unimplemented!("load") }

    fn load_u64(&mut self, address: u64) -> Option<u64> {
        // alignment
        if address % 8 != 0 {
            return None;
        }

        // bounds
        //TODO: handle MMIO
        if address < self.ram_base || address >= (self.ram_base + self.ram.len() as u64) {
            return None;
        }

        let ram_off = (address - self.ram_base) as usize;
        Some(u64::from_le_bytes(self.ram[ram_off..][..8].try_into().unwrap()))
    }

    pub fn step(&mut self) {
        debug_assert!(self.regs[0] == 0);

        let insn = self.fetch_and_decode_insn(self.pc);
        match insn {
            Instruction::Auipc(u) => {
                self.regs[u.rd as usize] = self.pc.wrapping_add(u.imm as u64);
                self.pc += 4;
            }
            Instruction::Lui(u) => {
                if u.rd != 0 {
                    self.regs[u.rd as usize] = u.imm as i64 as u64;
                }
                self.pc += 4;
            }
            Instruction::Addi(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize].wrapping_add(i.imm as u64);
                }
                self.pc += 4;
            }
            Instruction::Ori(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] | i.imm as u64;
                }
                self.pc += 4;
            }
            Instruction::Slli(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] << (i.imm as u64 & 0x1F);
                }
                self.pc += 4;
            }
            Instruction::Srli(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] >> (i.imm as u64 & 0x1F);
                }
                self.pc += 4;
            }
            Instruction::Csrrw(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                self.write_csr(csrid, val);
                self.regs[i.rd as usize] = csr;
                self.pc += 4;
            }
            Instruction::Csrrs(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                self.write_csr(csrid, val | csr);
                self.regs[i.rd as usize] = csr;
                self.pc += 4;
            }
            Instruction::Csrrc(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                self.write_csr(csrid, val & !csr);
                self.regs[i.rd as usize] = csr;
                self.pc += 4;
            }
            Instruction::Add(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa.wrapping_add(opb);
                self.pc += 4;
            }
            Instruction::And(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa & opb;
                self.pc += 4;
            }
            Instruction::Or(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa | opb;
                self.pc += 4;
            }
            Instruction::Xor(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa ^ opb;
                self.pc += 4;
            }
            Instruction::Mul(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa * opb;
                self.pc += 4;
            }
            Instruction::Mulh(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = ((opa as u128 * opb as u128) >> 64) as u64;
                self.pc += 4;
            }
            Instruction::Jal(j) => {
                if j.rd != 0 {
                    self.regs[j.rd as usize] = self.pc + 4;
                }

                let target = self.pc.wrapping_add_signed(j.imm as i64);
                self.pc = target;
            }
            Instruction::Jalr(i) => {
                let val = self.pc + 4;
                let target = self.regs[i.rs1 as usize].wrapping_add_signed(i.imm as i64) & !1;
                self.pc = target;

                if i.rd != 0 {
                    self.regs[i.rd as usize] = val;
                }
            }
            Instruction::Store(s) => {
                // size
                let addr = self.regs[s.rs1 as usize].wrapping_add_signed(s.imm as i64);
                let val = self.regs[s.rs2 as usize];
                let retired = match s.funct3 {
                    0 => self.store_u8(addr, val as u8),
                    1 => self.store_u16(addr, val as u16),
                    2 => self.store_u32(addr, val as u32),
                    3 => self.store_u64(addr, val),
                    _ => unimplemented!("pc={:08X} {:X?}", self.pc, insn),
                };

                if retired {
                    self.pc += 4;
                } else {
                    unimplemented!("store exception pc={:08X} addr={:#010X?} val={:#X?}", self.pc, addr, val);
                }
            }
            Instruction::Load(i) => {
                let addr = self.regs[i.rs1 as usize].wrapping_add_signed(i.imm as i64);
                let val = match i.funct3 {
                    0 => self.load_u8(addr),
                    1 => self.load_u16(addr),
                    2 => self.load_u32(addr),
                    3 => self.load_u64(addr),
                    _ => unimplemented!("pc={:08X} {:X?}", self.pc, insn),
                };

                if let Some(val) = val {
                    self.regs[i.rd as usize] = val;
                    self.pc += 4;
                } else {
                    unimplemented!("load exception pc={:08X} addr={:#010X?}", self.pc, addr);
                }
            }
            _ => unimplemented!("pc={:08X} {:X?}", self.pc, insn),
        }
    }

    fn fetch_and_decode_insn(&self, address: u64) -> Instruction {
        let ram_off = (address - self.ram_base) as usize;
        let bytes = &self.ram[ram_off..][..4];

        let instruction = u32::from_le_bytes(bytes.try_into().unwrap());

        Instruction::decode(instruction)
    }
}
