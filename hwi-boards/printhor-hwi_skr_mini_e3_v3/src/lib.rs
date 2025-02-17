#![no_std]
#![feature(trait_alias)]
#![feature(type_alias_impl_trait)]
#![allow(stable_features)]
use embassy_stm32::interrupt;
use embassy_stm32::interrupt::Priority;
use embassy_stm32::interrupt::InterruptExt;
use embassy_executor::InterruptExecutor;
use embassy_executor::SendSpawner;

pub use defmt::{trace,debug,info,warn, error};
pub use defmt;

mod board;

pub use board::device;
pub use board::consts;
pub use board::IODevices;
pub use board::Controllers;
pub use board::MotionDevices;
pub use board::PwmDevices;

pub use board::init;
pub use board::setup;
pub use board::heap_current_size;
pub use board::heap_current_usage_percentage;
pub use board::stack_reservation_current_size;
pub use board::MACHINE_BOARD;
pub use board::MACHINE_TYPE;
pub use board::MACHINE_PROCESSOR;
pub use board::HEAP_SIZE_BYTES;
pub use board::MAX_STATIC_MEMORY;
pub use board::VREF_SAMPLE;
#[cfg(feature = "with-sdcard")]
pub use board::SDCARD_PARTITION;
#[cfg(feature = "with-usbserial")]
const USBSERIAL_BUFFER_SIZE: usize = 32;
#[cfg(feature = "with-uart-port-1")]
const UART_PORT1_BUFFER_SIZE: usize = 32;
#[cfg(feature = "with-uart-port-1")]
const UART_PORT1_BAUD_RATE: u32 = 115200;

pub static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn I2C2_3() {
    EXECUTOR_HIGH.on_interrupt()
}

#[inline]
pub fn get_stepper_spawner() -> SendSpawner {
    interrupt::I2C2_3.set_priority(Priority::P5);
    interrupt::USB_UCPD1_2.set_priority(Priority::P6);
    EXECUTOR_HIGH.start(interrupt::I2C2_3)
}

#[inline]
pub fn launch_high_priotity<S: 'static + Send>(token: embassy_executor::SpawnToken<S>) -> Result<(),()> {
    interrupt::I2C2_3.set_priority(Priority::P5);
    interrupt::USB_UCPD1_2.set_priority(Priority::P6);
    let spawner = EXECUTOR_HIGH.start(interrupt::I2C2_3);
    spawner.spawn(token).map_err(|_| ())
}

#[inline]
pub fn init_logger() {
}

#[inline]
pub fn sys_reset() {
    cortex_m::peripheral::SCB::sys_reset();
}
