use std::collections::BTreeMap;

pub struct Memory {
    regions: BTreeMap<u64, Box<[u8]>>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
        }
    }

    pub fn add_region(&mut self, base: u64, size: usize) {
        let mut mem = Vec::with_capacity(size);
        mem.resize(size, 0u8);
        self.regions.insert(base, mem.into_boxed_slice());
    }

    // Find the base address and offset within the region
    fn find_region(&self, addr: u64) -> Option<(u64, u64)> {
        if let Some((&base, data)) = self.regions.range(..=addr).next_back() {
            if addr < base + data.len() as u64 {
                return Some((base, addr - base));
            }
        }

        None
    }

    pub fn get_slice(&self, addr: u64, len: usize) -> Option<&[u8]> {
        let (base, off) = self.find_region(addr)?;
        let mem = self.regions.get(&base)?;
        Some(&mem[off as usize..][..len])
    }

    pub fn get_slice_mut(&mut self, addr: u64, len: usize) -> Option<&mut [u8]> {
        let (base, off) = self.find_region(addr)?;
        let mem = self.regions.get_mut(&base)?;
        Some(&mut mem[off as usize..][..len])
    }
}
