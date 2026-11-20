/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

//! This file contains a single struct `HuffmanTable` that
//! stores Huffman tables needed during `BitStream` decoding.

use alloc::string::ToString;

use crate::errors::DecodeErrors;

/// Determines how many bits of lookahead we have for our bitstream decoder.
pub const HUFF_LOOKAHEAD: u8 = 9;

/// A struct which contains necessary tables for decoding a JPEG
/// huffman encoded bitstream
pub struct HuffmanTable {
    // element `[0]` of each array is unused
    /// largest code of length k
    pub(crate) maxcode: [i32; 18],
    /// offset for codes of length k
    /// Answers the question, where do code-lengths of length k end
    /// Element 0 is unused
    pub(crate) offset: [i32; 18],
    /// lookup table for fast decoding
    ///
    /// top  bits above HUFF_LOOKAHEAD contain the code length.
    ///
    /// Lower (8) bits contain the symbol in order of increasing code length.
    pub(crate) lookup: [i32; 1 << HUFF_LOOKAHEAD],

    /// A table which can be used to decode small AC coefficients and
    /// do an equivalent of receive_extend
    pub(crate) ac_lookup: Option<[i16; 1 << HUFF_LOOKAHEAD]>,

    /// Directly represent contents of a JPEG DHT marker
    ///
    /// \# number of symbols with codes of length `k` bits
    // bits[0] is unused
    /// Symbols in order of increasing code length
    pub(crate) values: [u8; 256],
}

impl HuffmanTable {
    pub fn new(
        codes: &[u8; 17],
        values: [u8; 256],
        is_dc: bool,
        is_progressive: bool,
    ) -> Result<Self, DecodeErrors> {
        let too_long_code = i32::from(HUFF_LOOKAHEAD.saturating_add(1)) << HUFF_LOOKAHEAD;
        let mut p = Self {
            maxcode: [0; 18],
            offset: [0; 18],
            lookup: [too_long_code; 1 << HUFF_LOOKAHEAD],
            values,
            ac_lookup: None,
        };

        p.make_derived_table(is_dc, is_progressive, codes)?;

        Ok(p)
    }

    /// Create a new huffman tables with values that aren't fixed
    /// used by fill_mjpeg_tables
    pub fn new_unfilled(
        codes: &[u8; 17],
        values: &[u8],
        is_dc: bool,
        is_progressive: bool,
    ) -> Result<Self, DecodeErrors> {
        let mut buf = [0; 256];
        buf.get_mut(..values.len())
            .ok_or_else(|| DecodeErrors::HuffmanDecode("Bad Huffman Table".to_string()))?
            .copy_from_slice(values);
        Self::new(codes, buf, is_dc, is_progressive)
    }

    /// Compute derived values for a Huffman table
    ///
    /// This routine performs some validation checks on the table
    fn make_derived_table(
        &mut self,
        is_dc: bool,
        _is_progressive: bool,
        bits: &[u8; 17],
    ) -> Result<(), DecodeErrors> {
        // build a list of code size
        let mut huff_size = [0_u8; 257];
        // Huffman code lengths
        let mut huff_code: [u32; 257] = [0; 257];
        // figure C.1 make table of Huffman code length for each symbol
        let mut p: usize = 0;

        for (l, &count) in (1_u8..=16).zip(bits.iter().skip(1)) {
            // table overrun is checked before, this guards against malformed input
            for _ in 0..count {
                *huff_size.get_mut(p).ok_or_else(bad_table)? = l;
                p = p.checked_add(1).ok_or_else(bad_table)?;
            }
        }

        *huff_size.get_mut(p).ok_or_else(bad_table)? = 0;

        let num_symbols = p;
        // Generate the codes themselves
        // We also validate that the counts represent a legal Huffman code tree
        let mut code: u32 = 0;
        let mut si = huff_size[0];

        p = 0;

        while *huff_size.get(p).ok_or_else(bad_table)? != 0 {
            while *huff_size.get(p).ok_or_else(bad_table)? == si {
                *huff_code.get_mut(p).ok_or_else(bad_table)? = code;
                code = code.checked_add(1).ok_or_else(bad_table)?;
                p = p.checked_add(1).ok_or_else(bad_table)?;
            }
            // maximum code of length si, pre-shifted by 16-k bits
            let shift = 16_u8.checked_sub(si).ok_or_else(bad_table)?;
            *self
                .maxcode
                .get_mut(usize::from(si))
                .ok_or_else(bad_table)? = i32::try_from(code << shift).map_err(|_| bad_table())?;
            // code is now 1 more than the last code used for code-length si; but
            // it must still fit in si bits, since no code is allowed to be all ones.
            if code >= (1_u32 << si) {
                return Err(bad_table());
            }

            code <<= 1;
            si = si.checked_add(1).ok_or_else(bad_table)?;
        }

        // Figure F.15 generate decoding tables for bit-sequential decoding
        p = 0;

        for ((&count, maxcode), offset) in bits
            .iter()
            .zip(self.maxcode.iter_mut())
            .zip(self.offset.iter_mut())
        {
            if count == 0 {
                // -1 if no codes of this length
                *maxcode = -1;
            } else {
                // offset[l]=codes[index of 1st symbol of code length l
                // minus minimum code of length l]
                let first_code = *huff_code.get(p).ok_or_else(bad_table)?;
                *offset = i32::try_from(p)
                    .map_err(|_| bad_table())?
                    .checked_sub(i32::try_from(first_code).map_err(|_| bad_table())?)
                    .ok_or_else(bad_table)?;
                p = p.checked_add(usize::from(count)).ok_or_else(bad_table)?;
            }
        }

        self.offset[17] = 0;
        // we ensure that decode terminates
        self.maxcode[17] = 0x000F_FFFF;

        /*
         * Compute lookahead tables to speed up decoding.
         * First we set all the table entries to 0(left justified), indicating "too long";
         * (Note too long was set during initialization)
         * then we iterate through the Huffman codes that are short enough and
         * fill in all the entries that correspond to bit sequences starting
         * with that code.
         */

        p = 0;

        for (l, &count) in (1..=HUFF_LOOKAHEAD).zip(bits.iter().skip(1)) {
            let shift = HUFF_LOOKAHEAD.saturating_sub(l);
            for _ in 0..count {
                // l -> Current code length,
                // p => Its index in self.code and self.values
                // Generate left justified code followed by all possible bit sequences
                let start = usize::try_from(*huff_code.get(p).ok_or_else(bad_table)?)
                    .map_err(|_| bad_table())?
                    << shift;
                let end = start.checked_add(1 << shift).ok_or_else(bad_table)?;
                let value = (i32::from(l) << HUFF_LOOKAHEAD)
                    | i32::from(*self.values.get(p).ok_or_else(bad_table)?);
                self.lookup
                    .get_mut(start..end)
                    .ok_or_else(bad_table)?
                    .fill(value);

                p = p.checked_add(1).ok_or_else(bad_table)?;
            }
        }
        // build an ac table that does an equivalent of decode and receive_extend
        if !is_dc {
            self.ac_lookup = Some(self.build_ac_lookup(&huff_size, &huff_code, num_symbols)?);
        }

        // Validate symbols as being reasonable
        // For AC tables, we make no check, but accept all byte values 0..255
        // For DC tables, we require symbols to be in range 0..15
        if is_dc {
            for &sym in self.values.iter().take(num_symbols) {
                if sym > 15 {
                    return Err(bad_table());
                }
            }
        }

        Ok(())
    }

