use super::{SocketReader, SocketWriter};
use alvr_common::{anyhow::Result, ConResult, ConnectionError, HandleTryAgain};
use alvr_session::SocketBufferSize;
use rusb::{DeviceDescriptor, DeviceHandle, GlobalContext};
use socket2::MaybeUninitSlice;
use std::{
    io::{self},
    mem,
    time::Duration,
};
const USB_TIMEOUT: Duration = Duration::from_millis(100);

pub struct UsbSocket(DeviceHandle<GlobalContext>, u8);

impl UsbSocket {
    pub fn open_descriptor(
        descriptor: DeviceDescriptor,
    ) -> io::Result<DeviceHandle<GlobalContext>> {
        // todo: replace with proper open, this is a convenience function
        let mut usb =
            rusb::open_device_with_vid_pid(descriptor.vendor_id(), descriptor.product_id())
                .expect("bruh");
        usb.set_active_configuration(1);
        usb.claim_interface(0);
        Ok(usb)
    }

    pub fn set_endpoint(&self, endpoint: u8) {
        self.1 = endpoint;
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self.0.read_bulk(self.1, buf, USB_TIMEOUT) {
            Ok(size) => Ok(size),
            Err(e) => {
                if e == rusb::Error::Timeout || e == rusb::Error::Overflow {
                    ConnectionError::TryAgain(e.into())
                } else {
                    ConnectionError::Other(e.into())
                }
            }
        }
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, rusb::Error> {
        self.0.write_bulk(self.1, buf, USB_TIMEOUT)
    }

    pub fn recv_from(&self, endpoint: u8, buf: &mut [u8]) -> Result<usize, rusb::Error> {
        self.0.read_bulk(endpoint, buf, USB_TIMEOUT)
    }

    pub fn send_to(&self, endpoint: u8, buf: &[u8]) -> Result<usize, rusb::Error> {
        self.0.write_bulk(endpoint, buf, USB_TIMEOUT)
    }

    pub fn peek(&self, buf: &mut [u8]) -> Result<usize, rusb::Error> {
        self.recv(buf)?;
        let mut copy_buf = buf.as_ref().clone();
        Ok(copy_buf.len())
    }
}

pub fn open_usb(
    descriptor: DeviceDescriptor,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<UsbSocket> {
    let usb = UsbSocket::open_descriptor(descriptor)?;
    println!(
        "Opening USB ID {:04x}:{:04x}",
        descriptor.vendor_id(),
        descriptor.product_id()
    );

    Ok(UsbSocket(usb, 0x00))
}

impl SocketWriter for UsbSocket {
    fn send(&mut self, buffer: &[u8]) -> Result<()> {
        UsbSocket::send(self, buffer)?;

        Ok(())
    }
}

impl SocketReader for UsbSocket {
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize> {
        UsbSocket::recv(&self, buffer).handle_try_again()
    }

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize> {
        Ok(self.peek(buf)?)
    }
}
