use std::{mem::MaybeUninit, panic::catch_unwind, path::PathBuf, rc::Rc};

use mozjpeg_sys::{
    boolean, jpeg_create_decompress, jpeg_decompress_struct, jpeg_destroy_decompress,
    jpeg_error_mgr, jpeg_mem_src, jpeg_read_coefficients, jpeg_read_header, jpeg_std_error,
    jpeg_stdio_src,
};

#[cfg(feature = "simd")]
use wide::f32x8;
use zune_jpeg::sample_factor::SampleFactor;

#[cfg(feature = "simd")]
use crate::compute::simd::f32x8;
use crate::jpeg::{Coefficient, Jpeg, JpegSource};

#[cfg(feature = "mozjpeg")]
struct MozDecoder {
    pub cinfo: Box<jpeg_decompress_struct>,
    jerr: Box<jpeg_error_mgr>,
    is_source_set: bool,
    is_header_read: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum MozDecoderErr {
    #[error("Trying to deref null pointer: {0}")]
    DerefNull(String),
    #[error("Failed to init `jpeg_error_mgr`")]
    InitJerrErr,
    #[error("Failed to init `jpeg_decompress_struct`")]
    InitCinfoErr,
    #[error("Input file does not exist")]
    FileNotExist,
    #[error("Input file is not a file")]
    FileIsNotFile,
    #[error("Input source is not set")]
    SourceNotSet,
    #[error("Header is not read yet")]
    HeaderNotReadYet,
    #[error("Failed to parse header: {0}")]
    ParseHeaderErr(String),
    #[error("THe coefficient array is empty")]
    EmptyCoefficientArr,
    #[error("Unsupported number of channel")]
    UnsupportedNumberOfChannel,
    #[error("Failed to access virtual block array")]
    AccessVirtualBlockArrayErr,
    #[error("No quantization table")]
    NoQuantizationTable,
    #[error("Invalid quantization table")]
    InvalidQuantTable,
    #[error("Invalid horizontal sampling factor")]
    InvalidHorizontalSampFactor,
    #[error("Invalid vertical sampling factor")]
    InvalidVerticalSampFactor,
    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(feature = "mozjpeg")]
impl Jpeg {
    pub fn from(jpeg_source: JpegSource) -> Result<Jpeg, String> {
        let mut decoder = MozDecoder::new().map_err(|e| e.to_string())?;
        decoder.set_source(jpeg_source).map_err(|e| e.to_string())?;
        decoder.read_header().map_err(|e| e.to_string())?;
        Ok(Self {
            chan_count: decoder.cinfo.num_components as u32,
            real_px_w: decoder.cinfo.image_width,
            real_px_h: decoder.cinfo.image_height,
            coefs: decoder.read_coefficients().map_err(|e| e.to_string())?,
        })
    }
}
impl MozDecoder {
    fn new() -> Result<Self, MozDecoderErr> {
        // init new error struct
        let mut jerr = Box::new(MaybeUninit::<jpeg_error_mgr>::uninit());
        let error = unsafe {
            jpeg_std_error(
                jerr.as_mut_ptr()
                    .as_mut()
                    .ok_or(MozDecoderErr::InitJerrErr)?,
            )
        };
        let jerr = unsafe { jerr.assume_init() };
        // init new decompressor struct
        let mut cinfo = Box::new(MaybeUninit::<jpeg_decompress_struct>::uninit());
        unsafe {
            cinfo
                .as_mut_ptr()
                .as_mut()
                .ok_or(MozDecoderErr::InitCinfoErr)?
                .common
                .err = error;
        };
        unsafe { jpeg_create_decompress(cinfo.as_mut_ptr()) };
        let cinfo = unsafe { cinfo.assume_init() };
        Ok(Self {
            cinfo,
            jerr,
            is_source_set: false,
            is_header_read: false,
        })
    }
    fn set_source(&mut self, source: JpegSource) -> Result<(), MozDecoderErr> {
        // set jpeg source
        match source {
            JpegSource::File(path) => {
                let path_ = PathBuf::from(&path);
                if !path_.exists() {
                    return Err(MozDecoderErr::FileNotExist);
                }
                if !path_.is_file() {
                    return Err(MozDecoderErr::FileIsNotFile);
                }
                let mut file = catch_unwind(|| unsafe {
                    let ptr = libc::fopen(path.as_ptr() as *const i8, "rb".as_ptr() as *const i8);
                    if ptr.is_null() {
                        return Err(MozDecoderErr::DerefNull("libc::open".to_string()))?;
                    }
                    Ok(Box::from_raw(ptr))
                })
                .map_err(|e| MozDecoderErr::Other(format!("{e:?}")))??;
                unsafe {
                    jpeg_stdio_src(self.cinfo.as_mut(), file.as_mut());
                }
            }
            JpegSource::Buffer(buffer) => unsafe {
                jpeg_mem_src(
                    self.cinfo.as_mut(),
                    buffer.as_ptr(),
                    buffer.len() as core::ffi::c_ulong,
                );
            },
        }
        self.is_source_set = true;
        Ok(())
    }
    fn read_header(&mut self) -> Result<(), MozDecoderErr> {
        if !self.is_source_set {
            return Err(MozDecoderErr::SourceNotSet);
        }
        if unsafe { jpeg_read_header(self.cinfo.as_mut(), true as boolean) } != 1 {
            return Err(MozDecoderErr::ParseHeaderErr('get_last_err: {
                let buffer = [0u8; 80];
                if let Some(format_fn) = self.jerr.format_message {
                    unsafe {
                        format_fn(&mut self.cinfo.common, &buffer);
                        break 'get_last_err std::ffi::CStr::from_ptr(buffer.as_ptr() as *const i8)
                            .to_string_lossy()
                            .into_owned();
                    }
                }
                "Unknown JPEG error".to_string()
            }))?;
        }
        self.is_header_read = true;
        Ok(())
    }
    fn read_coefficients(&mut self) -> Result<Vec<Coefficient>, MozDecoderErr> {
        if !self.is_header_read {
            return Err(MozDecoderErr::HeaderNotReadYet);
        }
        let coef_arrays = unsafe { jpeg_read_coefficients(self.cinfo.as_mut()) };
        if coef_arrays.is_null() {
            return Err(MozDecoderErr::EmptyCoefficientArr);
        }
        let num_components = self.cinfo.num_components as usize;
        if num_components != 1 && num_components != 3 {
            return Err(MozDecoderErr::UnsupportedNumberOfChannel);
        }
        let mut coefs = Vec::with_capacity(num_components);
        for c in 0..num_components {
            let comp_info = unsafe {
                let ptr = self.cinfo.comp_info.add(c);
                if ptr.is_null() {
                    return Err(MozDecoderErr::DerefNull("comp_info.add".to_string()));
                }
                Rc::from_raw(ptr)
            };
            let block_count = comp_info.width_in_blocks * comp_info.height_in_blocks;
            let rounded_px_w = comp_info.width_in_blocks * 8;
            let rounded_px_h = comp_info.height_in_blocks * 8;
            let rounded_px_count = rounded_px_w * rounded_px_h;

            let dct_coefs = {
                let mut data = Vec::with_capacity(rounded_px_count as usize);

                for y in 0..comp_info.height_in_blocks {
                    let block_arr = unsafe {
                        if self.cinfo.common.mem.is_null() {
                            return Err(MozDecoderErr::DerefNull("common.mem".to_string()));
                        }
                        let virt_barray = (*self.cinfo.common.mem)
                            .access_virt_barray
                            .ok_or(MozDecoderErr::AccessVirtualBlockArrayErr)?;
                        let block_arr_ptr = virt_barray(
                            &mut self.cinfo.as_mut().common,
                            *coef_arrays.add(c),
                            y,
                            1,
                            false as boolean,
                        );
                        if block_arr_ptr.is_null() {
                            return Err(MozDecoderErr::DerefNull("block_arr".to_string()));
                        }
                        *block_arr_ptr
                    };
                    for x in 0..comp_info.width_in_blocks {
                        let block = unsafe {
                            let block_ptr = block_arr.add(x as usize);
                            if block_ptr.is_null() {
                                return Err(MozDecoderErr::DerefNull("block".to_string()));
                            }
                            *block_ptr
                        };
                        data.extend_from_slice(&block);
                    }
                }
                data.iter().map(|x| *x as f32).collect::<Vec<_>>()
            };

            let quant_table = unsafe {
                self.cinfo.quant_tbl_ptrs[comp_info.quant_tbl_no as usize]
                    .as_ref()
                    .ok_or(MozDecoderErr::NoQuantizationTable)?
                    .quantval
                    .iter()
                    .map(|x| *x as f32)
                    .collect::<Vec<_>>()
            };

            let mut coef = Coefficient {
                rounded_px_w,
                rounded_px_h,
                rounded_px_count,
                block_w: comp_info.width_in_blocks,
                block_h: comp_info.height_in_blocks,
                block_count,
                horizontal_samp_factor: match (self.cinfo.max_h_samp_factor
                    / comp_info.h_samp_factor) as u32
                {
                    1 => SampleFactor::One,
                    2 => SampleFactor::Two,
                    _ => return Err(MozDecoderErr::InvalidHorizontalSampFactor),
                },
                vertical_samp_factor: match (self.cinfo.max_v_samp_factor / comp_info.v_samp_factor)
                    as u32
                {
                    1 => SampleFactor::One,
                    2 => SampleFactor::Two,
                    _ => return Err(MozDecoderErr::InvalidVerticalSampFactor),
                },
                #[cfg(not(feature = "simd"))]
                dct_coefs,
                #[cfg(feature = "simd")]
                dct_coefs: dct_coefs.chunks_exact(8).map(f32x8::from).collect(),
                image_data: vec![0.0; rounded_px_count as usize],
                #[cfg(not(feature = "simd"))]
                quant_table,
                #[cfg(feature = "simd")]
                quant_table: quant_table
                    .chunks_exact(8)
                    .map(f32x8::from)
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| MozDecoderErr::InvalidQuantTable)?,
                #[cfg(feature = "simd")]
                quant_table_squared: [f32x8::splat(0.0); 8],

                #[cfg(feature = "simd")]
                dequant_dct_coefs_min: vec![f32x8!(); rounded_px_count as usize / 8],
                #[cfg(feature = "simd")]
                dequant_dct_coefs_max: vec![f32x8!(); rounded_px_count as usize / 8],
            };
            coef.post_process();
            coefs.push(coef);
        }
        Ok(coefs)
    }
}

impl Drop for MozDecoder {
    fn drop(&mut self) {
        unsafe {
            jpeg_destroy_decompress(self.cinfo.as_mut());
        }
    }
}
