/*!
Base 32: encode numbers in a compact, URL-safe form
*/

use lazy_static::lazy_static;
use std::collections::HashMap;
use rocket::request::{FromParam, FromFormValue};
use rocket::http::RawStr;
use std::str::{Utf8Error, FromStr};
use std::fmt::{self, Display, Formatter};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use diesel::deserialize::{FromSql, Queryable};
use diesel::pg::Pg;
use std::error::Error;
use serde::{Serialize, Deserialize, Serializer, Deserializer, de};
use diesel::sql_types::BigInt;

pub const DIGITS: [char; 32] = [
    '2',
    '3',
    '4',
    '5',
    '6',
    '7',
    '8',
    '9',
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
    '_',
    '$',
    '.',
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
pub struct Base32Error;

impl Display for Base32Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "Invalid base128 number")
    }
}

impl Error for Base32Error {
    fn description(&self) -> &'static str {
        "Invalid base32 number"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Base32 {
    data: u64,
}

impl Base32 {
    pub fn into_u64(self) -> u64 {
        self.into()
    }
    pub fn from_u64(u: u64) -> Base32 {
        Self::from(u)
    }
    pub fn into_i64(self) -> i64 {
        self.into()
    }
    pub fn from_i64(i: i64) -> Base32 {
        Self::from(i)
    }
    pub fn zero() -> Base32 {
        Base32 { data: 0 }
    }
}

impl<DB> FromSql<BigInt, DB> for Base32
    where DB: diesel::backend::Backend,
          i64: FromSql<BigInt, DB> {
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error + Send + Sync>> {
        i64::from_sql(bytes).map(Into::into)
    }
}

impl Serialize for Base32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
        if serializer.is_human_readable() {
            serializer.serialize_str(&encode(self.data))
        } else {
            let mut bytes: [u8; 64 / 8] = [0; 64 / 8];
            let mut bytes: &mut [u8] = &mut bytes[..];
            bytes.write_u64::<BigEndian>(self.data).expect("64 bits should be writable to 64 bits");
            serializer.serialize_bytes(&bytes[..])
        }
    }
}

impl<'de> Deserialize<'de> for Base32 {
    fn deserialize<D>(deserializer: D) -> Result<Base32, D::Error> where
        D: Deserializer<'de> {
        if deserializer.is_human_readable() {
            struct Base128StringVisitor;
            impl<'vi> de::Visitor<'vi> for Base128StringVisitor {
                type Value = Base32;
                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    write!(formatter, "a base32 string")
                }
                fn visit_str<E: de::Error>(self, value: &str) -> Result<Base32, E> {
                    value.parse::<Base32>().map_err(E::custom)
                }
                fn visit_bytes<E: de::Error>(self, mut value: &[u8]) -> Result<Base32, E> {
                    let data = value.read_u64::<BigEndian>().expect("64 bits in a base64");
                    Ok(Base32 { data })
                }
            }
            deserializer.deserialize_str(Base128StringVisitor)
        } else {
            struct Base128BytesVisitor;
            impl<'vi> de::Visitor<'vi> for Base128BytesVisitor {
                type Value = Base32;
                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    write!(formatter, "bytes")
                }
                fn visit_bytes<E: de::Error>(self, mut value: &[u8]) -> Result<Base32, E> {
                    let data = value.read_u64::<BigEndian>().expect("128 bits in a base128");
                    Ok(Base32 { data })
                }
            }
            deserializer.deserialize_bytes(Base128BytesVisitor)
        }
    }
}

impl Queryable<BigInt, Pg> for Base32 {
    type Row = i64;

    fn build(row: Self::Row) -> Self {
        Base32::from_i64(<i64 as Queryable<BigInt, Pg>>::build(row))
    }
}

impl From<Utf8Error> for Base32Error {
    fn from(_: Utf8Error) -> Base32Error {
        Base32Error
    }
}

impl<'a> FromParam<'a> for Base32 {
    type Error = Base32Error;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        let data = decode(&param.percent_decode()?)?;
        Ok(Base32 { data })
    }
}

impl<'a> FromFormValue<'a> for Base32 {
    type Error = Base32Error;

    fn from_form_value(param: &'a RawStr) -> Result<Self, Self::Error> {
        let data = decode(&param.percent_decode()?)?;
        Ok(Base32 { data })
    }
}

impl rocket::http::uri::UriDisplay<rocket::http::uri::Query> for Base32 {
    fn fmt(&self, f: &mut rocket::http::uri::Formatter<rocket::http::uri::Query>) -> fmt::Result {
        f.write_named_value("post", &self.to_string())
    }
}

impl rocket::http::uri::UriDisplay<rocket::http::uri::Path> for Base32 {
    fn fmt(&self, f: &mut rocket::http::uri::Formatter<rocket::http::uri::Path>) -> fmt::Result {
        f.write_value(&self.to_string())
    }
}

impl<'a> rocket::http::uri::FromUriParam<rocket::http::uri::Query, &'a str> for Base32 {
    type Target = Base32;

    fn from_uri_param(page: &'a str) -> Base32 {
        Base32::from_str(page).unwrap_or(Base32 {data: 0})
    }
}

impl<'a> rocket::http::uri::FromUriParam<rocket::http::uri::Path, Base32> for Base32 {
    type Target = Base32;

    fn from_uri_param(page: Base32) -> Base32 {
        page
    }
}

impl From<u64> for Base32 {
    fn from(data: u64) -> Self {
        Base32 { data }
    }
}

impl Into<u64> for Base32 {
    fn into(self) -> u64 {
        self.data
    }
}

impl From<i64> for Base32 {
    fn from(data: i64) -> Self {
        Base32 { data: data as u64 }
    }
}

impl Into<i64> for Base32 {
    fn into(self) -> i64 {
        self.data as i64
    }
}

impl Into<String> for Base32 {
    fn into(self) -> String {
        encode(self.data)
    }
}

impl Display for Base32 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let string = encode(self.data);
        string.fmt(f)
    }
}

impl FromStr for Base32 {
    type Err = Base32Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Base32 { data: decode(s)? })
    }
}

fn encode(mut number: u64) -> String {
    if number == 0 {
        return "2".to_string();
    }
    let mut encoded = String::new();
    while number != 0 {
        let digit = (number & 0b_11_111) as usize;
        encoded.push(DIGITS[digit]);
        number = number >> 5;
    }
    encoded
}

fn decode(encoded: &str) -> Result<u64, Base32Error> {
    let mut decoded: u64 = 0;
    for c in encoded.chars().map(|c| c.to_ascii_uppercase()).rev() {
        if let Some(&digit) = DIGITS_BACK.get(&c) {
            decoded = decoded << 5;
            decoded |= digit as u64;
        } else {
            return Err(Base32Error);
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
    fn all_base_32_nfc_normalizes_to_one_char() {
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
        fn prop_round_trip(num: u64) -> bool {
            num == decode(&encode(num)).unwrap()
        }
        fn does_not_contain_hyphen(num: u64) -> bool {
            !encode(num).contains('-')
        }
        fn does_not_percent_encode(num: u64) -> bool {
            use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
            let s = utf8_percent_encode(&encode(num), DEFAULT_ENCODE_SET).to_string();
            !s.contains('%')
        }
    }
}
