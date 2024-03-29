use super::{
    image_filters::{self, ColorRgba, FilterPixelType, GradientDirection},
    ErrorCode, ImageProcessingResult, image_processing_result::ImageDimension,
};
use chrono::Local;
use image::{Rgba, imageops};
use image::{DynamicImage};
use imageproc::drawing::Canvas;
use log::*;
use std::{fmt::Display, io::Cursor};
use wasm_bindgen::prelude::*;

//Image encoding pass in parameters
pub trait InputType {
    fn to_byte(&self) -> Result<Vec<u8>, ErrorCode>;
}
//InputType base64
impl InputType for String {
    fn to_byte(&self) -> Result<Vec<u8>, ErrorCode> {
        match base64::decode(ImageProcess::parse_base64_input_if_needed(&self)) {
            Ok(img_bytes) => {
                trace!("Convert Vec<u8> byte from base64 image");
                return Ok(img_bytes);
            }
            Err(e) => {
                error!("Failed base64::decode() : {}", e);
                return Err(ErrorCode::UnableToDecode);
            }
        }
    }
}

//InputType bytes
impl InputType for Vec<u8> {
    fn to_byte(&self) -> Result<Vec<u8>, ErrorCode> {
        Ok(self.to_vec())
    }
}

impl InputType for ImageProcessingResult {
    fn to_byte(&self) -> Result<Vec<u8>, ErrorCode> {
        Ok(self.to_byte())
    }
}

#[derive(Debug)]
pub struct ImageProcess {
    pub input: Vec<u8>,
}

//Can be instanciate from Typescript
#[wasm_bindgen]
#[derive(Debug)]
pub struct ImageParameters {
    //add colorops::invert, colorops::BiLevel, colorops::index_colors, https://docs.rs/image/latest/image/imageops/enum.FilterType.html
    pub brighten: Option<i32>,
    pub hue: Option<i32>,
    pub blur: Option<f32>,
    pub constrast: Option<f32>,
    pub grayscale: Option<bool>,
    pub invert: Option<bool>,
}

#[wasm_bindgen]
impl ImageParameters {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ImageParameters {
        ImageParameters::default()
    }
}

impl ImageParameters {
    pub fn default() -> Self {
        Self {
            brighten: Some(0),
            blur: Some(0.0),
            hue: Some(0),
            constrast: Some(0.0),
            grayscale: Some(false),
            invert: Some(false),
        }
    }

    /// Perform basic filter on image
    pub fn apply_filter(&self, mut img: DynamicImage) -> DynamicImage {
        if let Some(brighten) = self.brighten {
            img = img.brighten(brighten);
            trace!("Brighten filter applied : {}", brighten);
        }

        if let Some(blur) = self.blur {
            img = img.blur(blur);
            trace!("Blur filter applied : {}", blur);
        }

        if let Some(hue) = self.hue {
            img = img.huerotate(hue);
            trace!("Huerotate filter applied : {}", hue);
        }

        if let Some(b) = self.grayscale {
            if b {
                img = img.grayscale();
                trace!("Grayscale filter applied");
            }
        }

        if let Some(constrast) = self.constrast {
            img = img.adjust_contrast(constrast);
            trace!("Constrast filter applied : {}", constrast);
        }

        if self.invert.unwrap_or(false) {
            img.invert();
            trace!("Invert filter applied");
        }

        img
    }
}

impl ImageProcess {
    pub fn new<T>(input: T) -> Result<ImageProcess, ErrorCode>
    where
        T: InputType,
    {
        trace!("New ImageProcess instance");
        Ok(ImageProcess {
            input: input.to_byte()?,
        })
    }

    ///Replace the "data:image/jpeg;base64," in the string
    pub fn parse_base64_input_if_needed(base64_input: &String) -> String {
        str::replace(base64_input.as_str(), "data:image/png;base64,", "")
    }

    /// Create a Dynamic image from bytes
    fn get_dynamic_image(&self) -> Result<DynamicImage, ErrorCode> {
        trace!("Try to create Dynamic image from byte");
        match image::load_from_memory(&self.input.as_slice()) {
            Ok(dynamic_image) => {
                trace!("Dynamic image instance created");
                Ok(dynamic_image)
            }
            Err(e) => {
                error!("Unable to create dynamic image from byte : {}", e);
                Err(ErrorCode::UnableToDecode)
            }
        }
    }

    pub fn as_byte(&self) -> Vec<u8> {
        self.input.clone()
    }

    pub fn to_base64(&self) -> String {
        base64::encode(&self.input)
    }

    pub fn get_image_dimension(&self) -> Result<ImageDimension, ErrorCode> {
        let d = self.get_dynamic_image()?.dimensions();
        Ok(ImageDimension::new(d.0, d.1))
    }

    /// Convert Dynamic image to bytes
    fn dynamic_image_to_byte(img: &DynamicImage) -> Vec<u8> {
        trace!("Convert image to bytes");
        let mut edited_image_bytes = Vec::new();
        img.write_to(
            &mut Cursor::new(&mut edited_image_bytes),
            image::ImageOutputFormat::Png,
        )
        .unwrap();

        edited_image_bytes
    }

