use bitmask_enum::bitmask;
use std::fmt::{self, Display};

// === Enums ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Keyboard(Keycode),
    Consumer(ConsumerKey),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keycode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    Minus,
    Equal,
    LBracket,
    RBracket,
    Backslash,
    NonUSHash,
    Semicolon,
    Quote,
    Comma,
    Dot,
    Slash,
    Backtick,
    Space,
    Enter,
    Tab,
    Backspace,
    Escape,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Up,
    Down,
    Left,
    Right,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    NumLock,
    KpSlash,
    KpAsterisk,
    KpMinus,
    KpPlus,
    KpEnter,
    KpDot,
    LCtrl,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerKey {
    PlayPause,
    Stop,
    NextTrack,
    PrevTrack,
    Mute,
    VolumeUp,
    VolumeDown,
    None,
    Unknown(u16),
}

#[bitmask(u8)]
pub enum ModifierKey {
    LCtrl,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui,
}

impl Display for ModifierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// === HidEvent ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub modifier_bitmask: ModifierKey,
    pub keys: [Key; 6], // Cached: Translated once during parse
    pub is_consumer: bool,
}

impl KeyEvent {
    /// Returns true if any modifier is currently active.
    #[inline]
    pub fn has_modifiers(&self) -> bool {
        !self.is_consumer && self.modifier_bitmask != 0
    }

    /// Returns true if given modifier is active
    #[inline]
    pub fn has_modifier(&self, modifier: ModifierKey) -> bool {
        !self.is_consumer && self.modifier_bitmask.contains(modifier)
    }

    /// Returns an iterator of modifiers
    pub fn modifiers(&self) -> impl Iterator<Item = ModifierKey> {
        let bits = self.modifier_bitmask.bits();
        (0..8).filter_map(move |bit| {
            let mask = 1 << bit;
            if bits & mask != 0 {
                // Construct the bitmask from the raw bit
                Some(ModifierKey { bits: mask })
            } else {
                None
            }
        })
    }

    /// Returns true if any key is pressed.
    #[inline]
    pub fn has_keys(&self) -> bool {
        self.keys[0] != Key::None
    }

    /// Returns true if target key is contained in this event
    pub fn contains_key(&self, target: Key) -> bool {
        if target == Key::None {
            return false;
        }
        // Simple unrolled loop or any() over the cached array
        self.keys.iter().any(|&k| k == target)
    }

    pub fn contains(&self, subset: KeyEvent) -> bool {
        self.modifier_bitmask & subset.modifier_bitmask != 0
    }

    /// Returns true if no key is pressed
    #[inline]
    pub fn empty(&self) -> bool {
        !self.has_keys() && !self.has_modifiers()
    }
}

// === Parser Logic ===

pub struct KeyParser;

impl KeyParser {
    pub fn parse(report: &[u8]) -> Option<KeyEvent> {
        let n = report.len();
        if n == 0 {
            return None;
        }

        let mut raw = [0u8; 8];
        let is_consumer = match report[0] {
            0x01 if n >= 9 => {
                raw.copy_from_slice(&report[1..9]);
                false
            }
            0x02 | 0x03 if n >= 3 => {
                raw[0] = report[1];
                raw[1] = report[2];
                true
            }
            _ if n == 8 => {
                raw.copy_from_slice(report);
                false
            }
            _ if n == 2 => {
                raw[0] = report[0];
                raw[1] = report[1];
                true
            }
            _ => return None,
        };

        // Translate and Cache keys once here
        let mut keys = [Key::None; 6];
        let mut modifier_bits: u8 = 0;
        if is_consumer {
            let usage = u16::from_le_bytes([raw[0], raw[1]]);
            if usage != 0 {
                keys[0] = Key::Consumer(match usage {
                    0x00CD => ConsumerKey::PlayPause,
                    0x00B7 => ConsumerKey::Stop,
                    0x00B5 => ConsumerKey::NextTrack,
                    0x00B6 => ConsumerKey::PrevTrack,
                    0x00E2 => ConsumerKey::Mute,
                    0x00E9 => ConsumerKey::VolumeUp,
                    0x00EA => ConsumerKey::VolumeDown,
                    _ => ConsumerKey::Unknown(usage),
                });
            }
        } else {
            modifier_bits = raw[0];
            for i in 0..6 {
                if let Some(kc) = Keycode::from_hid(raw[i + 2]) {
                    keys[i] = Key::Keyboard(kc);
                }
            }
        }

        Some(KeyEvent {
            modifier_bitmask: ModifierKey {
                bits: modifier_bits,
            },
            keys,
            is_consumer,
        })
    }
}

// === Implementation Helpers ===

pub struct ModifierIter {
    byte: u8,
    index: u8,
}

impl Iterator for ModifierIter {
    type Item = &'static str;
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < 8 {
            let bit = 1 << self.index;
            self.index += 1;
            if self.byte & bit != 0 {
                return Some(match bit {
                    0x01 => "LCtrl",
                    0x02 => "LShift",
                    0x04 => "LAlt",
                    0x08 => "LGui",
                    0x10 => "RCtrl",
                    0x20 => "RShift",
                    0x40 => "RAlt",
                    0x80 => "RGui",
                    _ => unreachable!(),
                });
            }
        }
        None
    }
}

