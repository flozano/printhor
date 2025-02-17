#[cfg(feature = "with-usbserial")]
pub mod usbserial {
    use crate::board::device::USBDrv;
    use futures::Stream;
    use core::pin::Pin;
    use futures::task::Context;
    use futures::task::Poll;
    use futures::Future;

    pub type USBSerialDeviceSender = embassy_usb::class::cdc_acm::Sender<'static, USBDrv>;
    pub type USBSerialDeviceReceiver = embassy_usb::class::cdc_acm::Receiver<'static, USBDrv>;
    pub type USBSerialTxControllerRef = crate::board::ControllerRef<USBSerialDeviceSender>;

    pub struct USBSerialDevice {
        pub builder: Option<embassy_usb::Builder<'static, USBDrv>>,
        pub sender: USBSerialDeviceSender,
        pub receiver: USBSerialDeviceReceiver,
    }

    #[embassy_executor::task(pool_size=1)]
    pub async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, USBDrv>) -> ! {
        defmt::info!("Running usb task...");
        usb.run().await;
        unreachable!("usb task ended")
    }

    impl USBSerialDevice {
        pub fn new(driver: USBDrv) -> Self {

            let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
            config.manufacturer = Some("Printor");
            config.product = Some("Printor-USBSerial");
            config.serial_number = Some("");

            config.device_class = 0xEF;
            config.device_sub_class = 0x02;
            config.device_protocol = 0x01;
            config.composite_with_iads = true;
            static DEVICE_DESCRIPTOR_ST: crate::board::TrackedStaticCell<[u8; 256]> = crate::board::TrackedStaticCell::new();
            let device_descriptor = DEVICE_DESCRIPTOR_ST.init("", [0; 256]);
            static CONFIG_DESCRIPTOR_ST: crate::board::TrackedStaticCell<[u8; 256]> = crate::board::TrackedStaticCell::new();
            let config_descriptor = CONFIG_DESCRIPTOR_ST.init("", [0; 256]);
            static BOS_DESCRIPTOR_ST: crate::board::TrackedStaticCell<[u8; 256]> = crate::board::TrackedStaticCell::new();
            let bos_descriptor = BOS_DESCRIPTOR_ST.init("", [0; 256]);
            static CONTROL_BUF_ST: crate::board::TrackedStaticCell<[u8; 64]> = crate::board::TrackedStaticCell::new();
            let control_buf = CONTROL_BUF_ST.init("", [0; 64]);

            static STATE_ST: crate::board::TrackedStaticCell<embassy_usb::class::cdc_acm::State> = crate::board::TrackedStaticCell::new();
            let state = STATE_ST.init("", embassy_usb::class::cdc_acm::State::new());
            let mut builder = embassy_usb::Builder::new(
                driver,
                config,
                device_descriptor,
                config_descriptor,
                bos_descriptor,
                control_buf,
            );

            //crate::info!("Creating USB CLASS");

            // Create classes on the builder.
            let class = embassy_usb::class::cdc_acm::CdcAcmClass::new(&mut builder, state, 64);

            let (sender, receiver) = class.split();

            Self {
                builder: Some(builder),
                sender,
                receiver,
            }
        }

        pub fn spawn(&mut self, spawner: crate::board::Spawner) {
            self.builder.take().map(|builder| {
                match spawner.spawn(usb_task(builder.build())) {
                    Ok(_) => {
                        ()
                    }
                    Err(_) => {
                        panic!("Unable to spawn USB task")
                    }
                }
            });
        }

        pub fn split(self) -> (USBSerialDeviceReceiver, USBSerialDeviceSender) {
            (
                self.receiver,
                self.sender,
            )
        }
    }

    pub struct USBSerialDeviceInputStream {
        receiver: embassy_usb::class::cdc_acm::Receiver<'static, crate::board::device::USBDrv>,
        buffer: [u8; crate::USBSERIAL_BUFFER_SIZE],
        bytes_read: u8,
        current_byte_index: u8,
    }

    impl USBSerialDeviceInputStream {
        pub fn new(receiver: USBSerialDeviceReceiver) -> Self {
            Self {
                receiver,
                buffer: [0; crate::USBSERIAL_BUFFER_SIZE],
                bytes_read: 0,
                current_byte_index: 0,
            }
        }
    }

    impl Stream for USBSerialDeviceInputStream
    {
        type Item = Result<u8, async_gcode::Error>;

        fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<<Self as futures::Stream>::Item>> {

            let this = self.get_mut();

            if this.current_byte_index < this.bytes_read {
                let byte = this.buffer[this.current_byte_index as usize];
                this.current_byte_index += 1;
                Poll::Ready(Some(Ok(byte)))
            }
            else {
                this.current_byte_index = 0;
                this.bytes_read = 0;

                if core::pin::pin!(this.receiver.wait_connection()).poll(ctx).is_pending() {
                    return Poll::Pending
                }

                let r = core::pin::pin!(
                    this.receiver.read_packet(&mut this.buffer)
                ).poll(ctx);
                match r {
                    Poll::Ready(rst) => {
                        match rst {
                            Ok(n) => {
                                this.bytes_read = n as u8;
                                if n > 0 {
                                    let byte = this.buffer[this.current_byte_index as usize];
                                    this.current_byte_index += 1;
                                    Poll::Ready(Some(Ok(byte)))
                                }
                                else {
                                    Poll::Ready(None)
                                }
                            }
                            Err(_e) => {
                                Poll::Ready(None)
                            }
                        }
                    }
                    Poll::Pending => Poll::Pending
                }
            }
        }
    }
}

#[cfg(feature = "with-uart-port-1")]
pub mod uart_port1 {
    use crate::device::UartPort1RxDevice;
    use futures::Stream;
    use core::pin::Pin;
    use futures::task::Context;
    use futures::task::Poll;
    use futures::Future;

    pub struct UartPort1RxInputStream {
        pub receiver: UartPort1RxDevice,
        buffer: [u8; crate::UART_PORT1_BUFFER_SIZE],
        bytes_read: u8,
        current_byte_index: u8,
    }

    impl UartPort1RxInputStream {
        pub fn new(receiver: UartPort1RxDevice) -> Self {
            Self {
                receiver,
                buffer: [0; crate::UART_PORT1_BUFFER_SIZE],
                bytes_read: 0,
                current_byte_index: 0,
            }
        }
    }

    impl Stream for UartPort1RxInputStream
    {
        type Item = Result<u8, async_gcode::Error>;

        fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<<Self as futures::Stream>::Item>> {

            let this = self.get_mut();

            if this.current_byte_index < this.bytes_read {
                let byte = this.buffer[this.current_byte_index as usize];
                this.current_byte_index += 1;
                Poll::Ready(Some(Ok(byte)))
            }
            else {
                this.current_byte_index = 0;
                this.bytes_read = 0;

                let r = core::pin::pin!(
                    this.receiver.read_until_idle(&mut this.buffer)
                ).poll(ctx);
                match r {
                    Poll::Ready(rst) => {
                        match rst {
                            Ok(n) => {
                                this.bytes_read = n as u8;
                                if n > 0 {
                                    let byte = this.buffer[this.current_byte_index as usize];
                                    this.current_byte_index += 1;
                                    Poll::Ready(Some(Ok(byte)))
                                }
                                else {
                                    Poll::Ready(None)
                                }
                            }
                            Err(_e) => {
                                Poll::Ready(None)
                            }
                        }
                    }
                    Poll::Pending => Poll::Pending
                }
            }
        }
    }



}