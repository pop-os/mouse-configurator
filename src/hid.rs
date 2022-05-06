use std::{
    fs::File,
    io,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    path::Path,
};

// TODO: Use `OwnedFd` when stable
pub struct Hid(RawFd);

impl Drop for Hid {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.0);
    }
}

impl AsRawFd for Hid {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl FromRawFd for Hid {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(fd)
    }
}

impl Hid {
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self(
            File::options()
                .read(true)
                .write(true)
                .open(path)?
                .into_raw_fd(),
        ))
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let length = nix::unistd::read(self.0, buf)?;
        Ok(length)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let length = nix::unistd::write(self.0, buf)?;
        Ok(length)
    }
}
