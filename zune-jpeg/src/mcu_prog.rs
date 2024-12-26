/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

//!Routines for progressive decoding
/*
This file is needlessly complicated,

It is that way to ensure we don't burn memory anyhow

Memory is a scarce resource in some environments, I would like this to be viable
in such environments

Half of the complexity comes from the jpeg spec, because progressive decoding,
is one hell of a ride.

*/
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::{format, vec};

use zune_core::bytestream::{ZByteReaderTrait, ZReader};
use zune_core::colorspace::ColorSpace;
use zune_core::log::{error, warn};

use crate::bitstream::BitStream;
use crate::components::{ComponentID, SampleRatios};
use crate::decoder::{JpegDecoder, MAX_COMPONENTS};
use crate::errors::DecodeErrors;
use crate::errors::DecodeErrors::Format;
use crate::headers::{parse_huffman, parse_sos};
use crate::marker::Marker;
use crate::sample_factor::SampleFactor;

impl<T: ZByteReaderTrait> JpegDecoder<T> {
    /// Decode a progressive image
    ///
    /// This routine decodes a progressive image, stopping if it finds any error.
    #[allow(
        clippy::needless_range_loop,
        clippy::cast_sign_loss,
        clippy::redundant_else,
        clippy::too_many_lines
    )]
    #[inline(never)]
    pub(crate) fn decode_mcu_ycbcr_progressive(
        &mut self,
        dct_coefs: &mut [Vec<i16>; MAX_COMPONENTS],
    ) -> Result<(), DecodeErrors> {
        let mut mcu_height;

        // memory location for decoded pixels for components
        // Delnegend: no this is actually the pre-dequantized DCT coefficients
        let block = dct_coefs;

        let mut mcu_width;

        let mut seen_scans = 1;

        if self.input_colorspace == ColorSpace::Luma && self.is_interleaved {
            warn!("Grayscale image with down-sampled component, resetting component details");
            self.reset_params();
        }

        if self.is_interleaved {
            // this helps us catch component errors.
            self.set_upsampling()?;
        }
        if self.is_interleaved {
            mcu_width = self.min_mcu_w;
            mcu_height = self.min_mcu_h;
        } else {
            mcu_width = (self.info.width as usize + 7) / 8;
            mcu_height = (self.info.height as usize + 7) / 8;
        }
        if self.is_interleaved
            && self.input_colorspace.num_components() > 1
            && self.options.jpeg_get_out_colorspace().num_components() == 1
            && (self.sub_sample_ratio == SampleRatios::V
                || self.sub_sample_ratio == SampleRatios::HV)
        {
            // For a specific set of images, e.g interleaved,
            // when converting from YcbCr to grayscale, we need to
            // take into account mcu height since the MCU decoding needs to take
            // it into account for padding purposes and the post processor
            // parses two rows per mcu width.
            //
            // set coeff to be 2 to ensure that we increment two rows
            // for every mcu processed also
            mcu_height *= self.max_vertical_samp as usize;
            mcu_height /= self.max_horizontal_samp as usize;
            self.coeff = 2;
        }

        mcu_width *= 64;

        if self.input_colorspace.num_components() > self.components.len() {
            let msg = format!(
                " Expected {} number of components but found {}",
                self.input_colorspace.num_components(),
                self.components.len()
            );
            return Err(DecodeErrors::Format(msg));
        }
        for i in 0..self.input_colorspace.num_components() {
            let comp = &self.components[i];
            let len =
                mcu_width * comp.vertical_samp.usize() * comp.horizontal_samp.usize() * mcu_height;

            block[i] = vec![0; len];
        }

        let mut stream = BitStream::new_progressive(
            self.succ_high,
            self.succ_low,
            self.spec_start,
            self.spec_end,
        );

        // there are multiple scans in the stream, this should resolve the first scan
        self.parse_entropy_coded_data(&mut stream, block)?;

        // extract marker
        let mut marker = stream
            .marker
            .take()
            .ok_or(DecodeErrors::FormatStatic("Marker missing where expected"))?;

        // if marker is EOI, we are done, otherwise continue scanning.
        //
        // In case we have a premature image, we print a warning or return
        // an error, depending on the strictness of the decoder, so there
        // is that logic to handle too
        'eoi: while marker != Marker::EOI {
            match marker {
                Marker::DHT => {
                    parse_huffman(self)?;
                }
                Marker::SOS => {
                    parse_sos(self)?;

                    stream.update_progressive_params(
                        self.succ_high,
                        self.succ_low,
                        self.spec_start,
                        self.spec_end,
                    );

                    // after every SOS, marker, parse data for that scan.
                    self.parse_entropy_coded_data(&mut stream, block)?;
                    // extract marker, might either indicate end of image or we continue
                    // scanning(hence the continue statement to determine).
                    match get_marker(&mut self.stream, &mut stream) {
                        Ok(marker_n) => {
                            marker = marker_n;
                            seen_scans += 1;
                            if seen_scans > self.options.jpeg_get_max_scans() {
                                return Err(DecodeErrors::Format(format!(
                                    "Too many scans, exceeded limit of {}",
                                    self.options.jpeg_get_max_scans()
                                )));
                            }

                            stream.reset();
                            continue 'eoi;
                        }
                        Err(msg) => {
                            if self.options.strict_mode() {
                                return Err(msg);
                            }
                            error!("{:?}", msg);
                            break 'eoi;
                        }
                    }
                }
                _ => {
                    break 'eoi;
                }
            }

            match get_marker(&mut self.stream, &mut stream) {
                Ok(marker_n) => {
                    marker = marker_n;
                }
                Err(e) => {
                    if self.options.strict_mode() {
                        return Err(e);
                    }
                    error!("{}", e);
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines, clippy::cast_sign_loss)]
    fn parse_entropy_coded_data(
        &mut self,
        stream: &mut BitStream,
        buffer: &mut [Vec<i16>; MAX_COMPONENTS],
    ) -> Result<(), DecodeErrors> {
        stream.reset();
        self.components.iter_mut().for_each(|x| x.dc_pred = 0);

        if usize::from(self.num_scans) > self.input_colorspace.num_components() {
            return Err(Format(format!(
                "Number of scans {} cannot be greater than number of components, {}",
                self.num_scans,
                self.input_colorspace.num_components()
            )));
        }

        if self.num_scans == 1 {
            // Safety checks
            if self.spec_end != 0 && self.spec_start == 0 {
                return Err(DecodeErrors::FormatStatic(
                    "Can't merge DC and AC corrupt jpeg",
                ));
            }
            // non interleaved data, process one block at a time in trivial scanline order

            let k = self.z_order[0];

            if k >= self.components.len() {
                return Err(DecodeErrors::Format(format!(
                    "Cannot find component {k}, corrupt image"
                )));
            }

            let (mcu_width, mcu_height);

            if self.components[k].component_id == ComponentID::Y
                && (self.components[k].vertical_samp.u8() != 1
                    || self.components[k].horizontal_samp.u8() != 1)
                || !self.is_interleaved
            {
                // For Y channel  or non interleaved scans ,
                // mcu's is the image dimensions divided by 8
                mcu_width = ((self.info.width + 7) / 8) as usize;
                mcu_height = ((self.info.height + 7) / 8) as usize;
            } else {
                // For other channels, in an interleaved mcu, number of MCU's
                // are determined by some weird maths done in headers.rs->parse_sos()
                mcu_width = self.min_mcu_w;
                mcu_height = self.min_mcu_h;
            }

            for i in 0..mcu_height {
                for j in 0..mcu_width {
                    if self.spec_start != 0 && self.succ_high == 0 && stream.eob_run > 0 {
                        // handle EOB runs here.
                        stream.eob_run -= 1;
                        continue;
                    }
                    let start = 64 * (j + i * (self.components[k].width_stride / 8));

                    let data: &mut [i16; 64] = buffer
                        .get_mut(k)
                        .unwrap()
                        .get_mut(start..start + 64)
                        .unwrap()
                        .try_into()
                        .unwrap();

                    if self.spec_start == 0 {
                        let pos = self.components[k].dc_huff_table & (MAX_COMPONENTS - 1);
                        let dc_table = self
                            .dc_huffman_tables
                            .get(pos)
                            .ok_or(DecodeErrors::FormatStatic(
                                "No huffman table for DC component",
                            ))?
                            .as_ref()
                            .ok_or(DecodeErrors::FormatStatic(
                                "Huffman table at index  {} not initialized",
                            ))?;

                        let dc_pred = &mut self.components[k].dc_pred;

                        if self.succ_high == 0 {
                            // first scan for this mcu
                            stream.decode_prog_dc_first(
                                &mut self.stream,
                                dc_table,
                                &mut data[0],
                                dc_pred,
                            )?;
                        } else {
                            // refining scans for this MCU
                            stream.decode_prog_dc_refine(&mut self.stream, &mut data[0])?;
                        }
                    } else {
                        let pos = self.components[k].ac_huff_table;
                        let ac_table = self
                            .ac_huffman_tables
                            .get(pos)
                            .ok_or_else(|| {
                                DecodeErrors::Format(format!(
                                    "No huffman table for component:{pos}"
                                ))
                            })?
                            .as_ref()
                            .ok_or_else(|| {
                                DecodeErrors::Format(format!(
                                    "Huffman table at index  {pos} not initialized"
                                ))
                            })?;

                        if self.succ_high == 0 {
                            debug_assert!(stream.eob_run == 0, "EOB run is not zero");

                            stream.decode_mcu_ac_first(&mut self.stream, ac_table, data)?;
                        } else {
                            // refinement scan
                            stream.decode_mcu_ac_refine(&mut self.stream, ac_table, data)?;
                        }
                    }
                    // + EOB and investigate effect.
                    self.todo -= 1;

                    if self.todo == 0 {
                        self.handle_rst(stream)?;
                    }
                }
            }
        } else {
            if self.spec_end != 0 {
                return Err(DecodeErrors::HuffmanDecode(
                    "Can't merge dc and AC corrupt jpeg".to_string(),
                ));
            }
            // process scan n elements in order

            // Do the error checking with allocs here.
            // Make the one in the inner loop free of allocations.
            for k in 0..self.num_scans {
                let n = self.z_order[k as usize];

                if n >= self.components.len() {
                    return Err(DecodeErrors::Format(format!(
                        "Cannot find component {n}, corrupt image"
                    )));
                }

                let component = &mut self.components[n];
                let _ = self
                    .dc_huffman_tables
                    .get(component.dc_huff_table)
                    .ok_or_else(|| {
                        DecodeErrors::Format(format!(
                            "No huffman table for component:{}",
                            component.dc_huff_table
                        ))
                    })?
                    .as_ref()
                    .ok_or_else(|| {
                        DecodeErrors::Format(format!(
                            "Huffman table at index  {} not initialized",
                            component.dc_huff_table
                        ))
                    })?;
            }
            // Interleaved scan

            // Components shall not be interleaved in progressive mode, except for
            // the DC coefficients in the first scan for each component of a progressive frame.
            for i in 0..self.min_mcu_h {
                for j in 0..self.min_mcu_w {
                    // process scan n elements in order
                    for k in 0..self.num_scans {
                        let n = self.z_order[k as usize];
                        let component = &mut self.components[n];
                        let huff_table = self
                            .dc_huffman_tables
                            .get(component.dc_huff_table)
                            .ok_or(DecodeErrors::FormatStatic("No huffman table for component"))?
                            .as_ref()
                            .ok_or(DecodeErrors::FormatStatic(
                                "Huffman table at index not initialized",
                            ))?;

                        for v_samp in 0..component.vertical_samp.usize() {
                            for h_samp in 0..component.horizontal_samp.usize() {
                                let x2 = j * component.horizontal_samp.usize() + h_samp;
                                let y2 = i * component.vertical_samp.usize() + v_samp;
                                let position = 64 * (x2 + y2 * component.width_stride / 8);

                                let data = &mut buffer[n][position];

                                if self.succ_high == 0 {
                                    stream.decode_prog_dc_first(
                                        &mut self.stream,
                                        huff_table,
                                        data,
                                        &mut component.dc_pred,
                                    )?;
                                } else {
                                    stream.decode_prog_dc_refine(&mut self.stream, data)?;
                                }
                            }
                        }
                    }
                    // We want wrapping subtraction here because it means
                    // we get a higher number in the case this underflows
                    self.todo = self.todo.wrapping_sub(1);
                    // after every scan that's a mcu, count down restart markers.
                    if self.todo == 0 {
                        self.handle_rst(stream)?;
                    }
                }
            }
        }
        return Ok(());
    }

    pub(crate) fn reset_params(&mut self) {
        /*
        Apparently, grayscale images which can be down sampled exists, which is weird in the sense
        that it has one component Y, which is not usually down sampled.

        This means some calculations will be wrong, so for that we explicitly reset params
        for such occurrences, warn and reset the image info to appear as if it were
        a non-sampled image to ensure decoding works
        */
        self.max_horizontal_samp = 1;
        self.options = self.options.jpeg_set_out_colorspace(ColorSpace::Luma);
        self.max_vertical_samp = 1;
        self.sub_sample_ratio = SampleRatios::None;
        self.is_interleaved = false;
        self.components[0].vertical_samp = SampleFactor::One;
        self.components[0].width_stride = (((self.info.width as usize) + 7) / 8) * 8;
        self.components[0].horizontal_samp = SampleFactor::One;
    }
}

///Get a marker from the bit-stream.
///
/// This reads until it gets a marker or end of file is encountered
fn get_marker<T>(reader: &mut ZReader<T>, stream: &mut BitStream) -> Result<Marker, DecodeErrors>
where
    T: ZByteReaderTrait,
{
    if let Some(marker) = stream.marker {
        stream.marker = None;
        return Ok(marker);
    }

    // read until we get a marker

    while !reader.eof()? {
        let marker = reader.read_u8_err()?;

        if marker == 255 {
            let mut r = reader.read_u8_err()?;
            // 0xFF 0XFF(some images may be like that)
            while r == 0xFF {
                r = reader.read_u8_err()?;
            }

            if r != 0 {
                return Marker::from_u8(r)
                    .ok_or_else(|| DecodeErrors::Format(format!("Unknown marker 0xFF{r:X}")));
            }
        }
    }
    return Err(DecodeErrors::ExhaustedData);
}
