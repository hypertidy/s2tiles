# s2tiles

Arithmetic tile index for Sentinel-2 COGs.

Given an MGRS code, a COG's IFD structure, and a bounding box, computes which
internal tiles intersect — **no HTTP, no data fetching, pure geometry**.

## Architecture

One logic core in Rust, with thin bindings for R and Python planned:

```
s2tiles/
+-- s2tiles-core/       # Rust library crate (the single source of truth)
```

The core logic is written once. R and Python bindings will be thin wrappers
onto the same implementation, not ports.

## Rust

```rust
use s2tiles_core::{MgrsTile, IfdLevel};

let mgrs = MgrsTile::from_code("55HBU").unwrap();
let ifd = IfdLevel::new(0, 10980, 10980, 512, 512);
let grid = mgrs.tile_grid(&ifd, 10.0);

let hits = grid.query_utm(
    mgrs.easting_origin, mgrs.easting_origin + 10000.0,
    mgrs.northing_origin, mgrs.northing_origin + 10000.0,
);
```

## R

```r
library(s2tiles)
m <- mgrs_decode("55HBU")
hits <- tile_query("55HBU", 0L, 10980L, 10980L, 512L, 512L, 10,
                   m$easting_origin, m$easting_origin + 10000,
                   m$northing_origin, m$northing_origin + 10000)
```

## Python

```python
import s2tiles
m = s2tiles.mgrs_decode("55HBU")
hits = s2tiles.tile_query(
    "55HBU", 0, 10980, 10980, 512, 512, 10.0,
    m["easting_origin"], m["easting_origin"] + 10000,
    m["northing_origin"], m["northing_origin"] + 10000,
)
```

## Design notes

- **IFD dimensions are not hardcoded.** Sentinel-2 overview levels can vary by
  processing baseline. Seed the pyramid from one `tiff_refs` probe per
  MGRS/band combination.
- **MGRS decoding** uses the `geoconvert` crate (Rust translation of
  GeographicLib).
- **The tile grid is regular**, so spatial queries are O(1) arithmetic on the
  geotransform — no spatial index needed.
