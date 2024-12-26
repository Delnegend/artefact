/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

use alloc::{format, vec};
use core::cmp::min;

use zune_core::bytestream::ZByteReaderTrait;
use zune_core::colorspace::ColorSpace;
use zune_core::log::{error, trace, warn};

use crate::bitstream::BitStream;
use crate::components::SampleRatios;
use crate::decoder::MAX_COMPONENTS;
use crate::errors::DecodeErrors;
use crate::marker::Marker;
use crate::JpegDecoder;

impl<T: ZByteReaderTrait> JpegDecoder<T> {
    /// Check for existence of DC and AC Huffman Tables
    pub(crate) fn check_tables(&self) -> Result<(), DecodeErrors> {
        // check that dc and AC tables exist outside the hot path
        for component in &self.components {
            let _ = &self
                .dc_huffman_tables
                .get(component.dc_huff_table)
                .as_ref()
                .ok_or_else(|| {
                    DecodeErrors::HuffmanDecode(format!(
                        "No Huffman DC table for component {:?} ",
                        component.component_id
                    ))
                })?
                .as_ref()
                .ok_or_else(|| {
                    DecodeErrors::HuffmanDecode(format!(
                        "No DC table for component {:?}",
                        component.component_id
                    ))
                })?;

            let _ = &self
                .ac_huffman_tables
                .get(component.ac_huff_table)
                .as_ref()
                .ok_or_else(|| {
                    DecodeErrors::HuffmanDecode(format!(
                        "No Huffman AC table for component {:?} ",
                        component.component_id
                    ))
                })?
                .as_ref()
                .ok_or_else(|| {
                    DecodeErrors::HuffmanDecode(format!(
                        "No AC table for component {:?}",
                        component.component_id
                    ))
                })?;
        }
        Ok(())
    }

