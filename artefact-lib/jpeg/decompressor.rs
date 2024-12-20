use std::{mem::MaybeUninit, panic::catch_unwind, path::PathBuf, rc::Rc};

use mozjpeg_sys::{
    boolean, jpeg_create_decompress, jpeg_decompress_struct, jpeg_destroy_decompress,
    jpeg_error_mgr, jpeg_mem_src, jpeg_read_coefficients, jpeg_read_header, jpeg_std_error,
    jpeg_stdio_src,
};

use crate::{
    jpeg::coefficient::Coefficient,
    utils::{boxing::unboxing, dct::idct8x8s},
};

pub struct Decompressor {
    jerr: Box<jpeg_error_mgr>,
    cinfo: Box<jpeg_decompress_struct>,
    is_source_set: bool,
    is_header_read: bool,
}

#[derive(Debug)]
pub enum JpegSource {
    File(String),
    Buffer(Vec<u8>),
}

#[derive(Debug)]
pub enum DecompressorErr {
    DerefNull(String),

    InitJerrErr,
    InitCinfoErr,

    FileNotExist,
    FileIsNotFile,

    SourceNotSet,
    HeaderNotReadYet,

    ParseHeaderErr(String),
    EmptyCoefficientArr,
    UnsupportedNumberOfComponents,
    AccessVirtualBlockArrayErr,
    NoQuantizationTable,

    Other(String),
}

