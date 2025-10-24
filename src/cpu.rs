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

    pub fn step(&mut self) {
        debug_assert!(self.regs[0] == 0);

        let insn = self.fetch_and_decode_insn(self.pc);

        match insn {
            _ => unimplemented!("{:?}", insn),
        }
    }

    fn fetch_and_decode_insn(&self, address: u64) -> Instruction {
        let ram_off = (address - self.ram_base) as usize;
        let bytes = &self.ram[ram_off..][..4];

        let instruction = u32::from_le_bytes(bytes.try_into().unwrap());

        Instruction::decode(instruction)
    }
}
