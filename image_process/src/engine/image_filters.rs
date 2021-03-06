use image::*;
use log::info;
use wasm_bindgen::prelude::wasm_bindgen;

use super::ErrorCode;

#[wasm_bindgen]
#[derive(Debug, Copy, Clone)]
pub enum FilterPixelType {
    VERTICAL,
    HORIZONTAL,
    DIAGONAL,
    CIRCLE,
}

#[wasm_bindgen]
#[derive(Debug, Copy, Clone)]
pub enum GradientDirection {
    VERTICAL,
    HORIZONTAL,
}
//Basic Color enum to be instanciate from front
#[wasm_bindgen]
#[derive(Debug, Copy, Clone)]
pub struct ColorRgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[wasm_bindgen]
impl ColorRgba {
    #[wasm_bindgen(constructor)]
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<ColorRgba> for Rgba<u8> {
    fn from(c: ColorRgba) -> Self {
        image::Rgba([c.red, c.green, c.blue, c.alpha])
    }
}

impl From<Rgba<u8>> for ColorRgba {
    fn from(rgba: Rgba<u8>) -> Self {
        ColorRgba {
            red: rgba.0[0],
            green: rgba.0[1],
            blue: rgba.0[2],
            alpha: rgba.0[3],
        }
    }
}

pub fn filter_pixel(img: &mut DynamicImage, pixelType: FilterPixelType, color: ColorRgba) -> Result<DynamicImage, ErrorCode> {
    filter_base(img, |x, y| {
        if x % 4 == 0 {
            color.into()
        } else {
            image::Rgba([255 as u8, 255, 255, 0])
        }
    })
}

/// Apply the filter function in parameter
fn filter_base<F>(img: &mut DynamicImage, func: F) -> Result<DynamicImage, ErrorCode>
where
    F: FnMut(u32, u32) -> Rgba<u8>,
{
    //Get the dimension of the image
    let (w, h) = img.dimensions();

    //Apply function pass in parameters
    let filter = ImageBuffer::from_fn(w, h, func);

    //Get back to DynamicImage
    let filter_dynamic = DynamicImage::ImageRgba8(filter);

    //And then apply the overlay
    imageops::overlay(img, &filter_dynamic, 0, 0);

    Ok(img.to_owned())
}

pub fn filter_gradient(
    img: &mut DynamicImage,
    color_from: Rgba<u8>,
    color_to: Rgba<u8>,
    gradient: GradientDirection,
) -> Result<DynamicImage, ErrorCode> {
    let mut img_buf = RgbaImage::new(img.width(), img.height());

    match gradient {
        GradientDirection::HORIZONTAL => {
            image::imageops::horizontal_gradient(&mut img_buf, &color_from, &color_to);
        }
        GradientDirection::VERTICAL => {
            image::imageops::vertical_gradient(&mut img_buf, &color_from, &color_to);
        }
    }

    imageops::overlay(img, &img_buf, 0, 0);

    Ok(img.to_owned())
}

pub fn filter_sobel(img: DynamicImage) -> Result<DynamicImage, ErrorCode> {
    let gray_image: GrayImage = img.to_luma8();
    let sobel = imageproc::gradients::sobel_gradients(&gray_image);
    Ok(DynamicImage::from(sobel))
}

pub fn filter_band_color(
    img: &mut DynamicImage,
    colors: Vec<Rgba<u8>>,
    direction: GradientDirection
) -> Result<DynamicImage, ErrorCode> {
    info!("Filter col color start");
    if colors.len() == 0 {
        info!("No color found, exit");
        //Nothing to do, exit early
        let edited_imgage = img;
        return Ok(edited_imgage.clone());
    }

    let (w, h) = img.dimensions();


    match direction {
        GradientDirection::VERTICAL => {
            let col_dimension: usize = w as usize / colors.len();
    info!(
        "Image dimension = {}*{} - {} colors to be apply - {}px by vertical band",
        w,
        h,
        colors.len(),
        col_dimension
    );

            for (i, color) in colors.into_iter().enumerate() {
                let pos_x: u32 = (i * col_dimension) as u32;
                info!("Loop col color - pos_x = {} - color = {:?}", pos_x, color);

                let filter = ImageBuffer::from_pixel(pos_x, h, color);
                let filter_dynamic = DynamicImage::ImageRgba8(filter);

                //And then apply the overlay
                imageops::overlay(img, &filter_dynamic, pos_x as i64, 0);
            }
        },
        GradientDirection::HORIZONTAL => {
            let row_dimension: usize = h as usize / colors.len();
            info!(
                "Image dimension = {}*{} - {} colors to be apply - {}px by horizontal band",
                w,
                h,
                colors.len(),
                row_dimension
            );

            for (i, color) in colors.into_iter().enumerate() {
                let pos_y: u32 = (i * row_dimension) as u32;
                info!("Loop col color - pos_x = {} - color = {:?}", pos_y, color);

                let filter = ImageBuffer::from_pixel(pos_y, h, color);
                let filter_dynamic = DynamicImage::ImageRgba8(filter);

                //And then apply the overlay
                imageops::overlay(img, &filter_dynamic, pos_y as i64, 0);
            }
        }
    }


    Ok(img.to_owned())
}
