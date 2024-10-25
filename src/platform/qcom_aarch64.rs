use crate::{arch::zone::HvArchZoneConfig, config::*};

pub const ROOT_ZONE_DTB_ADDR: u64 = 0xa0000000;
pub const ROOT_ZONE_KERNEL_ADDR: u64 = 0xa0400000;
pub const ROOT_ZONE_ENTRY: u64 = 0xa0400000;
pub const ROOT_ZONE_CPUS: u64 = (1 << 0) | (1 << 1);

pub const ROOT_ZONE_MEMORY_REGIONS: [HvConfigMemoryRegion; 5] = [
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_RAM,
        physical_start: 0x80000000,
        virtual_start: 0x80000000,
        size: 0x3a800000,
    },
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_RAM,
        physical_start: 0xc0000000,
        virtual_start: 0xc0000000,
        size: 0x01800000,
    },
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_RAM,
        physical_start: 0xc3400000,
        virtual_start: 0xc3400000,
        size: 0x3cc00000,
    },
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_RAM,
        physical_start: 0x100000000,
        virtual_start: 0x100000000,
        size: 0x100000000,
    },
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_IO,
        physical_start: 0x1000,
        virtual_start: 0x1000,
        size: 0x80000000 - 0x1000,
    },
];

pub const ROOT_ZONE_IRQS: [u32; 19] = [
    36, 52, 55, 59, 64, 67, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 150, 151, 152,
];

pub const ROOT_ARCH_ZONE_CONFIG: HvArchZoneConfig = HvArchZoneConfig {
    gicd_base: 0x17a00000,
    gicd_size: 0x10000,
    gicr_base: 0x17a60000,
    gicr_size: 0x100000,
};
