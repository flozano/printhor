use std::io;
use std::os::unix::io::{AsRawFd, RawFd};

use nix::errno::Errno;
use nix::fcntl::OFlag;
use nix::sys::termios;

pub(crate)  struct NativeSerialPort {
    fd: RawFd,
}

impl NativeSerialPort {
    pub(crate) fn new<P: ?Sized + nix::NixPath>(path: &P, baudrate: Option<termios::BaudRate>) -> io::Result<Self> {

        let fd = nix::fcntl::open(
            path,
            OFlag::O_RDONLY | OFlag::O_NOCTTY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        ).map_err(to_io_error).expect("Open Fail");

        if let Some(baud) = baudrate {
            let mut cfg = termios::tcgetattr(fd).map_err(to_io_error)?;
            cfg.input_flags = termios::InputFlags::empty();
            cfg.output_flags = termios::OutputFlags::empty();
            cfg.control_flags = termios::ControlFlags::empty();
            cfg.local_flags = termios::LocalFlags::empty();
            termios::cfmakeraw(&mut cfg);
            cfg.input_flags |= termios::InputFlags::IGNBRK;
            cfg.control_flags |= termios::ControlFlags::CREAD;
            //cfg.control_flags |= termios::ControlFlags::CRTSCTS;
            termios::cfsetospeed(&mut cfg, baud).map_err(to_io_error)?;
            termios::cfsetispeed(&mut cfg, baud).map_err(to_io_error)?;
            termios::cfsetspeed(&mut cfg, baud).map_err(to_io_error)?;
            // Set VMIN = 1 to block until at least one character is received.
            cfg.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 1;
            termios::tcsetattr(fd, termios::SetArg::TCSANOW, &cfg).map_err(to_io_error)?;
            termios::tcflush(fd, termios::FlushArg::TCIOFLUSH).map_err(to_io_error)?;
        }

        Ok(Self { fd })
    }
}

impl AsRawFd for NativeSerialPort {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl io::Read for NativeSerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        nix::unistd::read(self.fd, buf).map_err(to_io_error)
    }
}

impl io::Write for NativeSerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        nix::unistd::write(self.fd, buf).map_err(to_io_error)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn to_io_error(e: Errno) -> io::Error {
    e.into()
}