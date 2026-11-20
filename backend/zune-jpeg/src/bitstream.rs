/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

#![allow(
    clippy::if_not_else,
    clippy::similar_names,
    clippy::inline_always,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation
)]

//! This file exposes a single struct that can decode a huffman encoded
//! Bitstream in a JPEG file
//!
//! This code is optimized for speed.
//! It's meant to be super duper super fast, because everyone else depends on this being fast.
//! It's (annoyingly) serial hence we cant use parallel bitstreams(it's variable length coding.)
//!
//! Furthermore, on the case of refills, we have to do bytewise processing because the standard decided
//! that we want to support markers in the middle of streams(seriously few people use RST markers).
//!
//! So we pull in all optimization steps:
//! - use `inline[always]`? ✅ ,
//! - pre-execute most common cases ✅,
//! - add random comments ✅
//! -  fast paths ✅.
//!
//! Speed-wise: It is probably the fastest JPEG BitStream decoder to ever sail the seven seas because of
//! a couple of optimization tricks.
//! 1. Fast refills from libjpeg-turbo
//! 2. As few as possible branches in decoder fast paths.
//! 3. Accelerated AC table decoding borrowed from stb_image.h written by Fabian Gissen (@ rygorous),
//! improved by me to handle more cases.
//! 4. Safe and extensible routines(e.g. cool ways to eliminate bounds check)
//! 5. No unsafe here
//!
//! Readability comes as a second priority(I tried with variable names this time, and we are wayy better than libjpeg).
//!
//! Anyway if you are reading this it means your cool and I hope you get whatever part of the code you are looking for
//! (or learn something cool)
//!
//! Knock yourself out.
use alloc::format;
use alloc::string::ToString;
use core::cmp::min;

use zune_core::bytestream::{ZByteReaderTrait, ZReader};

use crate::errors::DecodeErrors;
use crate::huffman::{HUFF_LOOKAHEAD, HuffmanTable};
use crate::marker::Marker;
use crate::misc::UN_ZIGZAG;

/// A `BitStream` struct, a bit by bit reader with super powers
///
pub struct BitStream {
    /// A MSB type buffer that is used for some certain operations
    pub buffer: u64,
    /// A TOP  aligned MSB type buffer that is used to accelerate some operations like
    /// peek_bits and get_bits.
    ///
    /// By top aligned, I mean the top bit (63) represents the top bit in the buffer.
    aligned_buffer: u64,
    /// Tell us the bits left the two buffer
    pub(crate) bits_left: u8,
    /// Did we find a marker(RST/EOF) during decoding?
    pub marker: Option<Marker>,

    /// Progressive decoding
    pub successive_high: u8,
    pub successive_low: u8,
    spec_start: u8,
    spec_end: u8,
    pub eob_run: i32,
    pub overread_by: usize,
    /// True if we have seen end of image marker.
    /// Don't read anything after that.
    pub seen_eoi: bool,
}

impl BitStream {
    /// Create a new BitStream
    pub(crate) const fn new() -> Self {
        Self {
            buffer: 0,
            aligned_buffer: 0,
            bits_left: 0,
            marker: None,
            successive_high: 0,
            successive_low: 0,
            spec_start: 0,
            spec_end: 0,
            eob_run: 0,
            overread_by: 0,
            seen_eoi: false,
        }
    }

    /// Create a new Bitstream for progressive decoding
    #[allow(clippy::redundant_field_names)]
    pub(crate) const fn new_progressive(ah: u8, al: u8, spec_start: u8, spec_end: u8) -> Self {
        Self {
            buffer: 0,
            aligned_buffer: 0,
            bits_left: 0,
            marker: None,
            successive_high: ah,
            successive_low: al,
            spec_start: spec_start,
            spec_end: spec_end,
            eob_run: 0,
            overread_by: 0,
            seen_eoi: false,
        }
    }

