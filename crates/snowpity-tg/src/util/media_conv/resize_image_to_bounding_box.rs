use crate::prelude::*;
use crate::{fatal, Result};
use bytes::Bytes;
use fast_image_resize as fr;
use image::ColorType;
use metrics_bat::prelude::*;
use std::sync::OnceLock;

metrics_bat::histograms! {
    /// Number of seconds it took to resize the image to bounding box
    resize_image_to_bounding_box_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub async fn resize_image_to_bounding_box(bytes: Bytes, box_side: u32) -> Result<Bytes> {
    crate::util::tokio::spawn_blocking(move || resize_image_to_bounding_box_sync(bytes, box_side))
        .record_duration(resize_image_to_bounding_box_duration_seconds, vec![])
        .with_duration_log("Resize image to bounding box")
        .await
}

pub fn resize_image_to_bounding_box_sync(bytes: Bytes, box_side: u32) -> Result<Bytes> {
    let format =
        image::guess_format(&bytes).fatal_ctx(|| "Couldn't guess the format of the image")?;

    let src = image::load_from_memory_with_format(&bytes, format)
        .fatal_ctx(|| "Failed to load the image from the memory buffer")?;

    drop(bytes);

    let color = src.color();

    // Scale the image down to the bounding box
    let (dest_width, dest_height) = {
        let src_width = f64::from(src.width());
        let src_height = f64::from(src.height());
        let box_side = f64::from(box_side);
        let factor = (box_side / src_width).min(box_side / src_height);
        (
            (src_width * factor).floor() as u32,
            (src_height * factor).floor() as u32,
        )
    };

    let mut src = get_image_with_linear_colorspace(src)?;

    let mut dest = fr::images::Image::new(dest_width, dest_height, src.pixel_type());

    let mul_div = fr::MulDiv::default();

    let mut resizer = fr::Resizer::new();

    if color.has_alpha() {
        mul_div
            .multiply_alpha_inplace(&mut src)
            .fatal_ctx(|| "Failed to multiply color channels by alpha")?;
    }

    let options = fr::ResizeOptions::new()
        // Lanczos3 is the best algorithm for downsampling
        // https://en.wikipedia.org/wiki/Lanczos_resampling
        // Also apply antialiasing by using super sampling
        .resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3));

    resizer
        .resize(&src, &mut dest, &options)
        .fatal_ctx(|| "Failed to resize image")?;

    drop(src);

    if color.has_alpha() {
        mul_div
            .divide_alpha_inplace(&mut dest)
            .fatal_ctx(|| "Failed to divide color channels by alpha")?;
    }

    map_image_colorspace(
        &mut dest,
        color,
        fr::PixelComponentMapper::backward_map_inplace,
    )?;

    let mut output = Vec::new();

    image::write_buffer_with_format(
        &mut std::io::Cursor::new(&mut output),
        dest.buffer(),
        dest.width(),
        dest.height(),
        color,
        format,
    )
    .fatal_ctx(|| "Failed to write the image to the buffer")?;

    Ok(output.into())
}

// This code was based on https://github.com/Cykooz/fast_image_resize/blob/24edd65eef20596e51c23f84db79474a900e2d18/resizer/src/main.rs#L105-L225
fn get_image_with_linear_colorspace(
    image: image::DynamicImage,
) -> Result<fr::images::Image<'static>> {
    let width = image.width();

    let height = image.height();

    let color = image.color();

    // Explicitly spelling the match arms to make it easier to see what new formats
    // should be handled
    #[allow(clippy::wildcard_in_or_patterns)]
    let (pixel_type, buffer) = match image {
        image::DynamicImage::ImageLuma8(image) => (fr::PixelType::U8, image.into_raw()),
        image::DynamicImage::ImageLumaA8(image) => (fr::PixelType::U8x2, image.into_raw()),
        image::DynamicImage::ImageRgb8(image) => (fr::PixelType::U8x3, image.into_raw()),
        image::DynamicImage::ImageRgba8(image) => (fr::PixelType::U8x4, image.into_raw()),
        image::DynamicImage::ImageLuma16(image) => (fr::PixelType::U16, u16_to_u8(image)),
        image::DynamicImage::ImageLumaA16(image) => (fr::PixelType::U16x2, u16_to_u8(image)),
        image::DynamicImage::ImageRgb16(image) => (fr::PixelType::U16x3, u16_to_u8(image)),
        image::DynamicImage::ImageRgba16(image) => (fr::PixelType::U16x4, u16_to_u8(image)),
        image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_) | _ => {
            return Err(fatal!("Unsupported pixel's format of image: {color:?}"))
        }
    };

    let mut image = fr::images::Image::from_vec_u8(width, height, buffer, pixel_type)
        .fatal_ctx(|| "Failed to create source image pixels container")?;

    // Convert the source image from non-linear colorspace into linear
    map_image_colorspace(
        &mut image,
        color,
        fr::PixelComponentMapper::forward_map_inplace,
    )?;

    Ok(image)
}

fn map_image_colorspace<'a>(
    image: &mut fr::images::Image<'a>,
    color_type: ColorType,
    direction_fn: fn(
        &fr::PixelComponentMapper,
        image: &mut fr::images::Image<'a>,
    ) -> Result<(), fr::MappingError>,
) -> Result<()> {
    direction_fn(mapper_for_color_type(color_type), image).fatal_ctx(|| {
        format!(
            "Failed to map the image from non-linear \
                colorspace to linear. color type: {color_type:?}"
        )
    })
}

// FIXME: query the colorspace from the image metadata
fn mapper_for_color_type(color_type: ColorType) -> &'static fr::PixelComponentMapper {
    color_type
        .has_color()
        .then(srgb_to_rgb_mapper)
        .unwrap_or_else(gamma22_to_linear_mapper)
}

fn srgb_to_rgb_mapper() -> &'static fr::PixelComponentMapper {
    static GLOBAL: OnceLock<fr::PixelComponentMapper> = OnceLock::new();
    GLOBAL.get_or_init(fr::create_srgb_mapper)
}

fn gamma22_to_linear_mapper() -> &'static fr::PixelComponentMapper {
    static GLOBAL: OnceLock<fr::PixelComponentMapper> = OnceLock::new();
    GLOBAL.get_or_init(fr::create_gamma_22_mapper)
}

fn u16_to_u8<P: image::Pixel<Subpixel = u16>>(bytes: image::ImageBuffer<P, Vec<u16>>) -> Vec<u8> {
    bytes
        .into_raw()
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect()
}
