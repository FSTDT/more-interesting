/*!
Base 128: use Unicode to encode numbers (like UUIDs) in very compact form.
*/

use lazy_static::lazy_static;
use std::collections::HashMap;
use rocket::request::{FromParam, FromFormValue};
use rocket::http::RawStr;
use std::str::{Utf8Error, FromStr};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use diesel::deserialize::{FromSql, Queryable};
use diesel::pg::Pg;
use diesel::pg::types::sql_types::Uuid as PgUuid;
use std::error::Error;
use serde::{Serialize, Deserialize, Serializer, Deserializer, de};
use chrono::serde::ts_seconds::deserialize;
use diesel::query_builder::Query;

pub const DIGITS: [char; 128] = [
    '0',
    '1',
    '2',
    '3',
    '4',
    '5',
    '6',
    '7',
    '8',
    '9',
    'b',
    'c',
    'd',
    'f',
    'g',
    'h',
    'j',
    'k',
    'l',
    'm',
    'n',
    'p',
    'q',
    'r',
    's',
    't',
    'v',
    'w',
    'x',
    'y',
    'z',
    'B',
    'C',
    'D',
    'F',
    'G',
    'H',
    'J',
    'K',
    'L',
    'M',
    'N',
    'P',
    'Q',
    'R',
    'S',
    'T',
    'V',
    'W',
    'X',
    'Y',
    'Z',
    'ø',
    'đ',
    'ł',
    'ẁ',
    'ỳ',
    'ǹ',
    'ß',
    'ẃ',
    'ṕ',
    'ǵ',
    'ĺ',
    'ý',
    'ŕ',
    'ś',
    'ń',
    'ź',
    'ć',
    'ḱ',
    'ḿ',
    'ẅ',
    'ẍ',
    'ÿ',
    'ǧ',
    'ȟ',
    'ǰ',
    'ǩ',
    'ň',
    'ñ',
    '!',
    'ŷ',
    'ħ',
    'ð',
    'þ',
    'Ø',
    'Đ',
    'Ł',
    'Ẁ',
    'Ỳ',
    'Ǹ',
    '.',
    '-',
    '+',
    'Ẃ',
    'Ṕ',
    'Ǵ',
    'Ĺ',
    'Ý',
    'Ŕ',
    'Ś',
    'Ń',
    '@',
    '~',
    '_',
    'Ź',
    'Ć',
    'Ḱ',
    'Ḿ',
    'Ẅ',
    ',',
    'Ç',
    'Ẍ',
    'ẘ',
    'Ǧ',
    'Ȟ',
    'Ǩ',
    'Ň',
    'Ñ',
    'Æ',
    'ẙ',
    '=',
    'ç',
    '$',
    'Ŷ',
    'Ħ',
    'Ð',
    'Þ',
];

lazy_static!{
    static ref DIGITS_BACK: HashMap<char, u32> = {
        let mut map = HashMap::new();
        for (i, &c) in DIGITS[..].into_iter().enumerate() {
            map.insert(c, i as u32);
        }
        map
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Base128Error;

impl Display for Base128Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "Invalid base128 number")
    }
}

impl Error for Base128Error {
    fn description(&self) -> &'static str {
        "Invalid base128 number"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Base128 {
    data: u128,
}

impl Base128 {
    pub fn into_uuid(self) -> Uuid {
        self.into()
    }
    pub fn from_uuid(u: Uuid) -> Base128 {
        Self::from(u)
    }
}

impl FromSql<PgUuid, Pg> for Base128 {
    fn from_sql(bytes: Option<&<Pg as diesel::backend::Backend>::RawValue>) -> Result<Self, Box<Error + Send + Sync>> {
        Uuid::from_sql(bytes).map(Into::into)
    }
}

impl Serialize for Base128 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
        if serializer.is_human_readable() {
            serializer.serialize_str(&encode(self.data))
        } else {
            let mut bytes: [u8; 128 / 8] = [0; 128 / 8];
            let mut bytes: &mut [u8] = &mut bytes[..];
            bytes.write_u128::<BigEndian>(self.data).expect("128 bits should be writable to 128 bits");
            serializer.serialize_bytes(&bytes[..])
        }
    }
}

