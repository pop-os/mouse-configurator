//! Use a separate process, started with `pkexec`, to monitor for devices
//! with udev and open them. Using `pkexec` allows it to access the device
//! without a persistent daemon or udev rule.

use nix::{
    cmsg_space,
    errno::Errno,
    sys::socket::{
        recvmsg, sendmsg, socketpair, AddressFamily, ControlMessage, ControlMessageOwned, MsgFlags,
        SockFlag, SockType, UnixAddr,
    },
};
use std::{
    ffi::OsStr,
    io::{self, IoSlice, IoSliceMut},
    os::unix::{
        ffi::OsStrExt,
        io::{AsRawFd, FromRawFd, RawFd},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use hp_mouse_configurator::HpMouse;

pub struct DeviceMonitorProcess {
    sock: RawFd,
    buf: [u8; 1024],
}

impl Drop for DeviceMonitorProcess {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.sock);
    }
}

impl DeviceMonitorProcess {
    pub fn new() -> io::Result<Self> {
        let (sock1, sock2) = socketpair(
            AddressFamily::Unix,
            SockType::Datagram,
            None,
            SockFlag::SOCK_CLOEXEC,
        )?;
        // XXX appimage? Own executable?
        let stdin = unsafe { Stdio::from_raw_fd(sock1) };
        let mut child = Command::new("pkexec")
            .args(&["hp-mouse-configurator", "--device-monitor"])
            .stdin(stdin)
            .spawn()?;

        let mut buf = [0; 1024];

        loop {
            if let Err(err) = nix::unistd::read(sock2, &mut buf) {
                match err {
                    Errno::EINTR => {
                        continue;
                    }
                    Errno::EPIPE => {
                        let status = child.wait()?;
                        let message = format!("Pkexec process failed: {}", status);
                        return Err(io::Error::new(io::ErrorKind::Other, message));
                    }
                    _ => {}
                }
            }
            break;
        }

        Ok(Self { sock: sock2, buf })
    }
}

impl Iterator for DeviceMonitorProcess {
    type Item = io::Result<(PathBuf, HpMouse)>;

    fn next(&mut self) -> Option<io::Result<(PathBuf, HpMouse)>> {
        loop {
            let mut iov = [IoSliceMut::new(&mut self.buf)];
            let mut cmsg = cmsg_space!(RawFd);
            match recvmsg::<UnixAddr>(self.sock, &mut iov, Some(&mut cmsg), MsgFlags::empty()) {
                Ok(msg) => {
                    for cmsg in msg.cmsgs() {
                        if let ControlMessageOwned::ScmRights(fds) = cmsg {
                            assert_eq!(fds.len(), 1);
                            let fd = fds[0];
                            let path =
                                Path::new(OsStr::from_bytes(&self.buf[..msg.bytes])).to_owned();
                            let mouse = unsafe { HpMouse::from_raw_fd(fd) };
                            return Some(Ok((path, mouse)));
                        } else {
                            panic!("Unexpected control message: {:?}", cmsg);
                        }
                    }
                }
                Err(Errno::EINTR) => {}
                Err(err) => {
                    return Some(Err(err.into()));
                }
            }
        }
    }
}

pub fn device_monitor_process() {
    let current_devices = hp_mouse_configurator::enumerate().unwrap();
    let monitor_devices = hp_mouse_configurator::monitor().unwrap();

    while nix::unistd::write(libc::STDIN_FILENO, b"Started\n") == Err(Errno::EINTR) {}

    for device_info in current_devices.into_iter().chain(monitor_devices) {
        match device_info.open() {
            Ok(device) => {
                let path = device_info.devnode.as_os_str().as_bytes();
                let fds = &[device.as_raw_fd()];
                let iov = &[IoSlice::new(path)];
                let cmsgs = &[ControlMessage::ScmRights(fds)];
                loop {
                    let res = sendmsg(
                        libc::STDIN_FILENO,
                        iov,
                        cmsgs,
                        MsgFlags::empty(),
                        None::<&UnixAddr>,
                    );
                    match res {
                        Ok(_) => {
                            break;
                        }
                        Err(Errno::EINTR) => {}
                        Err(Errno::EPIPE) => {
                            return;
                        }
                        Err(err) => {
                            eprintln!("Error writing to socket: {}", err);
                            break;
                        }
                    }
                }
            }
            Err(err) => eprintln!("Failed to open device: {}", err),
        }
    }
}
