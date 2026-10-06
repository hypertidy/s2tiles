use crate::ifd::IfdLevel;
use crate::mgrs_tile::MgrsTile;

/// A tile grid: the combination of an MGRS tile, IFD level, and geotransform.
///
/// This is the query-able index. Given a bounding box in UTM coordinates,
/// it returns the set of COG tiles that intersect.
#[derive(Debug, Clone)]
pub struct TileGrid {
    pub mgrs: MgrsTile,
    pub ifd: IfdLevel,
    /// GDAL-style geotransform [ul_x, pixel_w, 0, ul_y, 0, -pixel_h]
    pub gt: [f64; 6],
}

/// A single tile hit from a spatial query.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TileHit {
    /// Column index of the tile (0-based)
    pub tile_col: u32,
    /// Row index of the tile (0-based)
    pub tile_row: u32,
    /// Linear tile index: tile_row * n_tile_cols + tile_col
    pub tile_index: u32,
    /// Western edge of this tile in the grid CRS (UTM metres)
    pub xmin: f64,
    /// Eastern edge of this tile in the grid CRS
    pub xmax: f64,
    /// Southern edge of this tile in the grid CRS
    pub ymin: f64,
    /// Northern edge of this tile in the grid CRS
    pub ymax: f64,
}

impl TileGrid {
    /// Create a new TileGrid from its components.
    pub fn new(mgrs: MgrsTile, ifd: IfdLevel, gt: [f64; 6]) -> Self {
        TileGrid { mgrs, ifd, gt }
    }

    /// Pixel width (gt[1])
    pub fn pixel_width(&self) -> f64 {
        self.gt[1]
    }

    /// Pixel height (absolute value of gt[5])
    pub fn pixel_height(&self) -> f64 {
        self.gt[5].abs()
    }

    /// The full extent of the image in UTM coordinates.
    pub fn image_extent(&self) -> (f64, f64, f64, f64) {
        let xmin = self.gt[0];
        let ymax = self.gt[3];
        let xmax = xmin + self.ifd.image_width as f64 * self.gt[1];
        let ymin = ymax + self.ifd.image_height as f64 * self.gt[5]; // gt[5] is negative
        (xmin, xmax, ymin, ymax)
    }

    /// Compute the UTM bounding box for a specific tile.
    ///
    /// The last tile in each row/column may extend beyond the image extent
    /// (partial tile padding). We clamp to the image extent.
    pub fn tile_extent(&self, tile_col: u32, tile_row: u32) -> (f64, f64, f64, f64) {
        let (img_xmin, img_xmax, img_ymin, img_ymax) = self.image_extent();
        let pw = self.gt[1];
        let ph = self.gt[5]; // negative

        // Tile UL corner in pixel space
        let px_left = tile_col * self.ifd.tile_width;
        let px_top = tile_row * self.ifd.tile_height;
        // Tile LR corner in pixel space (may exceed image dims for edge tiles)
        let px_right = ((tile_col + 1) * self.ifd.tile_width).min(self.ifd.image_width);
        let px_bottom = ((tile_row + 1) * self.ifd.tile_height).min(self.ifd.image_height);

        let xmin = self.gt[0] + px_left as f64 * pw;
        let xmax = self.gt[0] + px_right as f64 * pw;
        let ymax_tile = self.gt[3] + px_top as f64 * ph;
        let ymin_tile = self.gt[3] + px_bottom as f64 * ph;

        // Clamp to image extent
        let xmin = xmin.max(img_xmin);
        let xmax = xmax.min(img_xmax);
        let ymin = ymin_tile.max(img_ymin);
        let ymax = ymax_tile.min(img_ymax);

        (xmin, xmax, ymin, ymax)
    }

    /// Enumerate all tiles and their extents.
    pub fn all_tiles(&self) -> Vec<TileHit> {
        let mut hits = Vec::with_capacity(self.ifd.n_tiles() as usize);
        for row in 0..self.ifd.n_tile_rows {
            for col in 0..self.ifd.n_tile_cols {
                let (xmin, xmax, ymin, ymax) = self.tile_extent(col, row);
                hits.push(TileHit {
                    tile_col: col,
                    tile_row: row,
                    tile_index: row * self.ifd.n_tile_cols + col,
                    xmin,
                    xmax,
                    ymin,
                    ymax,
                });
            }
        }
        hits
    }

