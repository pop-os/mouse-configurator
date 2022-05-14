#![allow(unused, non_upper_case_globals, overflowing_literals)]

// Matches /sys/kernel/debug/hid/*/rdesc

pub const MOD_Ctrl: i8 = 1 << 0;
pub const MOD_Shift: i8 = 1 << 1;
pub const MOD_Alt: i8 = 1 << 2;
pub const MOD_Super: i8 = 1 << 3;

pub const KEY_A: i8 = 0x04;
pub const KEY_B: i8 = 0x05;
pub const KEY_C: i8 = 0x06;
pub const KEY_D: i8 = 0x07;
pub const KEY_E: i8 = 0x08;
pub const KEY_F: i8 = 0x09;
pub const KEY_G: i8 = 0x0A;
pub const KEY_H: i8 = 0x0B;
pub const KEY_I: i8 = 0x0C;
pub const KEY_J: i8 = 0x0D;
pub const KEY_K: i8 = 0x0E;
pub const KEY_L: i8 = 0x0F;
pub const KEY_M: i8 = 0x10;
pub const KEY_N: i8 = 0x11;
pub const KEY_O: i8 = 0x12;
pub const KEY_P: i8 = 0x13;
pub const KEY_Q: i8 = 0x14;
pub const KEY_R: i8 = 0x15;
pub const KEY_S: i8 = 0x16;
pub const KEY_T: i8 = 0x17;
pub const KEY_U: i8 = 0x18;
pub const KEY_V: i8 = 0x19;
pub const KEY_W: i8 = 0x1A;
pub const KEY_X: i8 = 0x1B;
pub const KEY_Y: i8 = 0x1C;
pub const KEY_Z: i8 = 0x1D;
pub const KEY_1: i8 = 0x1E;
pub const KEY_2: i8 = 0x1F;
pub const KEY_3: i8 = 0x20;
pub const KEY_4: i8 = 0x21;
pub const KEY_5: i8 = 0x22;
pub const KEY_6: i8 = 0x23;
pub const KEY_7: i8 = 0x24;
pub const KEY_8: i8 = 0x25;
pub const KEY_9: i8 = 0x26;
pub const KEY_0: i8 = 0x27;
pub const KEY_Enter: i8 = 0x28;
pub const KEY_Esc: i8 = 0x29;
pub const KEY_Backspace: i8 = 0x2A;
pub const KEY_Tab: i8 = 0x2B;
pub const KEY_Space: i8 = 0x2C;
pub const KEY_Minus: i8 = 0x2D;
pub const KEY_Equal: i8 = 0x2E;
pub const KEY_LeftBrace: i8 = 0x2F;
pub const KEY_RightBrace: i8 = 0x30;
// ...
pub const KEY_Slash: i8 = 0x38;
// ...
pub const KEY_F1: i8 = 0x3A;
pub const KEY_F2: i8 = 0x3B;
pub const KEY_F3: i8 = 0x3C;
pub const KEY_F4: i8 = 0x3D;
pub const KEY_F5: i8 = 0x3E;
pub const KEY_F6: i8 = 0x3F;
pub const KEY_F7: i8 = 0x40;
pub const KEY_F8: i8 = 0x41;
pub const KEY_F9: i8 = 0x42;
pub const KEY_F10: i8 = 0x43;
pub const KEY_F11: i8 = 0x44;
pub const KEY_F12: i8 = 0x45;
pub const KEY_SysRq: i8 = 0x46;
// ...
pub const KEY_Home: i8 = 0x4A;
pub const KEY_PageUp: i8 = 0x4B;
pub const KEY_Delete: i8 = 0x4C;
pub const KEY_End: i8 = 0x4D;
pub const KEY_PageDown: i8 = 0x4E;
pub const KEY_Right: i8 = 0x4F;
pub const KEY_Left: i8 = 0x50;
pub const KEY_Down: i8 = 0x51;
pub const KEY_Up: i8 = 0x52;

// Consumer page
pub const MEDIA_Restart: i8 = 0x31;
// ...
pub const MEDIA_BrightnessUp: i8 = 0x6F;
pub const MEDIA_BrightnessDown: i8 = 0x70;
// ...
pub const MEDIA_Play: i8 = 0xB0;
pub const MEDIA_Pause: i8 = 0xB1;
pub const MEDIA_Record: i8 = 0xB2;
pub const MEDIA_FastForward: i8 = 0xB3;
pub const MEDIA_Rewind: i8 = 0xB4;
pub const MEDIA_NextSong: i8 = 0xB5;
pub const MEDIA_PreviousSong: i8 = 0xB6;
// ...
pub const MEDIA_PlayPause: i8 = 0xCD;
// ...
pub const MEDIA_Mute: i8 = 0xE2;
// ...
pub const MEDIA_VolumeUp: i8 = 0xE9;
pub const MEDIA_VolumeDown: i8 = 0xEA;

// TODO: Other supported codes, as needed