    /// Save the image on the specific location
    pub fn save_image(img: &DynamicImage, path: &str) -> Option<ErrorCode> {
        let local_date = Local::now();
        let date_string = local_date.format("%Y-%m-%d_%H-%M-%S").to_string();
        let full_file_path = format!("{}image_save_{}.png", &path, &date_string);

        println!("Full path to save : {}", &full_file_path);

        if let Err(_) = img.save(full_file_path) {
            return Some(ErrorCode::UnableToSave);
        }
        None
    }

    pub fn compute_parameters(
        &self,
        params: ImageParameters,
    ) -> Result<ImageProcessingResult, ErrorCode> {
        let mut img = self.get_dynamic_image()?;

        img = params.apply_filter(img.clone());

        Ok(ImageProcessingResult::new(
            ImageProcess::dynamic_image_to_byte(&img),
        ))
    }

    pub fn compute_filter_sobel(&self) -> Result<ImageProcessingResult, ErrorCode> {
        self.compute_filters(|| {
            image_filters::filter_sobel(
                self.get_dynamic_image()?,
            )
        })
    }

    pub fn compute_filter_band_color(
        &self,
        colors: Vec<ColorRgba>,
        direction: GradientDirection
    ) -> Result<ImageProcessingResult, ErrorCode> {
        self.compute_filters(|| {
            image_filters::filter_band_color(
                &mut self.get_dynamic_image()?,
                colors.iter().map(|c| Rgba::<u8>::from(*c)).collect(),
                direction
            )
        })
    }

    pub fn compute_filter_gradient(
        &self,
        start: ColorRgba,
        to: ColorRgba,
        gradient: GradientDirection,
    ) -> Result<ImageProcessingResult, ErrorCode> {
        self.compute_filters(|| {
            image_filters::filter_gradient(
                &mut self.get_dynamic_image()?,
                start.into(),
                to.into(),
                gradient,
            )
        })
    }

    pub fn compute_filter_pixel(
        &self,
        pixel_filter: FilterPixelType,
        color: ColorRgba,
    ) -> Result<ImageProcessingResult, ErrorCode> {
        self.compute_filters(|| {
            image_filters::filter_pixel(&mut self.get_dynamic_image()?, pixel_filter, color)
        })
    }

    /// Perform the filter function
    fn compute_filters<F>(&self, func: F) -> Result<ImageProcessingResult, ErrorCode>
    where
        F: Fn() -> Result<DynamicImage, ErrorCode>,
    {
        Ok(ImageProcessingResult::new(
            ImageProcess::dynamic_image_to_byte(&func()?),
        ))
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<ImageProcessingResult, ErrorCode> {
        Ok(ImageProcessingResult::new(
            ImageProcess::dynamic_image_to_byte(&self.get_dynamic_image()?.resize(
                width,
                height,
                imageops::FilterType::Lanczos3,
            )),
        ))
    }

    /// Return the bytes size of the base64 image
    /// Formula : x = (n * (3/4)) - y
    ///     1. x is the size of a file in bytes
    ///     2. n is the length of the Base64 String
    ///     3. y will be 2 if Base64 ends with '==' and 1 if Base64 ends with '='.
    pub fn get_image_weight_byte(base64_input: String) -> Result<usize, ErrorCode> {
        let clean_base64 = ImageProcess::parse_base64_input_if_needed(&base64_input);
        let length = clean_base64.len();
        let output_padding = if clean_base64.ends_with("==") { 1 } else if clean_base64.ends_with("=") { 2 } else { 0 };

        trace!("base64 image [{}...{}]", &clean_base64[0..20], &clean_base64[(clean_base64.len() - 20)..]);
        trace!("base64 length = {}", length);
        trace!("output_padding = {}", output_padding);

        if length == 0 || clean_base64.is_empty() {
            return Err(ErrorCode::ImageEmpty);
        }

        let size_in_byte = (length as f64 * (3 as f64 / 4 as f64)) as usize - output_padding;
        trace!("\nImage size : {} bytes", size_in_byte);
        Ok(size_in_byte)
    }

    /// We try to have an image < 200 kbytes
    pub fn calc_best_size_ratio(base64_input: String, target_size: usize) -> Result<ImageProcessingResult, ErrorCode> {

        let mut img = ImageProcess::new(base64_input)?;

        let mut image_size = ImageProcess::get_image_weight_byte(img.to_base64())?;
        info!("Image_size = {} (target size = {})", image_size, target_size);

        while image_size > target_size {
            let (width, height) = img.get_dynamic_image()?.dimensions();
            let (new_width, new_height) = ((width * 90 / 100), (height * 90 / 100));
            // info!("[Calc best size ratio] Image size = {} | Image dimension = {}x{} | New image dimension = {}x{}", image_size, width, height, new_width, new_height);

            let result = img.resize(new_width, new_height)?;

            img = ImageProcess::new(result)?;

            image_size = ImageProcess::get_image_weight_byte(img.to_base64())?;
        }
        Ok(ImageProcessingResult::new(
            ImageProcess::dynamic_image_to_byte(&img.get_dynamic_image()?),
        ))
    }
}

impl Display for ImageProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base64::encode(&self.input))
    }
}
