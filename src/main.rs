//! The main module and entrypoint
//!
//! Various facilities of hvisor are implemented as submodules. The most
//! important ones are:
//!
//! - [`memory`]: Memory management
//! - [`hypercall`]: Hypercall handling
//! - [`device`]: Device management
//! - [`arch`]: Architecture's related

#![no_std] // 禁用标准库链接
#![no_main]
// 不使用main入口，使用自己定义实际入口_start，因为我们还没有初始化堆栈指针
#![feature(asm_const)]
#![feature(naked_functions)] //  surpport naked function
#![feature(core_panic)]
// 支持内联汇编
// #![deny(warnings, missing_docs)] // 将warnings作为error
#[macro_use]
extern crate alloc;
extern crate buddy_system_allocator;
#[macro_use]
mod error;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
mod logging;
mod arch;
mod consts;
mod device;
mod event;
mod hypercall;
mod memory;
mod panic;
mod percpu;
mod platform;
mod zone;
mod config;

use aarch64_paging::{
    idmap::IdMap,
    paging::{Attributes, MemoryRegion, TranslationRegime},
};

use crate::arch::mm::setup_parange;
use crate::consts::MAX_CPU_NUM;
use arch::{cpu::cpu_start, entry::arch_entry};
use config::root_zone_config;
use zone::zone_create;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use percpu::PerCpu;

static INITED_CPUS: AtomicU32 = AtomicU32::new(0);
static ENTERED_CPUS: AtomicU32 = AtomicU32::new(0);
static INIT_EARLY_OK: AtomicU32 = AtomicU32::new(0);
static INIT_LATE_OK: AtomicU32 = AtomicU32::new(0);
static MASTER_CPU: AtomicI32 = AtomicI32::new(-1);

pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let mut p = sbss as *mut u8;
    while p < ebss as _ {
        unsafe {
            *p = 0;
            p = p.add(1);
        };
    }
}

fn wait_for(condition: impl Fn() -> bool) {
    while condition() {
        core::hint::spin_loop();
    }
}

fn wait_for_counter(counter: &AtomicU32, max_value: u32) {
    wait_for(|| counter.load(Ordering::Acquire) < max_value)
}

fn primary_init_early() {
    extern "C" {
        fn __core_end();
    }
    // logging::init();
    info!("Logging is enabled.");
    info!("__core_end = {:#x?}", __core_end as usize);
    // let system_config = HvSystemConfig::get();
    // let revision = system_config.revision;
    info!("Hypervisor initialization in progress...");
    info!(
        "build_mode: {}, log_level: {}, arch: {}, vendor: {}, stats: {}",
        option_env!("MODE").unwrap_or(""),
        option_env!("LOG").unwrap_or(""),
        option_env!("ARCH").unwrap_or(""),
        option_env!("VENDOR").unwrap_or(""),
        option_env!("STATS").unwrap_or("off"),
    );

    memory::frame::init();
    memory::frame::test();
    event::init(MAX_CPU_NUM);

    device::irqchip::primary_init_early();
    // crate::arch::mm::init_hv_page_table().unwrap();

    zone_create(root_zone_config()).unwrap();
    INIT_EARLY_OK.store(1, Ordering::Release);
}

fn primary_init_late() {
    info!("Primary CPU init late...");
    device::irqchip::primary_init_late();

    INIT_LATE_OK.store(1, Ordering::Release);
}

fn per_cpu_init(cpu: &mut PerCpu) {
    if cpu.zone.is_none() {
        warn!("zone is not created for cpu {}", cpu.id);
    }
    // unsafe {
    //     memory::hv_page_table().read().activate();
    // };
    info!("CPU {} hv_pt_install OK.", cpu.id);
}

fn wakeup_secondary_cpus(this_id: usize, host_dtb: usize) {
    for cpu_id in 0..MAX_CPU_NUM {
        if cpu_id == this_id {
            continue;
        }
        info!("Waking up CPU {}...", cpu_id);
        cpu_start(cpu_id, arch_entry as _, host_dtb);
    }
}

pub unsafe fn enable_mmu() {
    // const MAIR_FLAG: usize = 0x004404ff; //10001000000010011111111
       const MAIR_FLAG: usize = 0xff440c0400;
    const SCTLR_FLAG: usize = 0x30c51835; //110000110001010001100000110101
    const TCR_FLAG: usize = 0x8081351c; //10000000100001010011010100010000

    core::arch::asm!(
        "
        /* setup the MMU for EL2 hypervisor mappings */
        ldr	x1, ={MAIR_FLAG}     
        msr	mair_el2, x1       // memory attributes for pagetable
        ldr	x1, ={TCR_FLAG}
	    msr	tcr_el2, x1        // translate control, virt range = [0, 2^36)

	    /* Enable MMU, allow cacheability for instructions and data */
	    ldr	x1, ={SCTLR_FLAG}
	    msr	sctlr_el2, x1      // system control register

	    // isb
	    // tlbi alle2
	    // dsb	nsh
    ",
        MAIR_FLAG = const MAIR_FLAG,
        TCR_FLAG = const TCR_FLAG,
        SCTLR_FLAG = const SCTLR_FLAG,
    );
}


