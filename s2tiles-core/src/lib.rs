//! # s2tiles
//!
//! Arithmetic tile index for Sentinel-2 COGs.
//!
//! Given an MGRS code, a COG's IFD structure (image dimensions, tile dimensions),
//! and a geotransform, compute which `(tile_col, tile_row)` entries intersect
//! an arbitrary bounding box, and their geographic extents.
//!
//! No HTTP, no data fetching — pure geometry.
//!
//! ## Design
//!
//! The IFD pyramid dimensions are **not hardcoded**. Sentinel-2 overview levels
//! can vary by processing baseline and GDAL version used during JP2K→COG conversion.
//! Instead, `IfdLevel` accepts the actual dimensions from a single `tiff_refs` probe
//! against one representative COG per MGRS/band combination.
//!
//! ## Usage
//!
//! ```rust
//! use s2tiles_core::{MgrsTile, IfdLevel, TileHit};
//!
//! // Construct from MGRS code — gets UTM origin + EPSG
//! let mgrs = MgrsTile::from_code("55HBU").unwrap();
//!
//! // Define the IFD level (from one tiff_refs call)
//! let ifd = IfdLevel::new(
//!     0,          // ifd index (0 = native resolution)
//!     10980,      // image_width
//!     10980,      // image_height
//!     512,        // tile_width
//!     512,        // tile_height
//! );
//!
//! // Build the tile grid for this MGRS tile at this IFD level
//! let grid = mgrs.tile_grid(&ifd, 10.0); // 10m pixel size
//!
//! // Query: which tiles intersect this UTM bounding box?
//! let hits = grid.query_utm(500000.0, 510000.0, 6000000.0, 6010000.0);
//! ```

mod mgrs_tile;
pub mod ifd;
mod grid;

pub use mgrs_tile::MgrsTile;
pub use ifd::IfdLevel;
pub use grid::{TileGrid, TileHit};