    /// Decode MCUs and carry out post processing.
    ///
    /// This is the main decoder loop for the library, the hot path.
    ///
    /// Because of this, we pull in some very crazy optimization tricks hence readability is a pinch
    /// here.
    #[allow(
        clippy::similar_names,
        clippy::too_many_lines,
        clippy::cast_possible_truncation
    )]
    #[inline(never)]
    pub(crate) fn decode_mcu_ycbcr_baseline(
        &mut self,
        dct_coefs: &mut [Vec<i16>; MAX_COMPONENTS],
    ) -> Result<(), DecodeErrors> {
        // check dc and AC tables
        self.check_tables()?;

        let (mcu_width, mcu_height) = {
            let (mut mcu_width, mut mcu_height);

            if self.is_interleaved {
                // set upsampling functions
                self.set_upsampling()?;

                mcu_width = self.min_mcu_w;
                mcu_height = self.min_mcu_h;
            } else {
                // For non-interleaved images( (1*1) subsampling)
                // number of MCU's are the widths (+7 to account for paddings) divided bu 8.
                mcu_width = ((self.info.width + 7) / 8) as usize;
                mcu_height = ((self.info.height + 7) / 8) as usize;
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
                mcu_height *= self.max_vertical_samp;
                mcu_height /= self.max_horizontal_samp;
                self.coeff = 2;
            }

            if self.input_colorspace.num_components() > self.components.len() {
                let msg = format!(
                    " Expected {} number of components but found {}",
                    self.input_colorspace.num_components(),
                    self.components.len()
                );
                return Err(DecodeErrors::Format(msg));
            }

            if self.input_colorspace == ColorSpace::Luma && self.is_interleaved {
                warn!("Grayscale image with down-sampled component, resetting component details");

                self.reset_params();

                mcu_width = ((self.info.width + 7) / 8) as usize;
                mcu_height = ((self.info.height + 7) / 8) as usize;
            }

            (mcu_width, mcu_height)
        };

        let mut stream = BitStream::new();

        let comp_len = self.components.len();

        for (pos, comp) in self.components.iter_mut().enumerate() {
            // Allocate only needed components.
            //
            // For special colorspaces i.e YCCK and CMYK, just allocate all of the needed
            // components.
            if min(
                self.options.jpeg_get_out_colorspace().num_components() - 1,
                pos,
            ) == pos
                || comp_len == 4
            // Special colorspace
            {
                // allocate enough space to hold a whole MCU width
                // this means we should take into account sampling ratios
                // `*8` is because each MCU spans 8 widths.
                let len = comp.width_stride * comp.vertical_samp * 8;

                comp.needed = true;
                comp.raw_coeff = vec![0; len];
            } else {
                comp.needed = false;
            }
        }

        for curr_mcu_row in 0..mcu_height {
            // Report if we have no more bytes
            // This may generate false negatives since we over-read bytes
            // hence that why 37 is chosen(we assume if we over-read more than 37 bytes, we have a problem)
            if stream.overread_by > 37
            // favourite number :)
            {
                if self.options.strict_mode() {
                    return Err(DecodeErrors::FormatStatic("Premature end of buffer"));
                };

                error!("Premature end of buffer");
                break;
            }
            // decode a whole MCU width,
            // this takes into account interleaved components.
            if self.decode_mcu_width(mcu_width, &mut stream, curr_mcu_row, dct_coefs)? {
                warn!("Got terminate signal, will not process further");
                return Ok(());
            };
        }
        // it may happen that some images don't have the whole buffer
        // so we can't panic in case of that
        // assert_eq!(pixels_written, pixels.len());

        trace!("Finished decoding image");

        Ok(())
    }
    fn decode_mcu_width(
        &mut self,
        mcu_width: usize,
        stream: &mut BitStream,
        curr_mcu_row: usize,
        dct_coefs: &mut [Vec<i16>; MAX_COMPONENTS],
    ) -> Result<bool, DecodeErrors> {
        for curr_mcu_col in 0..mcu_width {
            // iterate over components
            for (comp_idx, comp) in &mut self.components.iter_mut().enumerate() {
                let dc_table = self.dc_huffman_tables[comp.dc_huff_table % MAX_COMPONENTS]
                    .as_ref()
                    .unwrap();
                let ac_table = self.ac_huffman_tables[comp.ac_huff_table % MAX_COMPONENTS]
                    .as_ref()
                    .unwrap();

                // If image is interleaved iterate over scan components,
                // otherwise if it-s non-interleaved, these routines iterate in
                // trivial scanline order(Y,Cb,Cr)
                match comp.sample_ratio {
                    SampleRatios::HV => {
                        let mcu_idx = curr_mcu_row * mcu_width + curr_mcu_col;
                        stream.decode_mcu_block(
                            &mut self.stream,
                            dc_table,
                            ac_table,
                            &mut dct_coefs[comp_idx][mcu_idx * 64..(mcu_idx + 1) * 64],
                            &mut comp.dc_pred,
                        )?;
                    }
                    SampleRatios::V => todo!(),
                    SampleRatios::H => todo!(),
                    SampleRatios::None => {
                        let mcu_idx = curr_mcu_row * mcu_width * 2 + curr_mcu_col;
                        for idx in [
                            2 * mcu_idx,
                            2 * mcu_idx + 1,
                            2 * mcu_idx + mcu_width * 2,
                            2 * mcu_idx + mcu_width * 2 + 1,
                        ] {
                            stream.decode_mcu_block(
                                &mut self.stream,
                                dc_table,
                                ac_table,
                                &mut dct_coefs[comp_idx][idx * 64..(idx + 1) * 64],
                                &mut comp.dc_pred,
                            )?;
                        }
                    }
                }
            }

            self.todo = self.todo.saturating_sub(1);
            // After all interleaved components, that's an MCU
            // handle stream markers
            //
            // In some corrupt images, it may occur that header markers occur in the stream.
            // The spec EXPLICITLY FORBIDS this, specifically, in
            // routine F.2.2.5  it says
            // `The only valid marker which may occur within the Huffman coded data is the RSTm marker.`
            //
            // But libjpeg-turbo allows it because of some weird reason. so I'll also
            // allow it because of some weird reason.
            if let Some(m) = stream.marker {
                if m == Marker::EOI {
                    // acknowledge and ignore EOI marker.
                    stream.marker.take();
                    trace!("Found EOI marker");
                    // Google Introduced the Ultra-HD image format which is basically
                    // stitching two images into one container.
                    // They basically separate two images via a EOI and SOI marker
                    // so let's just ensure if we ever see EOI, we never read past that
                    // ever.
                    // https://github.com/google/libultrahdr
                    stream.seen_eoi = true;
                } else if let Marker::RST(_) = m {
                    if self.todo == 0 {
                        self.handle_rst(stream)?;
                    }
                } else {
                    if self.options.strict_mode() {
                        return Err(DecodeErrors::Format(format!(
                            "Marker {m:?} found where not expected"
                        )));
                    }
                    error!(
                        "Marker `{:?}` Found within Huffman Stream, possibly corrupt jpeg",
                        m
                    );
                    self.parse_marker_inner(m)?;
                    if m == Marker::SOS {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
    // handle RST markers.
    // No-op if not using restarts
    // this routine is shared with mcu_prog
    #[cold]
    pub(crate) fn handle_rst(&mut self, stream: &mut BitStream) -> Result<(), DecodeErrors> {
        self.todo = self.restart_interval;

        if let Some(marker) = stream.marker {
            // Found a marker
            // Read stream and see what marker is stored there
            match marker {
                Marker::RST(_) => {
                    // reset stream
                    stream.reset();
                    // Initialize dc predictions to zero for all components
                    self.components.iter_mut().for_each(|x| x.dc_pred = 0);
                    // Start iterating again. from position.
                }
                Marker::EOI => {
                    // silent pass
                }
                _ => {
                    return Err(DecodeErrors::MCUError(format!(
                        "Marker {marker:?} found in bitstream, possibly corrupt jpeg"
                    )));
                }
            }
        }
        Ok(())
    }
}
