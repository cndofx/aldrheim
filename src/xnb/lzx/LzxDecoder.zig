// ported from https://github.com/DanielRodriguezAriza/MagickaPUP/blob/4ca5974a8590912e34004fcc524b198053d3469d/MagickaPUP/MagickaPUP/Utility/Compression/Lzx/LzxDecoder.cs
// which was copied or derived from https://github.com/MonoGame/MonoGame/blob/develop/MonoGame.Framework/Content/LzxDecoder.cs

// other references:
// https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-patch/cc78752a-b4af-4eee-88cb-01f4d8a4c2bf
// https://github.com/LeonBlade/xnbcli
// https://github.com/Lonami/lzxd

const std = @import("std");

const rh = @import("../reader_helpers.zig");

const LzxState = @import("LzxState.zig");
const BitBuffer = @import("BitBuffer.zig");

const LzxDecoder = @This();

const extra_bits = extraBits();
const position_base = positionBase();

const min_match = 2;
const max_match = 257;
const num_chars = 256;
const pretree_num_elements = 20;
const aligned_num_elements = 8;
const num_primary_lengths = 7;
const num_secondary_lengths = 249;
const pretree_maxsymbols = pretree_num_elements;
const pretree_tablebits = 6;
const maintree_maxsymbols = num_chars + (50 * 8);
const maintree_tablebits = 12;
const length_maxsymbols = num_secondary_lengths + 1;
const length_tablebits = 12;
const aligned_maxsymbols = aligned_num_elements;
const aligned_tablebits = 7;
const lentable_safety = 64;

state: LzxState,

pub fn init(gpa: std.mem.Allocator, window_size_pow2: u5) !LzxDecoder {
    if (window_size_pow2 < 15 or window_size_pow2 > 21) {
        return error.InvalidWindowSize;
    }

    const wndsize = @as(u32, 1) << window_size_pow2;

    const posn_slots: u16 = if (window_size_pow2 == 20)
        42
    else if (window_size_pow2 == 21)
        50
    else
        @as(u16, window_size_pow2) << 1;

    const window = try gpa.alloc(u8, wndsize);
    errdefer gpa.free(window);
    @memset(window, 0xDC);

    const pretree_table = try gpa.alloc(u16, (1 << pretree_tablebits) + (pretree_maxsymbols << 1));
    errdefer gpa.free(pretree_table);

    const pretree_len = try gpa.alloc(u8, (pretree_maxsymbols + lentable_safety));
    errdefer gpa.free(pretree_len);

    const maintree_table = try gpa.alloc(u16, (1 << maintree_tablebits) + (maintree_maxsymbols << 1));
    errdefer gpa.free(maintree_table);

    const maintree_len = try gpa.alloc(u8, maintree_maxsymbols + lentable_safety);
    errdefer gpa.free(maintree_len);

    const length_table = try gpa.alloc(u16, (1 << length_tablebits) + (length_maxsymbols << 1));
    errdefer gpa.free(length_table);

    const length_len = try gpa.alloc(u8, length_maxsymbols + lentable_safety);
    errdefer gpa.free(length_len);

    const aligned_table = try gpa.alloc(u16, (1 << aligned_tablebits) + (aligned_maxsymbols << 1));
    errdefer gpa.free(aligned_table);

    const aligned_len = try gpa.alloc(u8, aligned_maxsymbols + lentable_safety);
    errdefer gpa.free(aligned_len);

    @memset(pretree_table, 0);
    @memset(pretree_len, 0);
    @memset(maintree_table, 0);
    @memset(maintree_len, 0);
    @memset(length_table, 0);
    @memset(length_len, 0);
    @memset(aligned_table, 0);
    @memset(aligned_len, 0);

    return LzxDecoder{
        .state = LzxState{
            .window = window,
            .actual_size = wndsize,
            .window_size = wndsize,
            .window_posn = 0,

            .r0 = 1,
            .r1 = 1,
            .r2 = 1,
            .main_elements = num_chars + (posn_slots << 3),
            .header_read = 0,
            .frames_read = 0,
            .block_remaining = 0,
            .block_length = 0,
            .block_kind = .invalid,
            .intel_filesize = 0,
            .intel_curpos = 0,
            .intel_started = 0,

            .pretree_table = pretree_table,
            .pretree_len = pretree_len,
            .maintree_table = maintree_table,
            .maintree_len = maintree_len,
            .length_table = length_table,
            .length_len = length_len,
            .aligned_table = aligned_table,
            .aligned_len = aligned_len,
        },
    };
}

