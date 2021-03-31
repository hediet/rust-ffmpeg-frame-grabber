use serde::{de, Deserialize, Deserializer};
use std::fmt::Display;
use std::result::Result;
use std::str::FromStr;

pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

pub fn fractional_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s: Vec<&str> = s.split('/').collect();

    if s.len() != 2 {
        return Err(de::Error::custom("Cannot parse fraction".to_owned()));
    }

    let numerator = f64::from_str(s[0]).map_err(de::Error::custom)?;
    let denominator = f64::from_str(s[1]).map_err(de::Error::custom)?;

    return Ok(numerator / denominator);
}
