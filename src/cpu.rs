use crate::{decoder::Instruction, mem::Memory};

pub enum HaltReason {
    Halt,
    Steps,
    Breakpoint(u64),
}

const CLINT_BASE: u64 = 0x02000000;
const PLIC_BASE: u64 = 0x0C000000;
const UART_BASE: u64 = 0x10000000;
const UART_SIZE: u64 = 0x100;
const VIRTIO_DISK_BASE: u64 = 0x10001000;
const VIRTIO_NET_BASE: u64 = 0x10001000; //XXX: enough distance?

fn rtc_time() -> u64 {
    let ts = std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap();
    (ts.as_nanos() / 100) as u64
}

pub struct Cpu {
    ram: Memory,

    // Interal state
    pc: u64,
    regs: [u64; 32],

    privl: u8,

    pmpcfg: [u64; 4],
    pmpaddr: [u64; 64],

    mstatus: u64,
    misa: u64,
    medeleg: u64,
    mideleg: u64,
    mie: u64,
    mip: u64,
    mtvec: u64,
    mcounteren: u64,
    menvcfg: u64,
    mscratch: u64,
    mepc: u64,

    satp: u64,
    stimecmp: u64,
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
    | (1<<5)    // F
    | (1<<3);    // D
    // | 1<<2;   // C


const MISA_RV64GC: u64 = (2 << 62) // XLEN=64
    | (1<<18)   // S
    | (1<<20)   // U
    | (1<<8)    // I
    | (1<<12)   // M
    | (1<<0)    // A
    | (1<<5)    // F
    | (1<<3)    // D
    | 1<<2;   // C

const SSTATUS_MASK: u64 = 0x30000de122;


impl Cpu {
    pub fn new() -> Self {
        let mut ram = Memory::new();
        ram.add_region(0x80000000, 128 * 1024 * 1024);
        ram.add_region(0x1000, 0x10000-0x1000);

        Cpu {
            ram: ram,

            pc: 0x1000, // trampoline is here
            regs: [0; 32],

            privl: 3,

            pmpcfg: [0; 4],
            pmpaddr: [0; 64],

            mstatus: 0,
            misa: MISA_RV64G,
            medeleg: 0,
            mideleg: 0,
            mie: 0,
            mip: 0,
            mtvec: 0,
            mcounteren: 0,
            menvcfg: 0,
            mscratch: 0,
            mepc: 0,

            satp: 0,
            stimecmp: 0,
        }
    }

    pub fn store_bytes(&mut self, offset: u64, bytes: &[u8]) -> bool {
        // println!("store @ {:#X}..{:#X}", offset, offset as usize+bytes.len());
        match self.ram.get_slice_mut(offset, bytes.len()) {
            Some(slice) => { slice.copy_from_slice(bytes);  true }
            None => false,
        }
    }

    pub fn debug_register_dump(&self) -> Vec<u8> {
        let mut dump = Vec::new();

        for x in self.regs.iter() {
            dump.extend_from_slice(&x.to_le_bytes());
        }

        dump.extend_from_slice(&self.pc.to_le_bytes());

        //TODO floating point regs
        for _ in 0..32 {
            dump.extend_from_slice(&0u64.to_le_bytes());
        }

        dump
    }

    pub fn debug_read_reg(&self, idx: usize) -> u64 {
        self.regs[idx]
    }

    pub fn debug_write_mem(&mut self, addr: u64, data: &[u8]) -> bool {
        self.store_bytes(addr, data)
    }

    pub fn debug_read_mem(&self, addr: u64, len: usize) -> Option<Vec<u8>> {
        //TODO: mmio
        Some(self.ram.get_slice(addr, len)?.into())
    }