pub fn deinit(self: *LzxDecoder, gpa: std.mem.Allocator) void {
    gpa.free(self.state.window);
    gpa.free(self.state.pretree_table);
    gpa.free(self.state.pretree_len);
    gpa.free(self.state.maintree_table);
    gpa.free(self.state.maintree_len);
    gpa.free(self.state.length_table);
    gpa.free(self.state.length_len);
    gpa.free(self.state.aligned_table);
    gpa.free(self.state.aligned_len);
    self.* = undefined;
}

pub fn decompress(self: *LzxDecoder, gpa: std.mem.Allocator, in: *std.Io.Reader, in_len: usize, out: *std.Io.Writer, out_len: usize) !void {
    const start_pos = in.seek;
    const end_pos = start_pos + in_len;

    var bitbuf = BitBuffer{ .reader = in };

    const window = self.state.window;

    var window_posn = self.state.window_posn;
    const window_size = self.state.window_size;
    var r0 = self.state.r0;
    var r1 = self.state.r1;
    var r2 = self.state.r2;
    var i: u32 = 0;
    var j: u32 = 0;

    var togo: i32 = @intCast(out_len);
    var this_run: i32 = 0;
    var main_element: i32 = 0;
    var match_length: i32 = 0;
    var match_offset: i32 = 0;
    var length_footer: i32 = 0;
    var extra: i32 = 0;
    var verbatim_bits: i32 = 0;
    var rundest: i32 = 0;
    var runsrc: i32 = 0;
    var copy_length: i32 = 0;
    var aligned_bits: i32 = 0;

    // read header if necessary
    if (self.state.header_read == 0) {
        const intel = try bitbuf.readBits(1);
        if (intel != 0) {
            i = try bitbuf.readBits(16);
            j = try bitbuf.readBits(16);
            self.state.intel_filesize = @intCast((i << 16) | j);
        }
        self.state.header_read = 1;
    }

    // main decoding loop
    while (togo > 0) {
        // last block finished, new block expected
        if (self.state.block_remaining == 0) {
            if (self.state.block_kind == .uncompressed) {
                if ((self.state.block_length & 1) == 1) {
                    // realign bitbuf to word
                    _ = try rh.readU8(in);
                    bitbuf.clear();
                }
            }

            self.state.block_kind = @enumFromInt(try bitbuf.readBits(3));
            i = try bitbuf.readBits(16);
            j = try bitbuf.readBits(8);
            self.state.block_length = (i << 8) | j;
            self.state.block_remaining = self.state.block_length;

            state: switch (self.state.block_kind) {
                .aligned => {
                    i = 0;
                    j = 0;
                    while (i < 8) : (i += 1) {
                        j = try bitbuf.readBits(3);
                        self.state.aligned_len[i] = @intCast(j);
                    }

                    try makeDecodeTable(aligned_maxsymbols, aligned_tablebits, self.state.aligned_len, self.state.aligned_table);
                    // rest of aligned header is same as verbatim
                    continue :state .verbatim;
                },
                .verbatim => {
                    try self.readLengths(self.state.maintree_len, 0, 256, &bitbuf);
                    try self.readLengths(self.state.maintree_len, 256, self.state.main_elements, &bitbuf);
                    try makeDecodeTable(maintree_maxsymbols, maintree_tablebits, self.state.maintree_len, self.state.maintree_table);
                    if (self.state.maintree_len[0xE8] != 0) {
                        self.state.intel_started = 1;
                    }

                    try self.readLengths(self.state.length_len, 0, num_secondary_lengths, &bitbuf);
                    try makeDecodeTable(length_maxsymbols, length_tablebits, self.state.length_len, self.state.length_table);
                },
                .uncompressed => {
                    self.state.intel_started = 1; // because we can't assume otherwise
                    try bitbuf.ensureBits(16); // get up to 16 pad bits into the buffer
                    if (bitbuf.bits_left > 16) {
                        in.seek -= 2; // and align the bitbuffer!
                    }

                    var hi: u32 = 0;
                    var mh: u32 = 0;
                    var ml: u32 = 0;
                    var lo: u32 = 0;

                    lo = @intCast(try rh.readU8(in));
                    ml = @intCast(try rh.readU8(in));
                    mh = @intCast(try rh.readU8(in));
                    hi = @intCast(try rh.readU8(in));
                    r0 = lo | ml << 8 | mh << 16 | hi << 24;

                    lo = @intCast(try rh.readU8(in));
                    ml = @intCast(try rh.readU8(in));
                    mh = @intCast(try rh.readU8(in));
                    hi = @intCast(try rh.readU8(in));
                    r1 = lo | ml << 8 | mh << 16 | hi << 24;

                    lo = @intCast(try rh.readU8(in));
                    ml = @intCast(try rh.readU8(in));
                    mh = @intCast(try rh.readU8(in));
                    hi = @intCast(try rh.readU8(in));
                    r2 = lo | ml << 8 | mh << 16 | hi << 24;
                },
                else => {
                    return error.InvalidBlock;
                },
            }
        }

        // buffer exhaustion check
        if (in.seek > end_pos) {
            // it's possible to have a file where the next run is less than
            // 16 bits in size. In this case, the READ_HUFFSYM() macro used
            // in building the tables will exhaust the buffer, so we should
            // allow for this, but not allow those accidentally read bits to
            // be used (so we check that there are at least 16 bits
            // remaining - in this boundary case they aren't really part of
            // the compressed data)

            // if (inData.Position > (startpos + inLen + 2) || bitbuf.GetBitsLeft() < 16) return -1;
            if (in.seek > (end_pos + 2) or bitbuf.bits_left < 16) {
                return error.BufferOverrun;
            }
        }

        this_run = @intCast(self.state.block_remaining);
        while (this_run > 0 and togo > 0) {
            if (this_run > togo) {
                this_run = togo;
            }
            togo -= this_run;
            self.state.block_remaining -= @intCast(this_run);

            // apply 2^x-1 mask
            window_posn &= window_size - 1;
            // runs can't straddle the window wraparound
            if (window_posn + @as(u32, @intCast(this_run)) > window_size) {
                return error.SomethingBad; // TODO
            }

            switch (self.state.block_kind) {
                .verbatim => {
                    while (this_run > 0) {
                        main_element = @intCast(try readHuffSym(self.state.maintree_table, self.state.maintree_len, maintree_maxsymbols, maintree_tablebits, &bitbuf));
                        if (main_element < num_chars) {
                            // literal: 0 to NUM_CHARS-1
                            window[window_posn] = @intCast(main_element);
                            window_posn += 1;
                            this_run -= 1;
                        } else {
                            // match: NUM_CHARS + ((slot<<3) | length_header (3 bits))
                            main_element -= num_chars;

                            match_length = main_element & num_primary_lengths;
                            if (match_length == num_primary_lengths) {
                                length_footer = @intCast(try readHuffSym(self.state.length_table, self.state.length_len, length_maxsymbols, length_tablebits, &bitbuf));
                                match_length += length_footer;
                            }
                            match_length += min_match;

                            match_offset = main_element >> 3;

                            if (match_offset > 2) {
                                // not repeated offset
                                if (match_offset != 3) {
                                    extra = extra_bits[@intCast(match_offset)];
                                    verbatim_bits = @intCast(try bitbuf.readBits(@intCast(extra)));
                                    match_offset = @as(i32, @intCast(position_base[@intCast(match_offset)])) - 2 + verbatim_bits;
                                } else {
                                    match_offset = 1;
                                }

                                // update repeated offset LRU queue
                                r2 = r1;
                                r1 = r0;
                                r0 = @intCast(match_offset);
                            } else if (match_offset == 0) {
                                match_offset = @intCast(r0);
                            } else if (match_offset == 1) {
                                match_offset = @intCast(r1);
                                r1 = r0;
                                r0 = @intCast(match_offset);
                            } else { // match_offset == 2
                                match_offset = @intCast(r2);
                                r2 = r0;
                                r0 = @intCast(match_offset);
                            }

                            rundest = @intCast(window_posn);
                            this_run -= match_length;

                            // copy any wrapped around source data
                            if (window_posn >= match_offset) {
                                // no wrap
                                runsrc = rundest - match_offset;
                            } else {
                                runsrc = rundest + (@as(i32, @intCast(window_size)) - match_offset);
                                copy_length = match_offset - @as(i32, @intCast(window_posn));
                                if (copy_length < match_length) {
                                    match_length -= copy_length;
                                    window_posn += @intCast(copy_length);
                                    while (copy_length > 0) : (copy_length -= 1) {
                                        window[@intCast(rundest)] = window[@intCast(runsrc)];
                                        rundest += 1;
                                        runsrc += 1;
                                    }
                                    runsrc = 0;
                                }
                            }
                            window_posn += @intCast(match_length);

                            // copy match data, no worries about destination wraps
                            while (match_length > 0) : (match_length -= 1) {
                                window[@intCast(rundest)] = window[@intCast(runsrc)];
                                rundest += 1;
                                runsrc += 1;
                            }
                        }
                    }
                },
                .aligned => {
                    while (this_run > 0) {
                        main_element = @intCast(try readHuffSym(self.state.maintree_table, self.state.maintree_len, maintree_maxsymbols, maintree_tablebits, &bitbuf));

                        if (main_element < num_chars) {
                            // literal 0 to NUM_CHARS-1
                            window[window_posn] = @intCast(main_element);
                            window_posn += 1;
                            this_run -= 1;
                        } else {
                            // match: NUM_CHARS + ((slot<<3) | length_header (3 bits))
                            main_element -= num_chars;

                            match_length = main_element & num_primary_lengths;
                            if (match_length == num_primary_lengths) {
                                length_footer = @intCast(try readHuffSym(self.state.length_table, self.state.length_len, length_maxsymbols, length_tablebits, &bitbuf));
                                match_length += length_footer;
                            }
                            match_length += min_match;

                            match_offset = main_element >> 3;

                            if (match_offset > 2) {
                                // not repeated offset
                                extra = extra_bits[@intCast(match_offset)];
                                match_offset = @as(i32, @intCast(position_base[@intCast(match_offset)])) - 2;
                                if (extra > 3) {
                                    // verbatim and aligned bits
                                    extra -= 3;
                                    verbatim_bits = @intCast(try bitbuf.readBits(@intCast(extra)));
                                    match_offset += verbatim_bits << 3;
                                    aligned_bits = @intCast(try readHuffSym(self.state.aligned_table, self.state.aligned_len, aligned_maxsymbols, aligned_tablebits, &bitbuf));
                                    match_offset += aligned_bits;
                                } else if (extra == 3) {
                                    // aligned bits only
                                    aligned_bits = @intCast(try readHuffSym(self.state.aligned_table, self.state.aligned_len, aligned_maxsymbols, aligned_tablebits, &bitbuf));
                                    match_offset += aligned_bits;
                                } else if (extra > 0) { // extra == 1, extra == 1
                                    // verbatim bits only
                                    verbatim_bits = @intCast(try bitbuf.readBits(@intCast(extra)));
                                    match_offset += verbatim_bits;
                                } else {
                                    // ???
                                    match_offset = 1;
                                }

                                r2 = r1;
                                r1 = r0;
                                r0 = @intCast(match_offset);
                            } else if (match_offset == 0) {
                                match_offset = @intCast(r0);
                            } else if (match_offset == 1) {
                                match_offset = @intCast(r1);
                                r1 = r0;
                                r0 = @intCast(match_offset);
                            } else {
                                // match_offset == 2
                                match_offset = @intCast(r2);
                                r2 = r0;
                                r0 = @intCast(match_offset);
                            }

                            rundest = @intCast(window_posn);
                            this_run -= match_length;

                            // copy any wrapped around source data
                            if (window_posn >= match_offset) {
                                // no wrap
                                runsrc = rundest - match_offset;
                            } else {
                                runsrc = rundest + (@as(i32, @intCast(window_size)) - match_offset);
                                copy_length = match_offset - @as(i32, @intCast(window_posn));
                                if (copy_length < match_length) {
                                    match_length -= copy_length;
                                    window_posn += @intCast(copy_length);
                                    while (copy_length > 0) : (copy_length -= 1) {
                                        window[@intCast(rundest)] = window[@intCast(runsrc)];
                                        rundest += 1;
                                        runsrc += 1;
                                    }
                                    runsrc = 0;
                                }
                            }
                            window_posn += @intCast(match_length);

                            // copy match data, no worries about destination wraps
                            while (match_length > 0) : (match_length -= 1) {
                                window[@intCast(rundest)] = window[@intCast(runsrc)];
                                rundest += 1;
                                runsrc += 1;
                            }
                        }
                    }
                },
                .uncompressed => {
                    // if ((inData.Position + this_run) > endpos) return -1;
                    const temp_buffer = try gpa.alloc(u8, @intCast(this_run));
                    defer gpa.free(temp_buffer);
                    try in.readSliceAll(temp_buffer);
                    @memcpy(window[window_posn .. window_posn + @as(usize, @intCast(this_run))], temp_buffer);
                    window_posn += @intCast(this_run);
                },
                else => {
                    return error.InvalidBlock;
                },
            }
        }
    }

    if (togo != 0) {
        return error.SomethingBad; // TODO
    }

    var start_window_pos = window_posn;
    if (start_window_pos == 0) {
        start_window_pos = window_size;
    }
    start_window_pos -= @intCast(out_len);
    try out.writeAll(window[start_window_pos .. start_window_pos + out_len]);

    self.state.window_posn = window_posn;
    self.state.r0 = r0;
    self.state.r1 = r1;
    self.state.r2 = r2;

    // TODO: intel E8 decoding
}