impl Decompressor {
    pub fn new() -> Result<Self, DecompressorErr> {
        // init new error struct
        let mut jerr = Box::new(MaybeUninit::<jpeg_error_mgr>::uninit());
        let error = unsafe {
            jpeg_std_error(
                jerr.as_mut_ptr()
                    .as_mut()
                    .ok_or(DecompressorErr::InitJerrErr)?,
            )
        };
        let jerr = unsafe { jerr.assume_init() };

        // init new decompressor struct
        let mut cinfo = Box::new(MaybeUninit::<jpeg_decompress_struct>::uninit());
        unsafe {
            cinfo
                .as_mut_ptr()
                .as_mut()
                .ok_or(DecompressorErr::InitCinfoErr)?
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

    pub fn set_source(&mut self, source: JpegSource) -> Result<(), DecompressorErr> {
        // set jpeg source
        match source {
            JpegSource::File(path) => {
                let path_ = PathBuf::from(&path);
                if !path_.exists() {
                    return Err(DecompressorErr::FileNotExist);
                }
                if !path_.is_file() {
                    return Err(DecompressorErr::FileIsNotFile);
                }

                let mut file = catch_unwind(|| unsafe {
                    let ptr = libc::fopen(path.as_ptr() as *const i8, "rb".as_ptr() as *const i8);
                    if ptr.is_null() {
                        return Err(DecompressorErr::DerefNull("libc::open".to_string()))?;
                    }

                    Ok(Box::from_raw(ptr))
                })
                .map_err(|e| DecompressorErr::Other(format!("{e:?}")))??;

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

    pub fn read_header(&mut self) -> Result<(), DecompressorErr> {
        if !self.is_source_set {
            return Err(DecompressorErr::SourceNotSet);
        }

        if unsafe { jpeg_read_header(self.cinfo.as_mut(), true as boolean) } != 1 {
            return Err(DecompressorErr::ParseHeaderErr('get_last_err: {
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

    pub fn read_coefficients(&mut self) -> Result<Vec<Coefficient>, DecompressorErr> {
        if !self.is_header_read {
            return Err(DecompressorErr::HeaderNotReadYet);
        }

        let coef_arrays = unsafe { jpeg_read_coefficients(self.cinfo.as_mut()) };
        if coef_arrays.is_null() {
            return Err(DecompressorErr::EmptyCoefficientArr);
        }

        let num_components = self.cinfo.num_components as usize;
        if !{ 1..=3 }.contains(&num_components) {
            return Err(DecompressorErr::UnsupportedNumberOfComponents);
        }

        let mut coefs = Vec::with_capacity(num_components);
        for c in 0..num_components {
            let comp_info = unsafe {
                let ptr = self.cinfo.comp_info.add(c);
                if ptr.is_null() {
                    return Err(DecompressorErr::DerefNull("comp_info.add".to_string()));
                }

                Rc::from_raw(ptr)
            };

            let block_count = comp_info.width_in_blocks * comp_info.height_in_blocks;
            let rounded_px_count = comp_info.width_in_blocks * comp_info.height_in_blocks * 64;

            let mut coef = Coefficient {
                rounded_px_w: comp_info.width_in_blocks * 8,
                rounded_px_h: comp_info.height_in_blocks * 8,
                rounded_px_count,

                block_w: comp_info.width_in_blocks,
                block_h: comp_info.height_in_blocks,
                block_count,

                w_samp_factor: (self.cinfo.max_h_samp_factor / comp_info.h_samp_factor) as u32,
                h_samp_factor: (self.cinfo.max_v_samp_factor / comp_info.v_samp_factor) as u32,

                dct_coefs: {
                    let mut data = Vec::with_capacity(rounded_px_count as usize);
                    for y in 0..comp_info.height_in_blocks {
                        let block_arr = unsafe {
                            if self.cinfo.common.mem.is_null() {
                                return Err(DecompressorErr::DerefNull("common.mem".to_string()));
                            }

                            let virt_barray = (*self.cinfo.common.mem)
                                .access_virt_barray
                                .ok_or(DecompressorErr::AccessVirtualBlockArrayErr)?;

                            let block_arr_ptr = virt_barray(
                                &mut self.cinfo.as_mut().common,
                                *coef_arrays.add(c),
                                y,
                                1,
                                false as boolean,
                            );

                            if block_arr_ptr.is_null() {
                                return Err(DecompressorErr::DerefNull("block_arr".to_string()));
                            }

                            *block_arr_ptr
                        };

                        for x in 0..comp_info.width_in_blocks {
                            let block = unsafe {
                                let block_ptr = block_arr.add(x as usize);
                                if block_ptr.is_null() {
                                    return Err(DecompressorErr::DerefNull("block".to_string()));
                                }

                                *block_ptr
                            };
                            data.extend_from_slice(&block);
                        }
                    }
                    data
                },

                image_data: vec![0.0; rounded_px_count as usize],
                quant_table: unsafe {
                    self.cinfo.quant_tbl_ptrs[comp_info.quant_tbl_no as usize]
                        .as_ref()
                        .ok_or(DecompressorErr::NoQuantizationTable)?
                        .quantval
                },
            };

            // the 2 below steps are done in the jpeg2png.c

            // DCT coefs + quantization table -> image data
            for i in 0..(block_count as usize) {
                for j in 0..64 {
                    coef.image_data[i * 64 + j] =
                        coef.dct_coefs[i * 64 + j] as f32 * coef.quant_table[j] as f32;
                }

                idct8x8s(
                    coef.image_data[i * 64..(i + 1) * 64]
                        .as_mut()
                        .try_into()
                        .expect("Invalid coef's image data length"),
                );
            }

            // 8x8 -> 64x1
            unboxing(
                &coef.image_data.clone(),
                coef.image_data.as_mut(),
                coef.rounded_px_w,
                coef.rounded_px_h,
                coef.block_w,
                coef.block_h,
            );

            coefs.push(coef);
        }

        Ok(coefs)
    }

    pub fn width(&self) -> u32 {
        self.cinfo.image_width
    }

    pub fn height(&self) -> u32 {
        self.cinfo.image_height
    }

    pub fn num_components(&self) -> u32 {
        self.cinfo.num_components as u32
    }
}

impl Drop for Decompressor {
    fn drop(&mut self) {
        unsafe {
            jpeg_destroy_decompress(self.cinfo.as_mut());
        }
    }
}
