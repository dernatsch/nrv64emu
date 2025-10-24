use std::ops::Mul;

use crate::decoder::Instruction;

const CLINT_BASE: u64 = 0x02000000;
const PLIC_BASE: u64 = 0x0C000000;
const UART_BASE: u64 = 0x10000000;
const VIRTIO_DISK_BASE: u64 = 0x10001000;
const VIRTIO_NET_BASE: u64 = 0x10001000; //XXX: enough distance?

pub struct Cpu {
    ram_base: u64,
    ram: Vec<u8>, //TODO: bus abstraction

    // Interal state
    pc: u64,
    regs: [u64; 32],
}

impl Cpu {
    pub fn new() -> Self {
        let mut ram = Vec::new();
        ram.resize(128 * 1024 * 1024, 0xAA);

        Cpu {
            ram_base: 0x80000000,
            ram: ram,

            pc: 0x80000000,
            regs: [0; 32],
        }
    }

    pub fn load_bytes(&mut self, offset: u64, bytes: &[u8]) {
        let ram_off = (offset - self.ram_base) as usize;
        self.ram[ram_off..][..bytes.len()].copy_from_slice(bytes);
    }

    fn read_csr(&mut self, csr: u32) -> u64 {
        match csr {
            0xF14 => 0, // mhartid
            _ => unimplemented!("csr {:03X} read, cause exception!", csr),
        }
    }

    fn write_csr(&mut self, csr: u32, val: u64) {}

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
                self.regs[i.rd as usize] = self.regs[i.rs1 as usize].wrapping_add(i.imm as u64);
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
                self.regs[r.rd as usize] = opa.wrapping_mul(opb);
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