fn readLengths(self: *LzxDecoder, lens: []u8, first: u32, last: u32, bitbuf: *BitBuffer) !void {
    var x: u32 = 0;
    var y: u32 = 0;
    var z: i32 = 0;

    x = 0;
    while (x < 20) : (x += 1) {
        y = try bitbuf.readBits(4);
        self.state.pretree_len[x] = @intCast(y);
    }
    try makeDecodeTable(pretree_maxsymbols, pretree_tablebits, self.state.pretree_len, self.state.pretree_table);

    x = first;
    while (x < last) {
        z = @intCast(try readHuffSym(self.state.pretree_table, self.state.pretree_len, pretree_maxsymbols, pretree_tablebits, bitbuf));

        if (z == 17) {
            y = try bitbuf.readBits(4);
            y += 4;
            while (y != 0) : (y -= 1) {
                lens[x] = 0;
                x += 1;
            }
        } else if (z == 18) {
            y = try bitbuf.readBits(5);
            y += 20;
            while (y != 0) : (y -= 1) {
                lens[x] = 0;
                x += 1;
            }
        } else if (z == 19) {
            y = try bitbuf.readBits(1);
            y += 4;
            z = @intCast(try readHuffSym(self.state.pretree_table, self.state.pretree_len, pretree_maxsymbols, pretree_tablebits, bitbuf));
            z = lens[x] - z;
            if (z < 0) {
                z += 17;
            }
            while (y != 0) : (y -= 1) {
                lens[x] = @intCast(z);
                x += 1;
            }
        } else {
            z = lens[x] - z;
            if (z < 0) {
                z += 17;
            }
            lens[x] = @intCast(z);
            x += 1;
        }
    }
}

