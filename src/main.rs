mod cpu;
mod decoder;
mod mem;

use bytes::BufMut;

fn main() {
    let kernel_bin = std::fs::read("./configs/opensbi/fw_jump.bin").unwrap();
    let dtb_bin = std::fs::read("./configs/virt.dtb").unwrap();

    let mut cpu = cpu::Cpu::new();
    cpu.store_bytes(0x80000000, &kernel_bin);

    let fdt_base = 0x1000 + 8*8;
    cpu.store_bytes(fdt_base, &dtb_bin);

    // trampoline
    // jump_addr = 0x80000000
    //
    // auipc t0, jump_addr
    // auipc a1, dtb
    // addi a1, a1, dtb
    // csrr a0, mhartid
    // jalr zero, t0, jump_addr

    let mut trampoline: Vec<u8> = Vec::new();
    trampoline.put_u32_le(0x297 + 0x80000000 - 0x1000);
    trampoline.put_u32_le(0x597);
    trampoline.put_u32_le(0x58593 + ((fdt_base as u32 - 4) << 20));
    trampoline.put_u32_le(0xf1402573);
    trampoline.put_u32_le(0x00028067);

    cpu.store_bytes(0x1000, &trampoline);

    loop {
        cpu.step();
    }
}