impl<'de> Deserialize<'de> for Base128 {
    fn deserialize<D>(deserializer: D) -> Result<Base128, D::Error> where
        D: Deserializer<'de> {
        if deserializer.is_human_readable() {
            struct Base128StringVisitor;
            impl<'vi> de::Visitor<'vi> for Base128StringVisitor {
                type Value = Base128;
                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    write!(formatter, "a base128 string")
                }
                fn visit_str<E: de::Error>(self, value: &str) -> Result<Base128, E> {
                    value.parse::<Base128>().map_err(E::custom)
                }
                fn visit_bytes<E: de::Error>(self, mut value: &[u8]) -> Result<Base128, E> {
                    let data = value.read_u128::<BigEndian>().expect("128 bits in a base128");
                    Ok(Base128 { data })
                }
            }
            deserializer.deserialize_str(Base128StringVisitor)
        } else {
            struct Base128BytesVisitor;
            impl<'vi> de::Visitor<'vi> for Base128BytesVisitor {
                type Value = Base128;
                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    write!(formatter, "bytes")
                }
                fn visit_bytes<E: de::Error>(self, mut value: &[u8]) -> Result<Base128, E> {
                    let data = value.read_u128::<BigEndian>().expect("128 bits in a base128");
                    Ok(Base128 { data })
                }
            }
            deserializer.deserialize_bytes(Base128BytesVisitor)
        }
    }
}

impl Queryable<PgUuid, Pg> for Base128 {
    type Row = Uuid;

    fn build(row: Self::Row) -> Self {
        <Uuid as Queryable<PgUuid, Pg>>::build(row).into()
    }
}

impl From<Utf8Error> for Base128Error {
    fn from(_: Utf8Error) -> Base128Error {
        Base128Error
    }
}

impl<'a> FromParam<'a> for Base128 {
    type Error = Base128Error;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        let data = decode(&param.percent_decode()?)?;
        Ok(Base128{ data })
    }
}

impl<'a> FromFormValue<'a> for Base128 {
    type Error = Base128Error;

    fn from_form_value(param: &'a RawStr) -> Result<Self, Self::Error> {
        let data = decode(&param.percent_decode()?)?;
        Ok(Base128{ data })
    }
}

impl From<Uuid> for Base128 {
    fn from(uuid: Uuid) -> Self {
        let data = uuid_to_u128(&uuid);
        Base128 { data }
    }
}

impl Into<Uuid> for Base128 {
    fn into(self) -> Uuid {
        u128_to_uuid(self.data)
    }
}

impl Into<String> for Base128 {
    fn into(self) -> String {
        encode(self.data)
    }
}

impl Display for Base128 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let string = encode(self.data);
        string.fmt(f)
    }
}

impl FromStr for Base128 {
    type Err = Base128Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Base128 { data: decode(s)? })
    }
}

fn uuid_to_u128(uuid: &Uuid) -> u128 {
    let v = uuid.as_bytes();
    let mut v = &v[..];
    v.read_u128::<BigEndian>().expect("128 bits in a UUID")
}

fn u128_to_uuid(number: u128) -> Uuid {
    let mut bytes: [u8; 128 / 8] = [0; 128 / 8];
    let mut bytes_: &mut [u8] = &mut bytes[..];
    bytes_.write_u128::<BigEndian>(number).expect("128 bits should be writable to 128 bits");
    Uuid::from_bytes(&bytes).expect("128 bit byte array should be convertible to a UUID")
}

fn encode(mut number: u128) -> String {
    if number == 0 {
        return "0".to_string();
    }
    let mut encoded = String::new();
    while number != 0 {
        let digit = (number & 0b_1_111_111) as usize;
        encoded.push(DIGITS[digit]);
        number = number >> 7;
    }
    encoded
}

fn decode(encoded: &str) -> Result<u128, Base128Error> {
    let mut decoded: u128 = 0;
    for c in encoded.chars().rev() {
        if let Some(&digit) = DIGITS_BACK.get(&c) {
            decoded = decoded << 7;
            decoded |= digit as u128;
        } else {
            return Err(Base128Error);
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;
    use std::collections::HashSet;
    use quickcheck::quickcheck;

    // http://unicode.org/reports/tr15/
    // https://docs.rs/unicode-normalization/0.1.8/unicode_normalization/trait.UnicodeNormalization.html
    #[test]
    fn all_base_128_nfc_normalizes_to_one_char() {
        let mut s = String::new();
        let mut s2 = String::new();
        for &c in &DIGITS[..] {
            s.clear();
            s.push(c);
            s2.clear();
            s2.extend(s.nfc());
            if s != s2 {
                panic!("Digit does not round-trip through NFC: {} -> {}", s, s2);
            }
        }
    }
    #[test]
    fn no_duplicates() {
        let mut h = HashSet::new();
        for &c in &DIGITS[..] {
            if h.contains(&c) {
                panic!("duplicate {}", c);
            }
            h.insert(c);
        }
    }
    quickcheck!{
        fn prop_round_trip(num: u128) -> bool {
            num == decode(&encode(num)).unwrap()
        }
    }
}