fn readHuffSym(table: []u16, lengths: []u8, nsyms: u32, nbits: u5, bitbuf: *BitBuffer) !u32 {
    var i: u32 = 0;
    var j: u32 = 0;
    try bitbuf.ensureBits(16);

    i = table[bitbuf.peekBits(nbits)];
    if (i >= nsyms) {
        const shift: u5 = @intCast(32 - @as(u8, nbits));
        j = @as(u32, 1) << shift;
        while (true) {
            j >>= 1;
            i <<= 1;
            i |= if ((bitbuf.buffer & j) != 0) 1 else 0;
            if (j == 0) {
                return error.ReadHuffSymFailed;
            }

            i = table[i];
            if (i < nsyms) {
                break;
            }
        }
    }
    j = lengths[i];
    bitbuf.removeBits(@intCast(j));

    return i;
}

fn makeDecodeTable(nsyms: u32, nbits: u5, length: []const u8, table: []u16) !void {
    var sym: u16 = 0;
    var leaf: u32 = 0;
    var bit_num: u8 = 1;
    var fill: u32 = 0;
    var pos: u32 = 0;
    var table_mask: u32 = @as(u32, 1) << nbits;
    var bit_mask = table_mask >> 1;
    var next_symbol = bit_mask;

    // fill entries for codes short enough for a direct mapping
    while (bit_num <= nbits) {
        sym = 0;
        while (sym < nsyms) : (sym += 1) {
            if (length[sym] == bit_num) {
                leaf = pos;

                pos += bit_mask;
                if (pos > table_mask) {
                    return error.TableOverrun;
                }

                // fill all possible lookups of this symbol with the symbol itself
                fill = bit_mask;
                while (fill > 0) : (fill -= 1) {
                    table[leaf] = sym;
                    leaf += 1;
                }
            }
        }

        bit_mask >>= 1;
        bit_num += 1;
    }

    // if there are any codes longer than nbits
    if (pos != table_mask) {
        // clear the remainder of the table
        sym = @intCast(pos);
        while (sym < table_mask) : (sym += 1) {
            table[sym] = 0;
        }

        // give ourselves room for codes to grow by up to 16 more bits
        pos <<= 16;
        table_mask <<= 16;
        bit_mask = 1 << 15;

        while (bit_num <= 16) {
            sym = 0;
            while (sym < nsyms) : (sym += 1) {
                if (length[sym] == bit_num) {
                    leaf = pos >> 16;
                    fill = 0;
                    while (fill < bit_num - nbits) : (fill += 1) {
                        // if this path hasn't been taken yet, allocate two entries
                        if (table[leaf] == 0) {
                            table[(next_symbol << 1)] = 0;
                            table[(next_symbol << 1) + 1] = 0;
                            table[leaf] = @intCast(next_symbol);
                            next_symbol += 1;
                        }

                        // follow the path and select either left or right for next bit
                        leaf = table[leaf] << 1;
                        const shift: u5 = @intCast(15 - fill);
                        if (((pos >> shift) & 1) == 1) {
                            leaf += 1;
                        }
                    }
                    table[leaf] = sym;

                    pos += bit_mask;
                    if (pos > table_mask) {
                        return error.TableOverrun;
                    }
                }
            }

            bit_mask >>= 1;
            bit_num += 1;
        }
    }

    // full table?
    if (pos == table_mask) {
        return;
    }

    // either erroneous table or all elements are 0, lets find out
    sym = 0;
    while (sym < nsyms) : (sym += 1) {
        if (length[sym] != 0) {
            return error.ErroneousTable;
        }
    }
}

fn extraBits() [52]u8 {
    var result: [52]u8 = undefined;

    var i: usize = 0;
    var j: u8 = 0;
    while (i <= 50) : (i += 2) {
        result[i] = j;
        result[i + 1] = j;
        if (i != 0 and j < 17) {
            j += 1;
        }
    }

    return result;
}

fn positionBase() [51]u32 {
    var result: [51]u32 = undefined;

    var i: usize = 0;
    var j: u32 = 0;

    while (i <= 50) : (i += 1) {
        result[i] = j;
        j += 1 << extra_bits[i];
    }

    return result;
}
