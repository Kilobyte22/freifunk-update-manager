use std::fmt;
use std::str::FromStr;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeTuple;
use serde::de::{Visitor, SeqAccess};
use failure::_core::fmt::Formatter;
use crate::mac::unwrap_err;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct NodeID([u8; 6]);

impl Serialize for NodeID {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let mut tuple = serializer.serialize_tuple(6)?;
            for byte in &self.0 {
                tuple.serialize_element(byte)?;
            }
            tuple.end()
        }
    }
}

impl<'de> Deserialize<'de> for NodeID {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_string(NodeIDVisitor)
        } else {
            deserializer.deserialize_tuple(6, NodeIDVisitor)
        }
    }
}

struct NodeIDVisitor;

impl<'de> Visitor<'de> for NodeIDVisitor {
    type Value = NodeID;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("A valid NodeID (6 hexadecimal bytes)")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error
    {
        v.parse().map_err(|_| E::custom("Invalid NodeID"))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, <A as SeqAccess<'de>>::Error>
    where
        A: SeqAccess<'de>
    {
        Ok(NodeID([
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?,
            unwrap_err(seq.next_element()?)?
        ]))
    }
}

fn conv_error<T, E>(thing: Result<T, E>) -> Result<T, ()> {
    thing.map_err(|_| ())
}

impl fmt::Display for NodeID {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        if f.alternate() {
            write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}", self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
        } else {
            write!(f, "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
        }
    }
}

impl FromStr for NodeID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let macless_str = s.replace(":", "");
        if macless_str.len() != 12 {
            return Err(());
        }



        Ok(NodeID([
            conv_error(u8::from_str_radix(&macless_str[0..2], 16))?,
            conv_error(u8::from_str_radix(&macless_str[2..4], 16))?,
            conv_error(u8::from_str_radix(&macless_str[4..6], 16))?,
            conv_error(u8::from_str_radix(&macless_str[6..8], 16))?,
            conv_error(u8::from_str_radix(&macless_str[8..10], 16))?,
            conv_error(u8::from_str_radix(&macless_str[10..12], 16))?
        ]))
    }
}

#[test]
fn test_deserialize() {
    assert_eq!(
        "001122334455".parse::<NodeID>().unwrap(), 
        NodeID([00, 0x11, 0x22, 0x33, 0x44, 0x55])
    );
}

#[test]
fn test_deserialize_mac() {
    assert_eq!(
        "00:11:22:33:44:55".parse::<NodeID>().unwrap(), 
        NodeID([00, 0x11, 0x22, 0x33, 0x44, 0x55])
    );
}