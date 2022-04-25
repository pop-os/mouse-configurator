use std::{
    fs::File,
    io,
    os::unix::io::{IntoRawFd, RawFd},
    path::Path,
};

// TODO: Use `OwnedFd` when stable
pub struct Hid(RawFd);

impl Drop for Hid {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.0);
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