    /// Refill the bit buffer by (a maximum of) 32 bits
    ///
    /// # Arguments
    ///  - `reader`:`&mut BufReader<R>`: A mutable reference to an underlying
    ///    File/Memory buffer containing a valid JPEG stream
    ///
    /// This function will only refill if `self.count` is less than 32
    #[inline(always)] // to many call sites? ( perf improvement by 4%)
    pub fn refill<T>(&mut self, reader: &mut ZReader<T>) -> Result<bool, DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        /// Macro version of a single byte refill.
        /// Arguments
        /// buffer-> our io buffer, because rust macros cannot get values from
        /// the surrounding environment bits_left-> number of bits left
        /// to full refill
        macro_rules! refill {
            ($buffer:expr,$byte:expr,$bits_left:expr) => {
                // read a byte from the stream
                $byte = u64::from(reader.read_u8());
                self.overread_by = self.overread_by.wrapping_add(usize::from(reader.eof()?));
                // append to the buffer
                // JPEG is a MSB type buffer so that means we append this
                // to the lower end (0..8) of the buffer and push the rest bits above..
                $buffer = ($buffer << 8) | $byte;
                // Increment bits left
                $bits_left = $bits_left.wrapping_add(8);
                // Check for special case  of OxFF, to see if it's a stream or a marker
                if $byte == 0xff {
                    // read next byte
                    let mut next_byte = reader.read_u8();
                    // Byte snuffing, if we encounter byte snuff, we skip the byte
                    if next_byte != 0x00 {
                        // skip that byte we read
                        while next_byte == 0xFF {
                            next_byte = reader.read_u8();
                        }

                        if next_byte != 0x00 {
                            // Undo the byte append and return
                            $buffer >>= 8;
                            $bits_left = $bits_left.wrapping_sub(8);

                            if $bits_left != 0 {
                                self.aligned_buffer = $buffer << 64_u8.wrapping_sub($bits_left);
                            }

                            self.marker = Some(Marker::from_u8(next_byte).ok_or_else(|| {
                                DecodeErrors::Format(format!("Unknown marker 0xFF{:X}", next_byte))
                            })?);
                            return Ok(false);
                        }
                    }
                }
            };
        }

