use std::str::FromStr;

use rusqlite::types::Type;

use crate::error::StorageError;

pub fn encode_f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|value| value.to_le_bytes()).collect()
}

pub fn decode_f32s(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

pub fn parse_id<T: FromStr>(value: String) -> rusqlite::Result<T>
where
    T::Err: std::fmt::Display,
{
    value.parse::<T>().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(StorageError::Id(error.to_string())),
        )
    })
}

pub fn parse_enum<T, F>(value: String, map_err: F) -> rusqlite::Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    F: FnOnce(String) -> StorageError,
{
    value.parse::<T>().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(map_err(error.to_string())))
    })
}

pub fn parse_pg<T: FromStr>(value: String) -> Result<T, StorageError>
where
    T::Err: std::fmt::Display,
{
    value.parse::<T>().map_err(|error| StorageError::Id(error.to_string()))
}