    /// Query: which tiles intersect the given UTM bounding box?
    ///
    /// The bbox is in the same CRS as the grid (UTM easting/northing in metres).
    /// Returns only tiles whose extent overlaps the query bbox.
    ///
    /// This is pure arithmetic — no spatial index needed because the grid is regular.
    /// We compute the tile column/row range directly from the geotransform.
    pub fn query_utm(
        &self,
        qxmin: f64,
        qxmax: f64,
        qymin: f64,
        qymax: f64,
    ) -> Vec<TileHit> {
        let pw = self.gt[1];
        let ph = self.gt[5].abs();
        let tw = self.ifd.tile_width as f64 * pw;
        let th = self.ifd.tile_height as f64 * ph;

        // Image UL corner
        let img_xmin = self.gt[0];
        let img_ymax = self.gt[3];

        // Convert query bbox to fractional tile indices
        // Column increases eastward, row increases southward (because gt[5] < 0)
        let col_min_f = (qxmin - img_xmin) / tw;
        let col_max_f = (qxmax - img_xmin) / tw;
        let row_min_f = (img_ymax - qymax) / th; // note: ymax of query → min row
        let row_max_f = (img_ymax - qymin) / th;

        // Clamp to valid tile range
        let col_start = (col_min_f.floor() as i64).max(0) as u32;
        let col_end = (col_max_f.ceil() as i64).min(self.ifd.n_tile_cols as i64) as u32;
        let row_start = (row_min_f.floor() as i64).max(0) as u32;
        let row_end = (row_max_f.ceil() as i64).min(self.ifd.n_tile_rows as i64) as u32;

        if col_start >= col_end || row_start >= row_end {
            return Vec::new();
        }

        let mut hits = Vec::with_capacity(
            ((col_end - col_start) * (row_end - row_start)) as usize,
        );

        for row in row_start..row_end {
            for col in col_start..col_end {
                let (xmin, xmax, ymin, ymax) = self.tile_extent(col, row);
                // Double-check actual overlap (handles edge tiles correctly)
                if xmin < qxmax && xmax > qxmin && ymin < qymax && ymax > qymin {
                    hits.push(TileHit {
                        tile_col: col,
                        tile_row: row,
                        tile_index: row * self.ifd.n_tile_cols + col,
                        xmin,
                        xmax,
                        ymin,
                        ymax,
                    });
                }
            }
        }

        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ifd::IfdLevel;
    use crate::mgrs_tile::MgrsTile;

    fn make_test_grid() -> TileGrid {
        // Synthetic grid: 1024×1024 image, 256×256 tiles, 10m pixels
        // Origin at (500000, 6000000) in UTM
        let mgrs = MgrsTile {
            code: "TEST".to_string(),
            zone: 55,
            north: false,
            epsg: 32755,
            easting_origin: 500000.0,
            northing_origin: 6000000.0,
        };
        let ifd = IfdLevel::new(0, 1024, 1024, 256, 256);
        let gt = [
            500000.0, // UL easting
            10.0,     // pixel width
            0.0,
            6010240.0, // UL northing = 6000000 + 1024*10
            0.0,
            -10.0, // pixel height
        ];
        TileGrid::new(mgrs, ifd, gt)
    }

    #[test]
    fn test_image_extent() {
        let grid = make_test_grid();
        let (xmin, xmax, ymin, ymax) = grid.image_extent();
        assert!((xmin - 500000.0).abs() < 1e-6);
        assert!((xmax - 510240.0).abs() < 1e-6);
        assert!((ymin - 6000000.0).abs() < 1e-6);
        assert!((ymax - 6010240.0).abs() < 1e-6);
    }

    #[test]
    fn test_all_tiles_count() {
        let grid = make_test_grid();
        let tiles = grid.all_tiles();
        assert_eq!(tiles.len(), 16); // 4×4
    }

    #[test]
    fn test_query_single_tile() {
        let grid = make_test_grid();
        // Query a small bbox that should hit exactly one tile (tile 0,0 = UL corner)
        let hits = grid.query_utm(500100.0, 500200.0, 6009800.0, 6010100.0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].tile_col, 0);
        assert_eq!(hits[0].tile_row, 0);
    }

    #[test]
    fn test_query_spanning_tiles() {
        let grid = make_test_grid();
        // Query a bbox that spans a tile boundary
        // Tile width = 256 * 10 = 2560m. So 500000..502560 is tile col 0,
        // 502560..505120 is tile col 1.
        // A query at 502500..502600 spans the boundary → 2 tile columns.
        let hits = grid.query_utm(502500.0, 502600.0, 6005000.0, 6005100.0);
        assert_eq!(hits.len(), 2);
        let cols: Vec<u32> = hits.iter().map(|h| h.tile_col).collect();
        assert!(cols.contains(&0));
        assert!(cols.contains(&1));
    }

    #[test]
    fn test_query_outside_returns_empty() {
        let grid = make_test_grid();
        // Query completely outside the image
        let hits = grid.query_utm(600000.0, 601000.0, 7000000.0, 7001000.0);
        assert!(hits.is_empty());
    }
}
