use std::{
    io,
    os::unix::io::{AsRawFd, FromRawFd, RawFd},
    path::Path,
    sync::Arc,
};

pub mod button;
pub use button::{Button, Op, PressType, Value};
mod enumerate;
pub use enumerate::{enumerate, monitor, DeviceInfo};
mod event;
pub use event::{Event, HpMouseEvents, ReadRes};
mod hid;
use hid::Hid;

const HP_SIGNATURE: u16 = 0xCF3;

#[derive(Debug)]
pub struct HpMouse {
    dev: Arc<Hid>,
}

impl HpMouse {
    pub fn open_devnode(path: &Path) -> io::Result<Self> {
        Ok(Self {
            dev: Arc::new(Hid::open(path)?),
        })
    }

    //TODO: support multi report packets
    pub fn write_report_1(&self, kind: u16, packet: &[u8]) -> io::Result<()> {
        let report = 1;
        let signature = HP_SIGNATURE + kind;
        assert_eq!(signature & 0xF000, 0);

        let mut data = [0; 21];
        data[0] = report;
        data[1] = signature as u8;
        data[2] = (signature >> 8) as u8;
        data[3] = packet.len() as u8;
        // data[4] is sequence, length high
        for i in 0..packet.len() {
            data[5 + i] = packet[i];
        }

        let len = self.dev.write(&data)?;
        eprintln!("HID write {}", len);

        for i in 0..len {
            eprint!(" {:02x}", data[i]);
        }
        eprintln!();

        Ok(())
    }

    /// Send query for firmware info
    pub fn query_firmware(&self) -> io::Result<()> {
        self.write_report_1(0, &[])
    }

    /// Send query for battery info
    pub fn query_battery(&self) -> io::Result<()> {
        let low_level = 0xFF; // do not set
        let crit_level = 0xFF; // do not set
        let power_off_timeout = 0xFF; // do not set
        let auto_report_delay = 0x06; // 60 seconds
        self.write_report_1(
            5,
            &[low_level, crit_level, power_off_timeout, auto_report_delay],
        )
    }

    /// Send query for button info
    pub fn query_button(&self) -> io::Result<()> {
        let command = 0; // request status command
        let host_id = 0; // current host
        self.write_report_1(13, &[command, host_id])
    }

    /// Send query for DPI info
    pub fn query_dpi(&self) -> io::Result<()> {
        let host_id = 0; // current host
        let command = 4; // request status command, no save to flash not set
        self.write_report_1(
            17,
            &[
                host_id, command, 0, 0, // payload
            ],
        )
    }

    pub fn set_dpi(&self, dpi: u16) -> io::Result<()> {
        let host_id = 0; // current host
        let command = 0; // set dpi
        let dpi = dpi.to_le_bytes();
        self.write_report_1(17, &[host_id, command, dpi[0], dpi[1]])
    }

    pub fn set_left_handed(&self, left_handed: bool) -> io::Result<()> {
        let host_id = 0; // current host
        let command = 6; // set handedness
        let value = if left_handed { 1 } else { 0 };
        self.write_report_1(17, &[host_id, command, value, 0])
    }

    pub fn set_button(&self, button: Button, no_save_to_flash: bool) -> io::Result<()> {
        let command = 1;
        let no_save_to_flash = if no_save_to_flash { 1 << 7 } else { 0 };
        let mut data = vec![command | no_save_to_flash];
        button.encode(&mut data);
        self.write_report_1(13, &data)
    }

    pub fn exec_button(&self, button: Button) -> io::Result<()> {
        let command = 2;
        let host_id = 0;
        let mut data = vec![command, host_id];
        button.encode(&mut data);
        self.write_report_1(13, &data)
    }

    pub fn reset(&self) -> io::Result<()> {
        // TODO: Other devices may have different number of buttons?
        // TODO: Press types
        for id in 0..7 {
            for host in [1, 2, 3, 255] {
                let button = Button::new(id, host, PressType::Normal, &[]);
                self.set_button(button, false)?;
            }
        }
        self.set_left_handed(false)?;
        Ok(())
    }

    // Using multiple readers will result in inconsistent behavior
    pub fn read(&self) -> HpMouseEvents {
        HpMouseEvents::new(self.dev.clone())
    }
}

impl AsRawFd for HpMouse {
    fn as_raw_fd(&self) -> RawFd {
        self.dev.as_raw_fd()
    }
}

impl FromRawFd for HpMouse {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        HpMouse {
            dev: Arc::new(Hid::from_raw_fd(fd)),
        }
    }
}