impl Keycode {
    pub fn from_hid(code: u8) -> Option<Keycode> {
        use Keycode::*;
        if code == 0 {
            return None;
        }
        Some(match code {
            // Letters A-Z
            0x04..=0x1D => {
                // Use a simple match or a safer offset if the enum remains stable
                match code {
                    0x04 => A,
                    0x05 => B,
                    0x06 => C,
                    0x07 => D,
                    0x08 => E,
                    0x09 => F,
                    0x0A => G,
                    0x0B => H,
                    0x0C => I,
                    0x0D => J,
                    0x0E => K,
                    0x0F => L,
                    0x10 => M,
                    0x11 => N,
                    0x12 => O,
                    0x13 => P,
                    0x14 => Q,
                    0x15 => R,
                    0x16 => S,
                    0x17 => T,
                    0x18 => U,
                    0x19 => V,
                    0x1A => W,
                    0x1B => X,
                    0x1C => Y,
                    0x1D => Z,
                    _ => unreachable!(),
                }
            }
            // Numbers
            0x1E..=0x26 => {
                [Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9][(code - 0x1E) as usize]
            }
            0x27 => Num0,
            // Specials
            0x28 => Enter,
            0x29 => Escape,
            0x2A => Backspace,
            0x2B => Tab,
            0x2C => Space,
            0x2D => Minus,
            0x2E => Equal,
            0x2F => LBracket,
            0x30 => RBracket,
            0x31 => Backslash,
            0x32 => NonUSHash,
            0x33 => Semicolon,
            0x34 => Quote,
            0x35 => Comma,
            0x36 => Dot,
            0x37 => Slash,
            0x38 => Backtick,
            // Function Keys
            0x3A..=0x45 => match code {
                0x3A => F1,
                0x3B => F2,
                0x3C => F3,
                0x3D => F4,
                0x3E => F5,
                0x3F => F6,
                0x40 => F7,
                0x41 => F8,
                0x42 => F9,
                0x43 => F10,
                0x44 => F11,
                0x45 => F12,
                _ => unreachable!(),
            },
            0x49 => Insert,
            0x4A => Home,
            0x4B => PageUp,
            0x4C => Delete,
            0x4D => End,
            0x4E => PageDown,
            0x4F => Right,
            0x50 => Left,
            0x51 => Down,
            0x52 => Up,
            0x53 => NumLock,
            0x54 => KpSlash,
            0x55 => KpAsterisk,
            0x56 => KpMinus,
            0x57 => KpPlus,
            0x58 => KpEnter,
            0x59 => KpDot,
            _ => Unknown(code),
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Keycode::A => "A",
            Keycode::B => "B",
            Keycode::C => "C",
            Keycode::D => "D",
            Keycode::E => "E",
            Keycode::F => "F",
            Keycode::G => "G",
            Keycode::H => "H",
            Keycode::I => "I",
            Keycode::J => "J",
            Keycode::K => "K",
            Keycode::L => "L",
            Keycode::M => "M",
            Keycode::N => "N",
            Keycode::O => "O",
            Keycode::P => "P",
            Keycode::Q => "Q",
            Keycode::R => "R",
            Keycode::S => "S",
            Keycode::T => "T",
            Keycode::U => "U",
            Keycode::V => "V",
            Keycode::W => "W",
            Keycode::X => "X",
            Keycode::Y => "Y",
            Keycode::Z => "Z",
            Keycode::Num1 => "1",
            Keycode::Num2 => "2",
            Keycode::Num3 => "3",
            Keycode::Num4 => "4",
            Keycode::Num5 => "5",
            Keycode::Num6 => "6",
            Keycode::Num7 => "7",
            Keycode::Num8 => "8",
            Keycode::Num9 => "9",
            Keycode::Num0 => "0",
            Keycode::Minus => "-",
            Keycode::Equal => "=",
            Keycode::LBracket => "[",
            Keycode::RBracket => "]",
            Keycode::Backslash => "\\",
            Keycode::NonUSHash => "#",
            Keycode::Semicolon => ";",
            Keycode::Quote => "'",
            Keycode::Comma => ",",
            Keycode::Dot => ".",
            Keycode::Slash => "/",
            Keycode::Backtick => "`",
            Keycode::Space => "Space",
            Keycode::Enter => "Enter",
            Keycode::Tab => "Tab",
            Keycode::Backspace => "Backspace",
            Keycode::Escape => "Esc",
            Keycode::Home => "Home",
            Keycode::End => "End",
            Keycode::PageUp => "PgUp",
            Keycode::PageDown => "PgDown",
            Keycode::Insert => "Insert",
            Keycode::Delete => "Delete",
            Keycode::Up => "Up",
            Keycode::Down => "Down",
            Keycode::Left => "Left",
            Keycode::Right => "Right",
            Keycode::F1 => "F1",
            Keycode::F12 => "F12", // ... (simplified for brevity)
            _ => "Key",
        }
    }
}

// === Display Trait Impls ===

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Keyboard(k) => write!(f, "{:?}", k),
            Key::Consumer(c) => write!(f, "{:?}", c),
            Key::None => Ok(()),
        }
    }
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Modifiers
        write!(f, "[mods=")?;
        let mut first = true;
        for m in self.modifiers() {
            if !first {
                write!(f, "+")?;
            }
            write!(f, "{}", m)?;
            first = false;
        }
        if first {
            write!(f, "-")?;
        }

        // Keys
        write!(f, " keys=")?;
        let mut first = true;
        for k in self.keys.iter().filter(|&&k| k != Key::None) {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", k)?;
            first = false;
        }
        if first {
            write!(f, "-")?;
        }

        write!(f, "]")
    }
}
