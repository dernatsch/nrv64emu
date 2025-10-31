mod cpu;
mod decoder;
mod mem;
mod gdb;

use bytes::BufMut;

fn main() {
    println!("nrv64emu: waiting for gdb...");
    let mut debugger = gdb::GdbConnection::new(3000).unwrap();

    let kernel_bin = std::fs::read("./configs/opensbi/fw_jump.bin").unwrap();
    let dtb_bin = std::fs::read("./configs/virt.dtb").unwrap();

    let mut running = false;
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

    cpu.run(6875);

    loop {
        if let Some(packet) = debugger.read_packet().unwrap() {
            debugger.ack().unwrap();

            if packet.starts_with("qSupported") {
                debugger.send_packet("PacketSize=4000").unwrap();
            } else if packet.starts_with("qAttached") {
                debugger.send_packet("0").unwrap();
            } else if packet.starts_with("?") {
                debugger.send_packet("S05").unwrap();
            } else if packet.starts_with("qfThreadInfo") {
                debugger.send_packet("1").unwrap();
            } else if packet.starts_with("vCont?") {
                debugger.send_packet("vCont;c;s").unwrap();
            } else if packet.starts_with("qC") {
                debugger.send_packet("1").unwrap();
            } else if packet.starts_with("g") {
                let regs = cpu.debug_register_dump();
                debugger.send_packet(&hex::encode(regs)).unwrap();
            } else if packet.starts_with("s") {
                cpu.run(1);
                debugger.send_packet("S05").unwrap();
            } else if packet.starts_with("\x03") {
                running = false;
                debugger.send_packet("S05").unwrap();
            } else if packet.starts_with("c") {
                running = true;
            } else if packet.starts_with("p") {
                //TODO
                let idx = u8::from_str_radix(&packet[1..], 16).unwrap();
                let reg = cpu.debug_read_reg(idx as usize);
                debugger.send_packet(&hex::encode(reg.to_le_bytes())).unwrap();
            } else if packet.starts_with("m") {
                let (addr, len) = &packet[1..].split_once(',').unwrap();
                let addr = u64::from_str_radix(addr, 16).unwrap();
                let len = usize::from_str_radix(len, 16).unwrap();

                if let Some(mem) = cpu.debug_read_mem(addr, len) {
                    debugger.send_packet(&hex::encode(mem)).unwrap();
                } else {
                    debugger.send_packet("").unwrap();
                }
            } else if packet.starts_with("M") {
                let (addrlen, data) = packet[1..].split_once(":").unwrap();
                let (addr, len) = addrlen.split_once(",").unwrap();
                let addr = u64::from_str_radix(addr, 16).unwrap();
                let _len = usize::from_str_radix(len, 16).unwrap();
                let data = hex::decode(data).unwrap();

                if cpu.debug_write_mem(addr, &data) {
                    debugger.send_packet("OK").unwrap();
                } else {
                    debugger.send_packet("E00").unwrap();
                }
            } else {
                debugger.send_packet("").unwrap();
            }
        }

        if running {
            match cpu.run(10000) {
                cpu::HaltReason::Steps => {}
                x => {
                    running = false;
                    debugger.send_packet("S05").unwrap();
                }
            }
        }
    }
}
