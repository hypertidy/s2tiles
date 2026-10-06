/// Description of one IFD level in a COG pyramid.
///
/// These dimensions are **not hardcoded** — they come from a single `tiff_refs`
/// probe against a representative COG. The overview dimensions can vary across
/// processing baselines and GDAL versions.
///
/// # Sentinel-2 typical values (10m bands)
///
/// | IFD | image_width | image_height | tile_width | tile_height | n_tile_cols | n_tile_rows |
/// |-----|-------------|--------------|------------|-------------|-------------|-------------|
/// | 0   | 10980       | 10980        | 512        | 512         | 22          | 22          |
/// | 1   | 5490        | 5490         | 512        | 512         | 11          | 11          |
/// | 2   | 2745        | 2745         | 512        | 512         | 6           | 6           |
/// | 3   | 1373        | 1373         | 512        | 512         | 3           | 3           |
/// | 4   | 687         | 687          | 512        | 512         | 2           | 2           |
///
/// Note: the exact values for IFDs 2-4 depend on rounding behaviour.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IfdLevel {
    /// IFD index (0 = native resolution, 1 = first overview, ...)
    pub ifd: u32,
    /// Image width in pixels at this IFD level
    pub image_width: u32,
    /// Image height in pixels at this IFD level
    pub image_height: u32,
    /// COG internal tile width (typically 512)
    pub tile_width: u32,
    /// COG internal tile height (typically 512)
    pub tile_height: u32,
    /// Number of tile columns: ceil(image_width / tile_width)
    pub n_tile_cols: u32,
    /// Number of tile rows: ceil(image_height / tile_height)
    pub n_tile_rows: u32,
}

impl IfdLevel {
    /// Create an IFD level from the image and tile dimensions.
    ///
    /// `n_tile_cols` and `n_tile_rows` are computed automatically.
    pub fn new(
        ifd: u32,
        image_width: u32,
        image_height: u32,
        tile_width: u32,
        tile_height: u32,
    ) -> Self {
        let n_tile_cols = (image_width + tile_width - 1) / tile_width;
        let n_tile_rows = (image_height + tile_height - 1) / tile_height;
        IfdLevel {
            ifd,
            image_width,
            image_height,
            tile_width,
            tile_height,
            n_tile_cols,
            n_tile_rows,
        }
    }

    /// Total number of tiles at this IFD level.
    pub fn n_tiles(&self) -> u32 {
        self.n_tile_cols * self.n_tile_rows
    }

    /// Effective pixel size at this IFD level, given the native pixel size
    /// and native image dimension (along one axis).
    ///
    /// For overview level i, pixel_size ≈ native_pixel_size * (native_dim / overview_dim).
    /// This isn't exactly a power of 2 because of rounding.
    pub fn effective_pixel_size(&self, native_pixel_size: f64, native_dim: u32) -> f64 {
        native_pixel_size * (native_dim as f64 / self.image_width as f64)
    }
}

/// Build a standard Sentinel-2 IFD pyramid from the native dimensions
/// and a set of overview dimensions.
///
/// `overview_dims` is a slice of `(width, height)` pairs for each overview level,
/// ordered from largest (first overview) to smallest.
///
/// # Example
///
/// ```rust
/// use s2tiles_core::IfdLevel;
/// use s2tiles_core::ifd::build_pyramid;
///
/// // Native + 4 overviews, all with 512×512 tiles
/// let levels = build_pyramid(
///     10980, 10980, 512, 512,
///     &[(5490, 5490), (2745, 2745), (1373, 1373), (687, 687)],
/// );
/// assert_eq!(levels.len(), 5);
/// assert_eq!(levels[0].n_tile_cols, 22);
/// ```
pub fn build_pyramid(
    native_width: u32,
    native_height: u32,
    tile_width: u32,
    tile_height: u32,
    overview_dims: &[(u32, u32)],
) -> Vec<IfdLevel> {
    let mut levels = Vec::with_capacity(1 + overview_dims.len());
    levels.push(IfdLevel::new(0, native_width, native_height, tile_width, tile_height));
    for (i, &(w, h)) in overview_dims.iter().enumerate() {
        levels.push(IfdLevel::new((i + 1) as u32, w, h, tile_width, tile_height));
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_counts_native() {
        let ifd = IfdLevel::new(0, 10980, 10980, 512, 512);
        // ceil(10980/512) = 22 (21.4453...)
        assert_eq!(ifd.n_tile_cols, 22);
        assert_eq!(ifd.n_tile_rows, 22);
        assert_eq!(ifd.n_tiles(), 484);
    }

    #[test]
    fn test_tile_counts_overview() {
        let ifd = IfdLevel::new(1, 5490, 5490, 512, 512);
        // ceil(5490/512) = 11 (10.72...)
        assert_eq!(ifd.n_tile_cols, 11);
        assert_eq!(ifd.n_tile_rows, 11);
    }

    #[test]
    fn test_build_pyramid() {
        let levels = build_pyramid(
            10980, 10980, 512, 512,
            &[(5490, 5490), (2745, 2745), (1373, 1373), (687, 687)],
        );
        assert_eq!(levels.len(), 5);
        assert_eq!(levels[0].ifd, 0);
        assert_eq!(levels[0].n_tile_cols, 22);
        assert_eq!(levels[4].ifd, 4);
        assert_eq!(levels[4].n_tile_cols, 2); // ceil(687/512)
    }
}
