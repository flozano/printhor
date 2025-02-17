use printhor_hwa_common::{DisplayScreenUI, EventBusRef, EventBusSubscriber, EventFlags, TrackedStaticCell};
use embedded_graphics::prelude::Point;
use embedded_graphics_core::pixelcolor::{RgbColor};

use embedded_graphics_core::Drawable;
use embedded_graphics::mono_font::{ MonoTextStyle};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::text::Text;
use embedded_graphics_core::prelude::DrawTarget;
#[cfg(feature = "ili9341_spi")]
use crate::display::ili9341_spi::RawDisplay;


pub struct EmbeddedGraphicsUI {
    #[allow(unused)]
    event_bus: &'static EventBusRef,
    #[allow(unused)]
    subscriber: &'static mut EventBusSubscriber<'static>,
}

impl EmbeddedGraphicsUI {
    pub async fn new(event_bus: EventBusRef) -> Self {
        static EVENT_BUS: TrackedStaticCell<EventBusRef> = TrackedStaticCell::new();
        let bus = EVENT_BUS.init("UIEventBusRef", event_bus);
        static UI_SUBSCRIBER: TrackedStaticCell<EventBusSubscriber<'static>> = TrackedStaticCell::new();
        let subscriber = UI_SUBSCRIBER.init("UIEventSubscriber", bus.subscriber().await);
        Self {
            event_bus: bus,
            subscriber,
        }
    }
}

impl DisplayScreenUI for EmbeddedGraphicsUI {
    async fn refresh<D>(&mut self, raw_display: &mut D)
        where
            D: DrawTarget,
            <D as DrawTarget>::Color: RgbColor,
    {
        let style = MonoTextStyle::new(&FONT_6X10, RgbColor::WHITE);

        let current_status = self.subscriber.get_status().await;
        let state = if current_status.contains(EventFlags::SYS_BOOTING) {
            "BOOTING"
        }
        else if current_status.contains(EventFlags::HOMMING) {
            "HOMMING"
        }
        else if current_status.contains(EventFlags::SYS_READY) {
            "READY"
        }
        else {
            "N/A"
        };
        let _t0 = embassy_time::Instant::now();
        //raw_display.retain().await;
        let _ = Text::new(state, Point::new(20, 30), style).draw(raw_display);
        //raw_display.release().await;
        //crate::info!("text upd in {} ms", _t0.elapsed().as_millis());
    }
}