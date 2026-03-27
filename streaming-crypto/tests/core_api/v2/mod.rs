#[cfg(feature = "core-api")]
mod test_public_api;
#[cfg(feature = "core-api")]
mod test_header_meta;

#[cfg(feature = "core-api")]
mod test_input_variants;
#[cfg(feature = "core-api")]
mod test_output_variants;
#[cfg(feature = "core-api")]
mod test_compression_cipher_variants;

#[cfg(feature = "core-api")]
mod test_roundtrip;
#[cfg(feature = "core-api")]
mod test_parallelism_profiles;
#[cfg(feature = "core-api")]
mod test_corrupt_detection;
#[cfg(feature = "core-api")]
mod test_telemetry_config;
