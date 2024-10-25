#![allow(dead_code)]

use core::ptr;

use crate::memory::addr::{PhysAddr, VirtAddr};

pub const UART_BASE_PHYS: PhysAddr = 0x994000;

const SE_GENI_STATUS: usize = 0x40;
const SE_GENI_CMD_ACTIVE: u32 = 0x1;
// #define SE_UART_TX_TRANS_LEN	0x270
const SE_UART_TX_TRANS_LEN: usize = 0x270;
// #define SE_GENI_M_CMD0	0x600
const SE_GENI_M_CMD0: usize = 0x600;
const UART_START_TX: u32 = 0x1;
const M_OPCODE_SHIFT: u32 = 27;
const SE_GENI_TX_FIFOn: usize =	0x700;

// lazy_static! {
static mut UART: QcomUART = {
    QcomUART::new(UART_BASE_PHYS)
    // Mutex::new(uart)
};
// }

struct QcomUART {
    base: PhysAddr,
}

/*

qcom_geni_serial_poll_bit(base, SE_GENI_STATUS,
				  M_GENI_CMD_ACTIVE, false);

	qcom_geni_serial_setup_tx(base, 1);
	writel(ch, base + SE_GENI_TX_FIFOn);
*/

impl QcomUART {
    const fn new(base: PhysAddr) -> Self {
        Self { base }
    }

    fn is_busy(&self) -> bool {
        let sts_addr = (self.base + SE_GENI_STATUS) as *mut u32;
        unsafe {
            return (ptr::read_volatile(sts_addr) & SE_GENI_CMD_ACTIVE) == 1;
        }
    }

    fn setup_tx(&self) {
        unsafe {
            ptr::write_volatile((self.base + SE_UART_TX_TRANS_LEN) as *mut u32, 1);
            ptr::write_volatile((self.base + SE_GENI_M_CMD0) as *mut u32, UART_START_TX << M_OPCODE_SHIFT);
        }
    }

    fn putchar(&self, c: u8) {
        unsafe {
            while self.is_busy() {}
            self.setup_tx();
            ptr::write_volatile((self.base + SE_GENI_TX_FIFOn) as *mut u32, c as u32);
        }
    }
    fn getchar(&mut self) -> Option<u8> {
        todo!()
    }
}

pub fn console_putchar(c: u8) {
    unsafe { UART.putchar(c) }
}

pub fn console_getchar() -> Option<u8> {
    unsafe { UART.getchar() }
}