    /// Build a table that decodes both magnitude and value of small AC
    /// coefficients in one go (an equivalent of decode and `receive_extend`).
    fn build_ac_lookup(
        &self,
        huff_size: &[u8; 257],
        huff_code: &[u32; 257],
        num_symbols: usize,
    ) -> Result<[i16; 1 << HUFF_LOOKAHEAD], DecodeErrors> {
        let mut fast = [255_i16; 1 << HUFF_LOOKAHEAD];
        // Iterate over number of symbols
        for (i, (&s, &sym_code)) in huff_size
            .iter()
            .zip(huff_code.iter())
            .take(num_symbols)
            .enumerate()
        {
            // s is the code size for an item
            if s <= HUFF_LOOKAHEAD {
                // if it's lower than what we need for our lookup table create the table
                let shift = HUFF_LOOKAHEAD.saturating_sub(s);
                let c = usize::try_from(sym_code << shift).map_err(|_| bad_table())?;
                let m = 1_usize << shift;
                let end = c.checked_add(m).ok_or_else(bad_table)?;

                fast.get_mut(c..end)
                    .ok_or_else(bad_table)?
                    .fill(i16::try_from(i).map_err(|_| bad_table())?);
            }
        }

        let mut fast_ac = [0_i16; 1 << HUFF_LOOKAHEAD];

        for (i, (&fast_v, fast_ac_v)) in fast.iter().zip(fast_ac.iter_mut()).enumerate() {
            if fast_v < 255 {
                let fast_idx = usize::try_from(fast_v).map_err(|_| bad_table())?;
                // get symbol value from AC table
                let rs = *self.values.get(fast_idx).ok_or_else(bad_table)?;
                // shift by 4 to get run length
                let run = i16::from((rs >> 4) & 15);
                // get magnitude bits stored at the lower 3 bits
                let mag_bits = i16::from(rs & 15);
                // length of the bit we've read
                let len = i16::from(*huff_size.get(fast_idx).ok_or_else(bad_table)?);
                let total_len = len.checked_add(mag_bits).ok_or_else(bad_table)?;

                if mag_bits != 0 && total_len <= i16::from(HUFF_LOOKAHEAD) {
                    // magnitude code followed by receive_extend code
                    let mut k = ((i16::try_from(i).map_err(|_| bad_table())? << len)
                        & ((1 << HUFF_LOOKAHEAD) - 1))
                        >> i16::from(HUFF_LOOKAHEAD)
                            .checked_sub(mag_bits)
                            .ok_or_else(bad_table)?;
                    let m = 1 << mag_bits.checked_sub(1).ok_or_else(bad_table)?;

                    if k < m {
                        k = k
                            .checked_add(!0_i16 << mag_bits)
                            .and_then(|v| v.checked_add(1))
                            .ok_or_else(bad_table)?;
                    }

                    // if result is small enough fit into fast ac table
                    if (-128..=127).contains(&k) {
                        *fast_ac_v = (k << 8)
                            .checked_add(run << 4)
                            .and_then(|v| v.checked_add(total_len))
                            .ok_or_else(bad_table)?;
                    }
                }
            }
        }
        Ok(fast_ac)
    }
}

fn bad_table() -> DecodeErrors {
    DecodeErrors::HuffmanDecode("Bad Huffman Table".to_string())
}
