pub mod biome;
pub mod block;
pub mod coordinates;
pub mod cylindrical_chunk_iterator;
pub mod entity;
pub mod item;
pub const WORLD_HEIGHT: usize = 384;
pub const WORLD_LOWEST_Y: i16 = -64;
pub const WORLD_MAX_Y: i16 = WORLD_HEIGHT as i16 - WORLD_LOWEST_Y.abs();
pub const DIRECT_PALETTE_BITS: u32 = 15;