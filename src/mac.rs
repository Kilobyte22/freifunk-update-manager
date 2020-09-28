use std::fmt;
use std::str::FromStr;
use std::hash::Hash;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use serde::ser::SerializeTuple;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    pub fn to_bytes(&self) -> &[u8; 6] {
        &self.0
    }
    pub fn into_bytes(self) -> [u8; 6] { self.0 }

    pub fn from_bytes(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }

    pub fn unspecified() -> MacAddr {
        MacAddr([0; 6])
    }
}

impl fmt::Display for MacAddr {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        if f.alternate() {
            write!(f, "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
        } else {
            write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
        }
    }
}

impl fmt::Debug for MacAddr {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(f, "MacAddr({})", self)
    }
}

impl FromStr for MacAddr {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let mut data = [0; 6];
        for (idx, el) in s.split(':').take(6).enumerate() {
            data[idx] = u8::from_str_radix(el, 16)
                .map_err(|_| failure::err_msg("invalid mac address"))?;
        }
        Ok(MacAddr(data))
    }
}

impl Serialize for MacAddr {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where S: Serializer {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("{}", &self))
        } else {
            let mut tup = serializer.serialize_tuple(6)?;
            for i in 0..6 {
                tup.serialize_element(&self.0[i])?
            }
            tup.end()
        }
    }
}

struct MacAddrVisitor;
impl<'de> Visitor<'de> for MacAddrVisitor {
    type Value = MacAddr;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("A MAC address in colon seperated hex representation or a tuple of bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<<Self as Visitor<'de>>::Value, E> where E: de::Error, {
        v.parse().map_err(de::Error::custom)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<<Self as Visitor<'de>>::Value, <A as de::SeqAccess<'de>>::Error> where A: de::SeqAccess<'de>, {
        Ok(MacAddr([
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?
        ]))
    }
}

impl <'a> Deserialize<'a> for MacAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'a>>::Error> where D: Deserializer<'a> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(MacAddrVisitor)
        } else {
            deserializer.deserialize_tuple(6, MacAddrVisitor)
        }
    }
}

fn unwrap_err<T, E>(value: Option<T>) -> Result<T, E> where E: serde::de::Error {
    if let Some(v) = value {
        Ok(v)
    } else {
        Err(E::custom("Missing value"))
    }
}

#[test]
fn test_parse_mac() {
    assert_eq!("00:11:22:33:44:55".parse::<MacAddr>().unwrap(), MacAddr([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
}

#[test]
fn test_ser() {
    assert_eq!(::serde_json::to_string(&MacAddr([0x00, 0x11, 0x22, 0x33, 0x44, 0x55])).unwrap(), "\"00:11:22:33:44:55\"");
}

#[test]
fn test_deser() {
    assert_eq!(::serde_json::from_str::<MacAddr>("\"00:11:22:33:44:55\"").unwrap(), MacAddr([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
}

#[test]
fn test_display() {
    assert_eq!(&format!("{}", MacAddr([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])), "aa:bb:cc:dd:ee:ff")
}

#[test]
fn test_display_alt() {
    assert_eq!(&format!("{:#}", MacAddr([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])), "AA:BB:CC:DD:EE:FF")
}