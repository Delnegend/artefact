/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

//! This module exports a single struct to store information about
//! JPEG image components
//!
//! The data is extracted from a SOF header.

use zune_core::log::trace;

use crate::{decoder::MAX_COMPONENTS, errors::DecodeErrors, sample_factor::SampleFactor};

/// Represents an up-sampler function, this function will be called to upsample
/// a down-sampled image
pub type UpSampler = fn(
    input: &[i16],
    in_near: &[i16],
    in_far: &[i16],
    scratch_space: &mut [i16],
    output: &mut [i16],
);

/// Component Data from start of frame
#[derive(Clone)]
pub struct Components {
    /// The type of component that has the metadata below, can be Y,Cb or Cr
    pub component_id: ComponentID,
    /// Sub-sampling ratio of this component in the x-plane
    pub vertical_samp: SampleFactor,
    /// Sub-sampling ratio of this component in the y-plane
    pub horizontal_samp: SampleFactor,
    /// DC huffman table position
    pub dc_huff_table: usize,
    /// AC huffman table position for this element.
    pub ac_huff_table: usize,
    /// Quantization table number
    pub quant_table_number: u8,
    /// Specifies quantization table to use with this component
    pub quant_table: [i32; 64],
    /// dc prediction for the component
    pub dc_pred: i32,
    /// How pixels do we need to go to get to the next line?
    pub width_stride: usize,
    /// Component ID for progressive
    pub id: u8,
    /// Whether we need to decode this image component.
    pub needed: bool,
    /// Upsample scanline
    pub raw_coeff: Vec<i16>,
    pub dct_coefs: Vec<i16>,
    /// Upsample destination, stores a scanline worth of sub sampled data
    pub upsample_dest: Vec<i16>,
    /// previous row, used to handle MCU boundaries
    pub row_up: Vec<i16>,
    /// current row, used to handle MCU boundaries again
    pub row: Vec<i16>,
    pub first_row_upsample_dest: Vec<i16>,
    pub idct_pos: usize,
    pub w2: usize,

    pub sample_ratio: SampleRatios,
    // a very annoying bug
    pub fix_an_annoying_bug: usize,

    pub horizontal_samp_factor: SampleFactor,
    pub vertical_samp_factor: SampleFactor,
    pub rounded_px_w: u16,
    pub rounded_px_h: u16,
    pub rounded_px_count: usize,
}

impl Components {
    /// Create a new instance from three bytes from the start of frame
    #[inline]
    pub fn from(a: [u8; 3], pos: u8) -> Result<Components, DecodeErrors> {
        // it's a unique identifier.
        // doesn't have to be ascending
        // see tests/inputs/huge_sof_number
        //
        // For such cases, use the position of the component
        // to determine width

        let id = match pos {
            0 => ComponentID::Y,
            1 => ComponentID::Cb,
            2 => ComponentID::Cr,
            3 => ComponentID::Q,
            _ => {
                return Err(DecodeErrors::Format(format!(
                    "Unknown component id found,{pos}, expected value between 1 and 4"
                )));
            }
        };

        let horizontal_samp = match a[1] >> 4 {
            1 => SampleFactor::One,
            2 => SampleFactor::Two,
            x => {
                return Err(DecodeErrors::Format(format!(
                    "Unknown horizontal sample found: {x}, expected either 1 or 2"
                )));
            }
        };
        let vertical_samp = match a[1] & 0x0f {
            1 => SampleFactor::One,
            2 => SampleFactor::Two,
            x => {
                return Err(DecodeErrors::Format(format!(
                    "Unknown vertical sample found: {x}, expected either 1 or 2"
                )));
            }
        };
        let quant_table_number = a[2];
        // confirm quantization number is between 0 and MAX_COMPONENTS
        if usize::from(quant_table_number) >= MAX_COMPONENTS {
            return Err(DecodeErrors::Format(format!(
                "Too large quantization number :{quant_table_number}, expected value between 0 and {MAX_COMPONENTS}"
            )));
        }

        trace!(
            "Component ID:{:?} \tHS:{} VS:{} QT:{}",
            id, horizontal_sample, vertical_sample, quantization_table_number
        );

        Ok(Components {
            component_id: id,
            vertical_samp,
            horizontal_samp,
            quant_table_number,
            first_row_upsample_dest: vec![],
            // These two will be set with sof marker
            dc_huff_table: 0,
            ac_huff_table: 0,
            quant_table: [0; 64],
            dc_pred: 0,
            // set later
            width_stride: horizontal_samp.usize(),
            id: a[0],
            needed: true,
            raw_coeff: vec![],
            dct_coefs: vec![],
            upsample_dest: vec![],
            row_up: vec![],
            row: vec![],
            idct_pos: 0,
            w2: 0,
            sample_ratio: SampleRatios::None,
            fix_an_annoying_bug: 1,
            horizontal_samp_factor: SampleFactor::One,
            vertical_samp_factor: SampleFactor::One,
            rounded_px_w: 0,
            rounded_px_h: 0,
            rounded_px_count: 0,
        })
    }
    /// Setup space for upsampling
    ///
    /// During upsample, we need a reference of the last row so that upsampling can
    /// proceed correctly,
    /// so we store the last line of every scanline and use it for the next upsampling procedure
    /// to store this, but since we don't need it for 1v1 upsampling,
    /// we only call this for routines that need upsampling
    ///
    /// # Requirements
    ///  - width stride of this element is set for the component.
    pub fn setup_upsample_scanline(&mut self) {
        self.row = vec![0; self.width_stride * self.vertical_samp.usize()];
        self.row_up = vec![0; self.width_stride * self.vertical_samp.usize()];
        self.first_row_upsample_dest =
            vec![128; self.vertical_samp.usize() * self.width_stride * self.sample_ratio.sample()];
        self.upsample_dest =
            vec![0; self.width_stride * self.sample_ratio.sample() * self.fix_an_annoying_bug * 8];
    }
}

/// Component ID's
#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum ComponentID {
    /// Luminance channel
    Y,
    /// Blue chrominance
    Cb,
    /// Red chrominance
    Cr,
    /// Q or fourth component
    Q,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum SampleRatios {
    HV,
    V,
    H,
    None,
}

impl SampleRatios {
    #[must_use]
    pub fn sample(self) -> usize {
        match self {
            SampleRatios::HV => 4,
            SampleRatios::V | SampleRatios::H => 2,
            SampleRatios::None => 1,
        }
    }
}
