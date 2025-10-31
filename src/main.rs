mod cpu;
mod decoder;
mod mem;

fn main() {
    let kernel_bin = std::fs::read("./configs/opensbi/fw_jump.bin").unwrap();
    let dtb_bin = std::fs::read("./configs/virt.dtb").unwrap();

    let mut cpu = cpu::Cpu::new();
    cpu.store_bytes(0x80000000, &kernel_bin);

    let fdt_base = 0x1000 + 8*8;
    cpu.store_bytes(fdt_base, &dtb_bin);

    loop {
        cpu.step();
    }
}