    fn read_csr(&mut self, csr: u32) -> u64 {
        match csr {
            0x100 => self.mstatus & SSTATUS_MASK,
            0x104 => self.mie & self.mideleg, // sie
            0x14D => self.stimecmp,
            0x180 => self.satp,
            0x300 => self.mstatus,
            0x301 => self.misa,
            0x302 => self.medeleg,
            0x303 => self.mideleg,
            0x304 => self.mie,
            0x305 => self.mtvec,
            0x306 => self.mcounteren,
            0x30a => self.menvcfg,
            0x320 => 0,
            0x3a0..=0x3a3 => self.pmpcfg[(csr & 0x0f) as usize],

            0x3b0..=0x3ff => self.pmpaddr[(csr & 0x3f) as usize],
            0x340 => self.mscratch,
            0x341 => self.mepc,
            0x344 => self.mip,
            0xB00..=0xB9F => 0,
            0xC01 => rtc_time(),
            0xF14 => 0, // mhartid
            _ => unimplemented!("csr {:03X} read, cause exception!", csr),
        }
    }

    fn write_csr(&mut self, csr: u32, val: u64) -> bool {
        match csr {
            0x100 => {
                self.mstatus &= !SSTATUS_MASK;
                self.mstatus |= val & SSTATUS_MASK;
            }
            0x104 => {
                let mask = self.mideleg;
                self.mie = (val & mask) | (self.mie & !mask);
            }
            0x14D => { self.stimecmp = val; }
            0x180 => { self.satp = val; println!("satp: {:#018X}", self.satp); } //TODO
            0x300 => { self.mstatus = val; }
            0x302 => { self.medeleg = val; }
            0x303 => { self.mideleg = val; }
            0x304 => { self.mie = val; }
            0x305 => { self.mtvec = val; }
            0x306 => { self.mcounteren = val; }
            0x30a => { self.menvcfg = val; }
            0x340 => { self.mscratch = val; }
            0x341 => { self.mepc = val; }
            0x344 => { self.mip = val; }
            0x3a0..=0x3a3 => { self.pmpcfg[(csr & 0x0f) as usize] = val; }
            0x3b0..=0x3ff => { self.pmpaddr[(csr & 0x3f) as usize] = val; }
            0xB00..=0xB9F => {}
            _ => unimplemented!("csr {:03X} write, cause exception!", csr),
        }

        true
    }

    fn uart_store_u8(&mut self, address: u64, value: u8) -> bool {
        match address {
            0 => { print!("{}", value as char); }
            _ => {}
        }

        true
    }

    fn uart_load_u8(&mut self, address: u64) -> Option<u8> {
        match address {
            0x05 => Some(0x60),
            _ => None,
        }
    }

    fn store_u8(&mut self, address: u64, value: u8) -> bool {
        // bounds
        //TODO: handle MMIO
        if (UART_BASE..UART_BASE+UART_SIZE).contains(&address) {
            return self.uart_store_u8(address - UART_BASE, value);
        }

        self.store_bytes(address, &value.to_le_bytes())
    }
    fn store_u16(&mut self, address: u64, value: u16) -> bool {
        // alignment
        if address % 2 != 0 {
            return false;
        }

        self.store_bytes(address, &value.to_le_bytes())
    }
    fn store_u32(&mut self, address: u64, value: u32) -> bool {
        // alignment
        if address % 4 != 0 {
            return false;
        }

        self.store_bytes(address, &value.to_le_bytes())
    }

    fn store_u64(&mut self, address: u64, value: u64) -> bool{
        // alignment
        if address % 8 != 0 {
            return false;
        }

        self.store_bytes(address, &value.to_le_bytes())
    }

    fn load_u8(&mut self, address: u64) -> Option<u8> {
        if (UART_BASE..UART_BASE+UART_SIZE).contains(&address) {
            return self.uart_load_u8(address - UART_BASE);
        }

        let slice = self.ram.get_slice(address, 1)?;

        Some(u8::from_le_bytes(slice.try_into().unwrap()))
    }
    fn load_u16(&mut self, address: u64) -> Option<u16> {
        // alignment
        if address % 2 != 0 {
            return None;
        }

        let slice = self.ram.get_slice(address, 2)?;
        Some(u16::from_le_bytes(slice.try_into().unwrap()))
    }
    fn load_u32(&mut self, address: u64) -> Option<u32> {
        // alignment
        if address % 4 != 0 {
            return None;
        }

        let slice = self.ram.get_slice(address, 4)?;
        Some(u32::from_le_bytes(slice.try_into().unwrap()))
    }

