mod cpu;
mod decoder;

fn main() {
    let kernel_bin = std::fs::read("./configs/xv6/kernel.bin").unwrap();

    let mut cpu = cpu::Cpu::new();
    cpu.load_bytes(0x80000000, &kernel_bin);

    loop {
        cpu.step();
    }
}