        // 32 bits is enough for a decode(16 bits) and receive_extend(max 16 bits)
        // If we have less than 32 bits we refill
        if self.bits_left < 32 && self.marker.is_none() && !self.seen_eoi {
            // we optimize for the case where we don't have 255 in the stream and have 4 bytes left
            // as it is the common case
            //
            // so we always read 4 bytes, if read_fixed_bytes errors out, the cursor is
            // guaranteed not to advance in case of failure (is this true), so
            // we revert the read later on (if we have 255), if this fails, we use the normal
            // byte at a time read
            if let Ok(bytes) = reader.read_fixed_bytes_or_error::<4>() {
                // we have 4 bytes to spare, read the 4 bytes into a temporary buffer
                // create buffer
                let msb_buf = u32::from_be_bytes(bytes);
                // check if we have 0xff
                if !has_byte(msb_buf, 255) {
                    self.bits_left = self.bits_left.wrapping_add(32);
                    self.buffer <<= 32;
                    self.buffer |= u64::from(msb_buf);
                    self.aligned_buffer = self.buffer << 64_u8.wrapping_sub(self.bits_left);
                    return Ok(true);
                }

                reader.rewind(4)?;
            }
            // This serves two reasons,
            // 1: Make clippy shut up
            // 2: Favour register reuse
            let mut byte;
            // 4 refills, if all succeed the stream should contain enough bits to decode a
            // value
            refill!(self.buffer, byte, self.bits_left);
            refill!(self.buffer, byte, self.bits_left);
            refill!(self.buffer, byte, self.bits_left);
            refill!(self.buffer, byte, self.bits_left);
            // Construct an MSB buffer whose top bits are the bitstream we are currently holding.
            self.aligned_buffer = self.buffer << 64_u8.wrapping_sub(self.bits_left);
        }
        Ok(true)
    }
    /// Decode one Huffman symbol from the stream using `table`.
    ///
    /// Codes of up to `HUFF_LOOKAHEAD` bits are resolved by a single lookup;
    /// longer codes are resolved by scanning the pre-shifted `maxcode` table.
    #[inline(always)]
    fn decode_huff(&mut self, table: &HuffmanTable) -> Result<u8, DecodeErrors> {
        let entry = *table
            .lookup
            .get(usize::from(self.peek_bits::<HUFF_LOOKAHEAD>()))
            .ok_or_else(out_of_range)?;
        let code_length = u8::try_from(entry >> HUFF_LOOKAHEAD).map_err(|_| out_of_range())?;

        if code_length <= HUFF_LOOKAHEAD {
            self.drop_bits(code_length);
            return u8::try_from(entry & LOOKUP_VALUE_MASK).map_err(|_| out_of_range());
        }
        // if the symbol cannot be resolved in the first HUFF_LOOKAHEAD bits,
        // we know it lies somewhere between HUFF_LOOKAHEAD and 16 bits since jpeg imposes 16 bit
        // limit, we can therefore look 16 bits ahead and try to resolve the symbol
        // starting from 1+HUFF_LOOKAHEAD bits.
        let code = i32::from(self.peek_bits::<16>());
        // (Credits to Sean T. Barrett stb library for this optimization)
        // maxcode is pre-shifted 16 bytes long so that it has (16-code_length)
        // zeroes at the end hence we do not need to shift in the inner loop.
        let Some((length, (_, &offset))) = (code_length..17)
            .zip(
                table
                    .maxcode
                    .iter()
                    .zip(&table.offset)
                    .skip(usize::from(code_length)),
            )
            .find(|&(_, (&maxcode, _))| code < maxcode)
        else {
            // symbol could not be decoded.
            //
            // We may think, lets fake zeroes, noo
            // panic, because Huffman codes are sensitive, probably everything
            // after this will be corrupt, so no need to continue.
            return Err(DecodeErrors::Format(format!(
                "Bad Huffman Code 0x{code:X}, corrupt JPEG"
            )));
        };
        // The value table has 256 entries, so only the low byte of the index is used.
        let [.., index] = (code >> 16_u8.saturating_sub(length))
            .wrapping_add(offset)
            .to_be_bytes();
        let symbol = *table
            .values
            .get(usize::from(index))
            .ok_or_else(out_of_range)?;
        // drop bits read
        self.drop_bits(length);
        Ok(symbol)
    }

    /// Decode the DC coefficient in a MCU block.
    ///
    /// The decoded coefficient is written to `dc_prediction`
    ///
    #[inline(always)]
    fn decode_dc<T>(
        &mut self,
        reader: &mut ZReader<T>,
        dc_table: &HuffmanTable,
        dc_prediction: &mut i32,
    ) -> Result<bool, DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        if self.bits_left < 32 {
            self.refill(reader)?;
        }
        let symbol = self.decode_huff(dc_table)?;

        let diff = if symbol == 0 {
            0
        } else {
            if symbol > 16 {
                return Err(DecodeErrors::HuffmanDecode(format!(
                    "Invalid DC difference category {symbol}, corrupt JPEG"
                )));
            }
            huff_extend(self.get_bits(symbol), symbol)
        };
        // Update DC prediction
        *dc_prediction = dc_prediction.wrapping_add(diff);

        Ok(true)
    }

    /// Decode a Minimum Code Unit(MCU) as quickly as possible
    ///
    /// # Arguments
    /// - reader: The bitstream from where we read more bits.
    /// - dc_table: The Huffman table used to decode the DC coefficient
    /// - ac_table: The Huffman table used to decode AC values
    /// - block: A memory region where we will write out the decoded values
    /// - DC prediction: Last DC value for this component
    ///
    /// Delnegend: this fn only used for baseline jpegs
    #[inline(never)]
    pub fn decode_mcu_block<T>(
        &mut self,
        reader: &mut ZReader<T>,
        dc_table: &HuffmanTable,
        ac_table: &HuffmanTable,
        dct_coefs: &mut [i16],
        dc_prediction: &mut i32,
    ) -> Result<(), DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        // TODO: do something about this, maybe a custom error
        if dct_coefs.is_empty() {
            return Ok(());
        }
        let dct_coefs = dct_coefs
            .first_chunk_mut::<64>()
            .ok_or(DecodeErrors::FormatStatic(
                "MCU block has fewer than 64 coefficients",
            ))?;

        // Get fast AC table as a reference before we enter the hot path
        let ac_lookup = ac_table.ac_lookup.as_ref().ok_or(MISSING_AC_LOOKUP)?;

        // Decode AC coefficients
        let mut pos: usize = 1;

        // decode DC, dc prediction will contain the value
        self.decode_dc(reader, dc_table, dc_prediction)?;
        dct_coefs[0] = truncate_i16(*dc_prediction);

        while pos < 64 {
            self.refill(reader)?;
            let fast_ac = *ac_lookup
                .get(usize::from(self.peek_bits::<HUFF_LOOKAHEAD>()))
                .ok_or_else(out_of_range)?;

            if fast_ac != 0 {
                //  FAST AC path
                // low nibble: code length, high nibble: run
                let [info, _] = fast_ac.to_le_bytes();
                pos = pos.saturating_add(usize::from(info >> 4)); // run

                *coefficient_mut(dct_coefs, min(pos, 63))? = fast_ac >> 8;

                self.drop_bits(info & 15);
                pos = pos.saturating_add(1);
            } else {
                let symbol = self.decode_huff(ac_table)?;

                let r = symbol >> 4;
                let size = symbol & 15;

                if size != 0 {
                    pos = pos.saturating_add(usize::from(r));
                    let value = huff_extend(self.get_bits(size), size);

                    *coefficient_mut(dct_coefs, pos & 63)? = truncate_i16(value);

                    pos = pos.saturating_add(1);
                } else if r != 15 {
                    return Ok(());
                } else {
                    pos = pos.saturating_add(16);
                }
            }
        }
        Ok(())
    }

    /// Peek `LOOKAHEAD` (at most 16) bits ahead without discarding them from the buffer
    #[inline(always)]
    const fn peek_bits<const LOOKAHEAD: u8>(&self) -> u16 {
        let [high, low, ..] = self.aligned_buffer.to_be_bytes();
        u16::from_be_bytes([high, low]) >> 16_u8.wrapping_sub(LOOKAHEAD)
    }

    /// Discard the next `N` bits without checking
    #[inline]
    const fn drop_bits(&mut self, n: u8) {
        self.bits_left = self.bits_left.saturating_sub(n);
        self.aligned_buffer <<= n;
    }

    /// Read `n_bits` (at most 16) from the buffer and discard them
    #[inline(always)]
    fn get_bits(&mut self, n_bits: u8) -> i32 {
        let mask = (1_u64 << n_bits).saturating_sub(1);

        self.aligned_buffer = self.aligned_buffer.rotate_left(u32::from(n_bits));
        let [.., high, low] = (self.aligned_buffer & mask).to_be_bytes();
        self.bits_left = self.bits_left.wrapping_sub(n_bits);
        i32::from(u16::from_be_bytes([high, low]))
    }

    /// Decode a DC block
    #[inline]
    pub(crate) fn decode_prog_dc_first<T>(
        &mut self,
        reader: &mut ZReader<T>,
        dc_table: &HuffmanTable,
        block: &mut i16,
        dc_prediction: &mut i32,
    ) -> Result<(), DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        self.decode_dc(reader, dc_table, dc_prediction)?;
        *block = truncate_i16(*dc_prediction).wrapping_mul(1_i16 << self.successive_low);
        Ok(())
    }
    #[inline]
    pub(crate) fn decode_prog_dc_refine<T>(
        &mut self,
        reader: &mut ZReader<T>,
        block: &mut i16,
    ) -> Result<(), DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        // refinement scan
        if self.bits_left < 1 {
            self.refill(reader)?;
        }

        if self.get_bit() {
            *block = block.wrapping_add(1 << self.successive_low);
        }

        Ok(())
    }

    /// Get a single bit from the bitstream
    const fn get_bit(&mut self) -> bool {
        let k = self.aligned_buffer >> 63 != 0;
        // discard a bit
        self.drop_bits(1);
        k
    }
    pub(crate) fn decode_mcu_ac_first<T>(
        &mut self,
        reader: &mut ZReader<T>,
        ac_table: &HuffmanTable,
        block: &mut [i16; 64],
    ) -> Result<bool, DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        let shift = self.successive_low;
        let fast_ac = ac_table.ac_lookup.as_ref().ok_or(MISSING_AC_LOOKUP)?;

        let mut k = usize::from(self.spec_start);

        // EOB runs are handled in mcu_prog.rs
        'block: loop {
            self.refill(reader)?;

            let fac = *fast_ac
                .get(usize::from(self.peek_bits::<HUFF_LOOKAHEAD>()))
                .ok_or_else(out_of_range)?;

            if fac != 0 {
                // fast ac path
                // low nibble: code length, high nibble: run
                let [info, _] = fac.to_le_bytes();
                k = k.saturating_add(usize::from(info >> 4)); // run
                *coefficient_mut(block, min(k, 63))? = (fac >> 8).wrapping_mul(1 << shift); // value
                self.drop_bits(info & 15);
                k = k.saturating_add(1);
            } else {
                let symbol = self.decode_huff(ac_table)?;

                let r = symbol >> 4;
                let size = symbol & 15;

                if size != 0 {
                    k = k.saturating_add(usize::from(r));
                    let value = huff_extend(self.get_bits(size), size);
                    *coefficient_mut(block, k & 63)? = truncate_i16(value).wrapping_mul(1 << shift);
                    k = k.saturating_add(1);
                } else {
                    if r != 15 {
                        self.eob_run = (1_i32 << r)
                            .saturating_add(self.get_bits(r))
                            .saturating_sub(1);
                        break;
                    }

                    k = k.saturating_add(16);
                }
            }

            if k > usize::from(self.spec_end) {
                break 'block;
            }
        }
        Ok(true)
    }
    #[allow(clippy::too_many_lines, clippy::op_ref)]
    pub(crate) fn decode_mcu_ac_refine<T>(
        &mut self,
        reader: &mut ZReader<T>,
        table: &HuffmanTable,
        block: &mut [i16; 64],
    ) -> Result<bool, DecodeErrors>
    where
        T: ZByteReaderTrait,
    {
        let bit = truncate_i16(1_i32 << self.successive_low);

        let mut k = self.spec_start;

        if self.eob_run == 0 {
            'no_eob: loop {
                // Decode a coefficient from the bit stream
                self.refill(reader)?;

                let symbol = self.decode_huff(table)?;

                let mut r = symbol >> 4;
                let size = symbol & 15;
                // new non-zero coefficient, if any
                let mut value = 0_i16;

                if size == 0 {
                    if r != 15 {
                        // EOB run is 2^r + bits
                        self.eob_run = (1_i32 << r).saturating_add(self.get_bits(r));
                        // EOB runs are handled by the eob logic
                        break 'no_eob;
                    }
                } else {
                    if size != 1 {
                        return Err(DecodeErrors::HuffmanDecode(
                            "Bad Huffman code, corrupt JPEG?".to_string(),
                        ));
                    }
                    // get sign bit
                    // We assume we have enough bits, which should be correct for sane images
                    // since we refill by 32 above
                    value = if self.get_bit() {
                        bit
                    } else {
                        bit.saturating_neg()
                    };
                }

                // Advance over already nonzero coefficients  appending
                // correction bits to the non-zeroes.
                // A correction bit is 1 if the absolute value of the coefficient must be increased

                if k <= self.spec_end {
                    'advance_nonzero: loop {
                        let coefficient = coefficient_mut(block, usize::from(k) & 63)?;

                        if *coefficient != 0 {
                            if self.get_bit() && (*coefficient & bit) == 0 {
                                if *coefficient >= 0 {
                                    *coefficient = coefficient.wrapping_add(bit);
                                } else {
                                    *coefficient = coefficient.wrapping_sub(bit);
                                }
                            }

                            if self.bits_left < 1 {
                                self.refill(reader)?;
                            }
                        } else if let Some(remaining) = r.checked_sub(1) {
                            r = remaining;
                        } else {
                            // reached target zero coefficient.
                            break 'advance_nonzero;
                        }

                        if k == self.spec_end {
                            break 'advance_nonzero;
                        }

                        k = k.saturating_add(1);
                    }
                }

                if value != 0 {
                    // output new non-zero coefficient.
                    *coefficient_mut(block, usize::from(k) & 63)? = value;
                }

                k = k.saturating_add(1);

                if k > self.spec_end {
                    break 'no_eob;
                }
            }
        }
        if self.eob_run > 0 {
            // only run if block does not consists of purely zeroes
            if &block[1..] != &[0; 63] {
                self.refill(reader)?;

                for k in k..=self.spec_end {
                    let coefficient = coefficient_mut(block, usize::from(k) & 63)?;

                    if *coefficient != 0 && self.get_bit() {
                        // check if we already modified it, if so do nothing, otherwise
                        // append the correction bit.
                        if (*coefficient & bit) == 0 {
                            if *coefficient >= 0 {
                                *coefficient = coefficient.wrapping_add(bit);
                            } else {
                                *coefficient = coefficient.wrapping_sub(bit);
                            }
                        }
                    }
                    if self.bits_left < 1 {
                        // refill at the last possible moment
                        self.refill(reader)?;
                    }
                }
            }
            // count a block completed in EOB run
            self.eob_run = self.eob_run.saturating_sub(1);
        }
        Ok(true)
    }

    pub const fn update_progressive_params(
        &mut self,
        ah: u8,
        al: u8,
        spec_start: u8,
        spec_end: u8,
    ) {
        self.successive_high = ah;
        self.successive_low = al;
        self.spec_start = spec_start;
        self.spec_end = spec_end;
    }

    /// Reset the stream if we have a restart marker
    ///
    /// Restart markers indicate drop those bits in the stream and zero out
    /// everything
    #[cold]
    pub const fn reset(&mut self) {
        self.bits_left = 0;
        self.marker = None;
        self.buffer = 0;
        self.aligned_buffer = 0;
        self.eob_run = 0;
    }
}