    fn load_u64(&mut self, address: u64) -> Option<u64> {
        // alignment
        if address % 8 != 0 {
            return None;
        }

        let slice = self.ram.get_slice(address, 8)?;
        Some(u64::from_le_bytes(slice.try_into().unwrap()))
    }

    pub fn step(&mut self) -> Option<HaltReason> {
        debug_assert!(self.regs[0] == 0);

        println!("{:#010X}", self.pc);

        let insn = self.fetch_and_decode_insn(self.pc).unwrap();
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
            Instruction::Addiw(i) => {
                let res = (self.regs[i.rs1 as usize] as u32).wrapping_add_signed(i.imm as i32) as i64;
                if i.rd != 0 {
                    self.regs[i.rd as usize] = ((res << 32) >> 32) as u64;
                }
                self.pc += 4;
            }
            Instruction::Slliw(i) => {
                let res = self.regs[i.rs1 as usize] << (i.imm & 0x1F) as i64;
                let res = ((res << 32) >> 32) as u64;
                if i.rd != 0 {
                    self.regs[i.rd as usize] = res;
                }
                self.pc += 4;
            }
            Instruction::Srliw(i) => {
                let res = self.regs[i.rs1 as usize] >> (i.imm & 0x1F) as i64;
                let res = ((res << 32) >> 32) as u64;
                if i.rd != 0 {
                    self.regs[i.rd as usize] = res;
                }
                self.pc += 4;
            }
            Instruction::Addw(r) => {
                let opa = self.regs[r.rs1 as usize] as i32;
                let opb = self.regs[r.rs2 as usize] as i32;
                let res = opa.wrapping_add(opb) as i64;
                if r.rd != 0 {
                    self.regs[r.rd as usize] = res as u64;
                }
                self.pc += 4;
            }
            Instruction::Mulw(r) => {
                let opa = self.regs[r.rs1 as usize] as i32;
                let opb = self.regs[r.rs2 as usize] as i32;
                let res = opa.wrapping_mul(opb) as i64;
                if r.rd != 0 {
                    self.regs[r.rd as usize] = res as u64;
                }
                self.pc += 4;
            }
            Instruction::Subw(r) => {
                let opa = self.regs[r.rs1 as usize] as i32;
                let opb = self.regs[r.rs2 as usize] as i32;
                let res = opa.wrapping_sub(opb) as i64;
                if r.rd != 0 {
                    self.regs[r.rd as usize] = res as u64;
                }
                self.pc += 4;
            }
            Instruction::Sllw(r) => {
                let opa = self.regs[r.rs1 as usize] as i32;
                let opb = self.regs[r.rs2 as usize] as i32 & 0x1F;
                let res = opa << opb as i64;
                if r.rd != 0 {
                    self.regs[r.rd as usize] = res as u64;
                }
                self.pc += 4;
            }
            Instruction::Andi(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] & i.imm as u64;
                }
                self.pc += 4;
            }
            Instruction::Ori(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] | i.imm as u64;
                }
                self.pc += 4;
            }
            Instruction::Xori(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] ^ i.imm as u64;
                }
                self.pc += 4;
            }
            Instruction::Slli(i) => {
                let res = self.regs[i.rs1 as usize] << (i.imm as u64 & 0x3F);
                if i.rd != 0 {
                    self.regs[i.rd as usize] = res;
                }
                self.pc += 4;
            }
            Instruction::Srli(i) => {
                if i.rd != 0 {
                    self.regs[i.rd as usize] = self.regs[i.rs1 as usize] >> (i.imm as u64 & 0x3F);
                }
                self.pc += 4;
            }
            Instruction::Srai(i) => {
                let shamt = i.imm as u64 & 0x3F;
                let res = self.regs[i.rs1 as usize] as i64 >> shamt;
                if i.rd != 0 {
                    self.regs[i.rd as usize] = res as u64;
                }
                self.pc += 4;
            }
            Instruction::Slti(i) => {
                let val = self.regs[i.rs1 as usize] as i64;
                if i.rd != 0 {
                    self.regs[i.rd as usize] = if val < i.imm as i64 { 1 } else { 0 };
                }
                self.pc += 4;
            }
            Instruction::Sltiu(i) => {
                let val = self.regs[i.rs1 as usize];
                if i.rd != 0 {
                    self.regs[i.rd as usize] = if val < i.imm as u64 { 1 } else { 0 };
                }
                self.pc += 4;
            }
            Instruction::Slt(r) => {
                let val1 = self.regs[r.rs1 as usize] as i64;
                let val2 = self.regs[r.rs2 as usize] as i64;
                if r.rd != 0 {
                    self.regs[r.rd as usize] = if val1 < val2 { 1 } else { 0 };
                }
                self.pc += 4;
            }
            Instruction::Sltu(r) => {
                let val1 = self.regs[r.rs1 as usize];
                let val2 = self.regs[r.rs2 as usize];
                if r.rd != 0 {
                    self.regs[r.rd as usize] = if val1 < val2 { 1 } else { 0 };
                }
                self.pc += 4;
            }
            Instruction::Csrrw(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                self.write_csr(csrid, val);
                if i.rd != 0 {
                    self.regs[i.rd as usize] = csr;
                }
                self.pc += 4;
            }
            Instruction::Csrrwi(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let imm = i.rs1 as u64;
                let csr = self.read_csr(csrid);
                self.write_csr(csrid, imm);
                if i.rd != 0 {
                    self.regs[i.rd as usize] = csr;
                }
                self.pc += 4;
            }
            Instruction::Csrrs(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                if val != 0 {
                    self.write_csr(csrid, val | csr);
                }
                if i.rd != 0 {
                    self.regs[i.rd as usize] = csr;
                }
                self.pc += 4;
            }
            Instruction::Csrrc(i) => {
                let csrid = i.imm as u32 & 0xfff;
                let val = self.regs[i.rs1 as usize];
                let csr = self.read_csr(csrid);
                if val != 0 {
                    self.write_csr(csrid, val & !csr);
                }
                if i.rd != 0 {
                    self.regs[i.rd as usize] = csr;
                }
                self.pc += 4;
            }
            Instruction::Add(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa.wrapping_add(opb);
                self.pc += 4;
            }
            Instruction::Sub(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa.wrapping_sub(opb);
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
            Instruction::Srl(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize] & 0x3F;
                self.regs[r.rd as usize] = opa >> opb;
                self.pc += 4;
            }
            Instruction::Sll(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize] & 0x3F;
                self.regs[r.rd as usize] = opa << opb;
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
            Instruction::Divu(r) => {
                let opa = self.regs[r.rs1 as usize];
                let opb = self.regs[r.rs2 as usize];
                self.regs[r.rd as usize] = opa / opb;
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
                    0 => self.load_u8(addr).map(|x| x as i8 as i64),
                    1 => self.load_u16(addr).map(|x| x as i16 as i64),
                    2 => self.load_u32(addr).map(|x| x as i32 as i64),
                    3 => self.load_u64(addr).map(|x| x as i64),
                    4 => self.load_u8(addr).map(|x| x as u64 as i64),
                    5 => self.load_u16(addr).map(|x| x as u64 as i64),
                    6 => self.load_u32(addr).map(|x| x as u64 as i64),
                    7 => self.load_u64(addr).map(|x| x as i64),
                    _ => unimplemented!("pc={:08X} {:X?}", self.pc, insn),
                };

                if let Some(val) = val {
                    self.regs[i.rd as usize] = val as u64;
                    self.pc += 4;
                } else {
                    unimplemented!("load exception pc={:08X} addr={:#010X?}", self.pc, addr);
                }
            }
            Instruction::Mret(i) => {
                let mpp = (self.mstatus >> 11) & 3;
                let mpie = (self.mstatus >> 7) & 1;

                self.mstatus &= !(1 << mpp);
                self.mstatus |= mpie << mpp;

                // set MPIE
                self.mstatus |= 1 << 7;

                self.mstatus &= !(3 << 11);
                self.privl = mpp as u8;
                self.pc = self.mepc;

            }
            Instruction::Beq(b) => {
                let cond = self.regs[b.rs1 as usize] == self.regs[b.rs2 as usize];
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Bne(b) => {
                let cond = self.regs[b.rs1 as usize] != self.regs[b.rs2 as usize];
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Blt(b) => {
                let cond = (self.regs[b.rs1 as usize] as i64) < self.regs[b.rs2 as usize] as i64;
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Bge(b) => {
                let cond = (self.regs[b.rs1 as usize] as i64) >= self.regs[b.rs2 as usize] as i64;
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Bltu(b) => {
                let cond = self.regs[b.rs1 as usize] < self.regs[b.rs2 as usize];
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Bgeu(b) => {
                let cond = self.regs[b.rs1 as usize] >= self.regs[b.rs2 as usize];
                if cond {
                    let target = self.pc.wrapping_add_signed(b.imm as i64);
                    self.pc = target;
                } else {
                    self.pc += 4;
                }
            }
            Instruction::Amoswapw(r) => {
                let addr = self.regs[r.rs1 as usize];
                let val = self.regs[r.rs2 as usize];
                let memval = self.load_u32(addr).unwrap_or_else(|| unimplemented!("load exception pc={:08X} addr={:#010X?}", self.pc, addr));
                if !self.store_u32(addr, val as u32) {
                        unimplemented!("store exception pc={:08X} addr={:#010X?}", self.pc, addr);
                }

                if r.rd != 0 {
                    self.regs[r.rd as usize] = ((memval as i64) << 32 >> 32) as u64;
                }
                self.pc += 4;
            }
            Instruction::Amoaddw(r) => {
                let addr = self.regs[r.rs1 as usize];
                let val = self.regs[r.rs2 as usize] as u32;
                let memval = self.load_u32(addr).unwrap_or_else(|| unimplemented!("load exception pc={:08X} addr={:#010X?}", self.pc, addr));
                if !self.store_u32(addr, memval + val) {
                        unimplemented!("store exception pc={:08X} addr={:#010X?}", self.pc, addr);
                }

                if r.rd != 0 {
                    self.regs[r.rd as usize] = ((memval as i64) << 32 >> 32) as u64;
                }
                self.pc += 4;
            }
            Instruction::Amoswapd(r) => {
                let addr = self.regs[r.rs1 as usize];
                let val = self.regs[r.rs2 as usize];
                let memval = self.load_u64(addr).unwrap_or_else(|| unimplemented!("load exception pc={:08X} addr={:#010X?}", self.pc, addr));
                if !self.store_u64(addr, val) {
                        unimplemented!("store exception pc={:08X} addr={:#010X?}", self.pc, addr);
                }

                if r.rd != 0 {
                    self.regs[r.rd as usize] = memval;
                }
                self.pc += 4;
            }
            Instruction::Fence => {
                self.pc += 4;
            }
            Instruction::Ebreak => {
                self.pc += 4;
                // return Some(HaltReason::Breakpoint(self.pc));
            }
            _ => unimplemented!("pc={:08X} {:X?}", self.pc, insn),
        }

        None
    }

    fn fetch_and_decode_insn(&mut self, address: u64) -> Option<Instruction> {
        let instruction = self.load_u32(address)?;
        Some(Instruction::decode(instruction))
    }

    pub fn run(&mut self, maxsteps: usize) -> HaltReason {
        for _ in 0..maxsteps {
            if let Some(hr) = self.step() {
                return hr;
            }
        }

        HaltReason::Steps
    }
}
