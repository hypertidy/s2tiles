use geoconvert::Mgrs;

use crate::ifd::IfdLevel;
use crate::grid::TileGrid;

/// A Sentinel-2 MGRS tile: the 100km grid square with its UTM origin and EPSG code.
///
/// The MGRS code (e.g. "55HBU") identifies a 100km × 100km square in a UTM zone.
/// Sentinel-2 tiles have a 109800m extent (10980 pixels × 10m) to provide 4.9km
/// overlap on each side with neighbouring tiles.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MgrsTile {
    /// The MGRS code, e.g. "55HBU"
    pub code: String,
    /// UTM zone number (1-60)
    pub zone: u32,
    /// Whether the tile is in the northern hemisphere
    pub north: bool,
    /// EPSG code for the UTM zone (e.g. 32755 for zone 55 south)
    pub epsg: u32,
    /// Easting of the SW corner of the 100km grid square (metres)
    pub easting_origin: f64,
    /// Northing of the SW corner of the 100km grid square (metres)
    pub northing_origin: f64,
}

impl MgrsTile {
    /// Decode an MGRS grid square code to its UTM origin.
    ///
    /// The code should be a Sentinel-2 tile ID like "55HBU" (grid zone designator
    /// + 100km square letters). We parse it at 1m precision to get the SW corner,
    /// then the extent is 100km in both easting and northing.
    ///
    /// # Errors
    ///
    /// Returns an error string if the MGRS code cannot be parsed.
    pub fn from_code(code: &str) -> Result<Self, String> {
        // geoconvert::Mgrs::parse expects a full MGRS string.
        // For a 100km grid square like "55HBU", we need to append "0000000000"
        // (5 digits easting + 5 digits northing at 1m precision) to represent
        // the SW corner (0,0 within the square).
        let full_mgrs = format!("{}0000000000", code);
        let mgrs = Mgrs::parse_str(&full_mgrs).map_err(|e| format!("MGRS parse error: {e}"))?;
        let utm = mgrs.to_utmups();

        let zone = utm.zone() as u32;
        let north = utm.is_north();
        let epsg = if north {
            32600 + zone
        } else {
            32700 + zone
        };

        Ok(MgrsTile {
            code: code.to_uppercase(),
            zone,
            north,
            epsg,
            easting_origin: utm.easting(),
            northing_origin: utm.northing(),
        })
    }

    /// Build the GDAL-style geotransform for this MGRS tile at a given pixel size.
    ///
    /// Sentinel-2 COGs use a north-up geotransform where:
    /// - gt[0] = easting of the UL corner
    /// - gt[1] = pixel width (positive, e.g. 10.0)
    /// - gt[2] = 0 (no rotation)
    /// - gt[3] = northing of the UL corner (top = origin_northing + extent)
    /// - gt[4] = 0 (no rotation)
    /// - gt[5] = -pixel_height (negative, y decreases downward)
    ///
    /// The extent is 109800m (for 10m resolution, 10980 pixels × 10m).
    /// For other resolutions the extent is `image_height * pixel_size`.
    pub fn geotransform(&self, pixel_size: f64, image_height: u32) -> [f64; 6] {
        let extent_y = image_height as f64 * pixel_size;
        [
            self.easting_origin,                    // UL easting
            pixel_size,                             // pixel width
            0.0,                                    // rotation
            self.northing_origin + extent_y,        // UL northing
            0.0,                                    // rotation
            -pixel_size,                            // pixel height (negative)
        ]
    }

    /// Build a `TileGrid` for this MGRS tile at a given IFD level and pixel size.
    pub fn tile_grid(&self, ifd: &IfdLevel, pixel_size: f64) -> TileGrid {
        let gt = self.geotransform(pixel_size, ifd.image_height);
        TileGrid::new(self.clone(), ifd.clone(), gt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mgrs_decode_55hbu() {
        let tile = MgrsTile::from_code("55HBU").unwrap();
        assert_eq!(tile.zone, 55);
        assert!(!tile.north); // H band is southern hemisphere
        assert_eq!(tile.epsg, 32755);
        // The SW corner of 55HBU should be at easting 100000, northing ~some value
        // (exact values depend on the MGRS grid definition)
        assert!(tile.easting_origin >= 100_000.0);
        assert!(tile.easting_origin < 900_000.0);
    }

    #[test]
    fn test_mgrs_decode_32tqm() {
        let tile = MgrsTile::from_code("32TQM").unwrap();
        assert_eq!(tile.zone, 32);
        assert!(tile.north); // T band is northern hemisphere
        assert_eq!(tile.epsg, 32632);
    }

    #[test]
    fn test_geotransform() {
        let tile = MgrsTile::from_code("55HBU").unwrap();
        let gt = tile.geotransform(10.0, 10980);
        assert_eq!(gt[1], 10.0);
        assert_eq!(gt[5], -10.0);
        assert_eq!(gt[2], 0.0);
        assert_eq!(gt[4], 0.0);
        // UL northing should be origin + 109800
        let expected_ul_northing = tile.northing_origin + 109800.0;
        assert!((gt[3] - expected_ul_northing).abs() < 1e-6);
    }
}