/// Mask selecting the decoded value from a `HuffmanTable::lookup` entry.
const LOOKUP_VALUE_MASK: i32 = (1 << HUFF_LOOKAHEAD) - 1;

/// Error for AC tables built without the fast AC lookup table.
const MISSING_AC_LOOKUP: DecodeErrors =
    DecodeErrors::FormatStatic("AC Huffman table is missing its fast lookup table");

/// Error for table lookups that fall outside their table, only possible for corrupt tables.
#[cold]
const fn out_of_range() -> DecodeErrors {
    DecodeErrors::FormatStatic("Huffman table index out of range, corrupt JPEG")
}

/// Return the coefficient for zig-zag position `pos` (expected `< 64`) of `block`.
#[inline(always)]
fn coefficient_mut(block: &mut [i16; 64], pos: usize) -> Result<&mut i16, DecodeErrors> {
    let natural = UN_ZIGZAG.get(pos).ok_or_else(out_of_range)? & 63;
    block.get_mut(natural).ok_or_else(out_of_range)
}

/// Keep the low 16 bits of `value`, matching an `as i16` cast.
#[inline(always)]
const fn truncate_i16(value: i32) -> i16 {
    let [.., high, low] = value.to_be_bytes();
    i16::from_be_bytes([high, low])
}

