#[allow(unused)]
use crate::hwa;
use crate::hwi;
pub use printhor_hwa_common::ControllerRef;
pub use crate::control::processor::GCodeProcessor;

#[cfg(feature = "with-defmt")]
pub use defmt;

#[cfg(feature = "with-defmt")]
pub use defmt::{trace, debug, info, warn, error};

pub mod devices;
pub mod controllers;
pub mod drivers;

//#region Main exports

pub use hwi::*;

//#endregion

pub mod mem {
    use crate::hwi;
    #[allow(unused)]
    #[inline]
    pub fn heap_current_size() -> u32 {
        hwi::heap_current_size()
    }
    #[allow(unused)]
    #[inline]
    pub fn heap_max_size() -> usize {
        hwi::HEAP_SIZE_BYTES
    }

    #[allow(unused)]
    #[inline]
    pub fn heap_current_usage_percentage() -> f32 {
        hwi::heap_current_usage_percentage()
    }

    #[allow(unused)]
    #[inline]
    pub fn stack_reservation_max_size() -> u32 {
        hwi::MAX_STATIC_MEMORY as u32

    }

    #[allow(unused)]
    #[inline]
    pub fn stack_reservation_current_size() -> u32 {
        hwi::stack_reservation_current_size()
    }

    #[allow(unused)]
    #[inline]
    pub fn stack_reservation_usage_percentage() -> f32 {
        let alloc = stack_reservation_current_size() as f32;
        let max = stack_reservation_max_size() as f32;
        (100.0f32 * alloc) / max
    }
}

#[cfg(feature = "with-display")]
pub mod display {
    pub use crate::hwi::device::DisplayDevice;
    pub use crate::hwi::device::DisplayScreen;
}

/// Simple module to manage and measure static controller allocations in async tasks
pub mod task_allocations {
    use printhor_hwa_common::{EventBusRef, EventBusSubscriber, TrackedStaticCell};

    pub async fn init_control_subscriber(event_bus: EventBusRef) -> EventBusSubscriber<'static>  {
        static SUBS: TrackedStaticCell<EventBusRef> = TrackedStaticCell::new();
        let bi: &mut EventBusRef = SUBS.init("control_task::EventBusSubscriber", event_bus);
        bi.subscriber().await
    }

    #[cfg(feature = "with-printjob")]
    pub async fn init_printer_subscriber(event_bus: EventBusRef) -> EventBusSubscriber<'static>  {
        static SUBS: TrackedStaticCell<EventBusRef> = TrackedStaticCell::new();
        let bi: &mut EventBusRef = SUBS.init("printer_task::EventBusSubscriber", event_bus);
        bi.subscriber().await
    }

    #[cfg(all(feature = "integration-test"))]
    pub async fn init_integration_subscriber(event_bus: EventBusRef) -> EventBusSubscriber<'static>  {
        static SUBS: TrackedStaticCell<EventBusRef> = TrackedStaticCell::new();
        let bi: &mut EventBusRef = SUBS.init("integration_task::EventBusSubscriber", event_bus);
        bi.subscriber().await
    }

    /*
    #[cfg(feature = "with-sdcard")]
    pub async fn init_defer_subscriber(event_bus: hwa::EventBusRef) -> EventBusSubscriber<'static>  {
        static SUBS: TrackedStaticCell<hwa::EventBusRef> = TrackedStaticCell::new();
        let bi: &mut hwa::EventBusRef = SUBS.init("deferred_task::EventBusSubscriber", event_bus);
        bi.subscriber().await
    }
     */
}

#[cfg(feature = "with-sdcard")]
pub struct DummyTimeSource {
    
}

#[cfg(feature = "with-sdcard")]
impl embedded_sdmmc::TimeSource for DummyTimeSource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub type WatchdogRef = ControllerRef<hwi::device::Watchdog>;


