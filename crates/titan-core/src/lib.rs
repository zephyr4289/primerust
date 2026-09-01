//! titan-core: The correctness substrate for prime computation.
//!
//! Provides compile-time constant lookup tables, exact integer roots,
//! Convention A Wheel-30 factorization, constant-time PhiTiny tables,
//! and borrowed zero-allocation bit window views.

pub mod bit_array;
pub mod phi_tiny;
pub mod roots;
pub mod tripwire;
pub mod wheel;
