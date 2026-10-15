//! Talking to the user's bucket over the S3 API — R2 in v1, any S3-compatible store by
//! construction (the POC's SPEC §9). Bytes always go direct to the bucket (N4).

pub mod s3;
pub mod sigv4;