const NORMAL_CACHEABLE: Attributes = Attributes::ATTRIBUTE_INDEX_4.union(Attributes::INNER_SHAREABLE);

fn pagetable_init(idmap: &mut IdMap) {
    match idmap.map_range(
        &MemoryRegion::new(0x1000, 0x80000000),
        Attributes::PXN | Attributes::UXN | Attributes::VALID | Attributes::ACCESSED,
    ) {
        Ok(()) => {},
        Err(e) => {
            println!("map_range failed! 0x1000");
        },
    };
    match idmap.map_range(
        &MemoryRegion::new(0x80000000, 0x80000000 + 0x3a800000),
        NORMAL_CACHEABLE | Attributes::VALID | Attributes::ACCESSED,
    ) {
        Ok(()) => {},
        Err(e) => {
            println!("map_range failed! 0x80000000");
        },
    };
    match idmap.map_range(
        &MemoryRegion::new(0xc0000000, 0xc0000000 + 0x01800000),
        NORMAL_CACHEABLE | Attributes::VALID | Attributes::ACCESSED,
    ) {
        Ok(()) => {},
        Err(e) => {
            println!("map_range failed! 0xc0000000");
        },
    };
    match idmap.map_range(
        &MemoryRegion::new(0xc3400000, 0xc3400000 + 0x3cc00000),
        NORMAL_CACHEABLE | Attributes::VALID | Attributes::ACCESSED,
    ) {
        Ok(()) => {},
        Err(e) => {
            println!("map_range failed! 0xc3400000");
        },
    };
    match idmap.map_range(
        &MemoryRegion::new(0x100000000, 0x100000000 + 0x100000000),
        NORMAL_CACHEABLE | Attributes::VALID | Attributes::ACCESSED,
    ) {
        Ok(()) => {},
        Err(e) => {
            println!("map_range failed! 0x100000000");
        },
    };

    println!("Activating pagetable!");
    unsafe {
        idmap.activate();
        enable_mmu();
    }
}

fn rust_main(cpuid: usize, mut host_dtb: usize) {
    arch::trap::install_trap_vector();

    let mut is_primary = false;
    println!("Hello, HVISOR! got dtb {:#x}", host_dtb);
    println!("Some more log messages...");

    let mut idmap;

    logging::init();

    info!("Logging is alive!\n");

    // BAD hacks lmao
    host_dtb = 0x9ec00000;

    // println!("addr 0x14820980: {:#x}", unsafe { *(0x14820980 as *const u64) });

    if MASTER_CPU.load(Ordering::Acquire) == -1 {
        MASTER_CPU.store(cpuid as i32, Ordering::Release);
        is_primary = true;
        memory::heap::init();
        memory::heap::test();
        idmap = IdMap::new(0, 1, TranslationRegime::El2);
        pagetable_init(&mut idmap);
    }

    let cpu = PerCpu::new(cpuid);

    info!(
        "Booting CPU {}: {:p}, DTB: {:#x}",
        cpu.id, cpu as *const _, host_dtb
    );

    if is_primary {
        wakeup_secondary_cpus(cpu.id, host_dtb);
    }

    ENTERED_CPUS.fetch_add(1, Ordering::SeqCst);
    wait_for(|| PerCpu::entered_cpus() < MAX_CPU_NUM as _);
    assert_eq!(PerCpu::entered_cpus(), MAX_CPU_NUM as _);

    println!(
        "{} CPU {} has entered.",
        if is_primary { "Primary" } else { "Secondary" },
        cpu.id
    );

    #[cfg(target_arch = "aarch64")]
    setup_parange();

    if is_primary {
        primary_init_early(); // create root zone here
    } else {
        wait_for_counter(&INIT_EARLY_OK, 1);
    }

    per_cpu_init(cpu);
    device::irqchip::percpu_init();

    INITED_CPUS.fetch_add(1, Ordering::SeqCst);
    wait_for_counter(&INITED_CPUS, MAX_CPU_NUM as _);

    if is_primary {
        primary_init_late();
    } else {
        wait_for_counter(&INIT_LATE_OK, 1);
    }

    cpu.run_vm();
}