/// Do the equivalent of JPEG HUFF_EXTEND
///
/// `x` holds `s` (1..=16) raw bits, so none of the operations below can overflow.
#[inline(always)]
const fn huff_extend(x: i32, s: u8) -> i32 {
    // if x<s return x else return x+offset[s] where offset[s] = ( (-1<<s)+1)
    x.saturating_add(
        (x.saturating_sub(1 << s.saturating_sub(1)) >> 31) & ((-1_i32 << s).saturating_add(1)),
    )
}

const fn has_zero(v: u32) -> bool {
    // Retrieved from Stanford bithacks
    // @ https://graphics.stanford.edu/~seander/bithacks.html#ZeroInWord
    // (v & 0x7F7F_7F7F) + 0x7F7F_7F7F <= 0xFEFE_FEFE, so the addition cannot overflow
    !(((v & 0x7F7F_7F7F).saturating_add(0x7F7F_7F7F) | v) | 0x7F7F_7F7F) != 0
}

const fn has_byte(b: u32, val: u8) -> bool {
    // Retrieved from Stanford bithacks
    // @ https://graphics.stanford.edu/~seander/bithacks.html#ZeroInWord
    // [val; 4] read as a u32 is the original `(!0 / 255) * val`
    has_zero(b ^ u32::from_ne_bytes([val; 4]))
}
