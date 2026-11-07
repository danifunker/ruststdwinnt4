//! Conversions between IEEE half precision (as its bits) and single precision, rounding to
//! nearest, ties to even. `core` calls them as `__gnu_h2f_ieee` and `__gnu_f2h_ieee` on
//! ARM; compiler_builtins leaves them out on this target.

/// The single-precision bits of the half-precision value `h`.
pub fn half_to_single(h: u16) -> u32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = ((h >> 10) & 0x1f) as i32;
    let mut man = (h & 0x3ff) as u32;
    match exp {
        0 if man == 0 => sign,
        0 => {
            // Subnormal: normalize. Its value is man * 2^-24 = (man / 1024) * 2^-14.
            let mut e = -14;
            while man & 0x400 == 0 {
                man <<= 1;
                e -= 1;
            }
            sign | (((e + 127) as u32) << 23) | ((man & 0x3ff) << 13)
        }
        // Infinity or NaN (the quiet bit, 0x200, lands on the single's quiet bit).
        0x1f => sign | 0x7f80_0000 | (man << 13),
        _ => sign | (((exp - 15 + 127) as u32) << 23) | (man << 13),
    }
}

/// The half-precision bits of the single-precision value with bits `x`.
pub fn single_to_half(x: u32) -> u16 {
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32;
    let man = x & 0x7f_ffff;
    if exp == 0xff {
        return if man != 0 {
            sign | 0x7c00 | 0x200 | (man >> 13) as u16
        } else {
            sign | 0x7c00
        };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    if e <= 0 {
        // Subnormal half (or zero). Below 2^-25 everything rounds to zero.
        if e < -10 {
            return sign;
        }
        let m = man | 0x80_0000;
        let shift = (14 - e) as u32;
        let half = 1u32 << (shift - 1);
        let rem = m & ((1 << shift) - 1);
        let mut r = m >> shift;
        if rem > half || (rem == half && r & 1 == 1) {
            r += 1;
        }
        return sign | r as u16;
    }
    let mut r = ((e as u32) << 10) | (man >> 13);
    let rem = man & 0x1fff;
    if rem > 0x1000 || (rem == 0x1000 && r & 1 == 1) {
        r += 1; // may carry into the exponent, up to infinity: that is the right result
    }
    sign | r as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h2f(h: u16) -> f32 {
        f32::from_bits(half_to_single(h))
    }
    fn f2h(f: f32) -> u16 {
        single_to_half(f.to_bits())
    }

    #[test]
    fn known_values() {
        assert_eq!(h2f(0x3c00), 1.0);
        assert_eq!(h2f(0xc000), -2.0);
        assert_eq!(h2f(0x7bff), 65504.0);
        assert_eq!(h2f(0x0400), 2f32.powi(-14));
        assert_eq!(h2f(0x0001), 2f32.powi(-24));
        assert_eq!(h2f(0x03ff), 1023.0 * 2f32.powi(-24));
        assert_eq!(h2f(0x7c00), f32::INFINITY);
        assert_eq!(h2f(0xfc00), f32::NEG_INFINITY);
        assert!(h2f(0x7e00).is_nan());
        assert_eq!(h2f(0x8000).to_bits(), (-0.0f32).to_bits());

        assert_eq!(f2h(1.0), 0x3c00);
        assert_eq!(f2h(65504.0), 0x7bff);
        assert_eq!(f2h(65520.0), 0x7c00, "rounds up to infinity");
        assert_eq!(f2h(65519.0), 0x7bff);
        assert_eq!(f2h(1e10), 0x7c00);
        assert_eq!(f2h(2f32.powi(-24)), 0x0001);
        assert_eq!(f2h(2f32.powi(-25)), 0x0000, "halfway to the smallest subnormal: ties to even");
        assert_eq!(f2h(2f32.powi(-25) * 1.0001), 0x0001);
        assert_eq!(f2h(1.5 * 2f32.powi(-24)), 0x0002, "halfway between 1 and 2 ulps: to even");
        assert_eq!(f2h(2.5 * 2f32.powi(-24)), 0x0002);
        assert_eq!(f2h(-0.0), 0x8000);
        assert_eq!(f2h(f32::NEG_INFINITY), 0xfc00);
        assert_eq!(f2h(f32::NAN) & 0x7e00, 0x7e00, "a quiet NaN");
        // 1 + 2^-11 is halfway between 1 and 1 + 2^-10: ties to even (1.0).
        assert_eq!(f2h(1.0 + 2f32.powi(-11)), 0x3c00);
        assert_eq!(f2h(1.0 + 3.0 * 2f32.powi(-11)), 0x3c02);
        // the largest subnormal rounding into the smallest normal
        assert_eq!(f2h(2f32.powi(-14) * (1.0 - 2f32.powi(-12))), 0x0400);
    }

    #[test]
    fn every_half_round_trips() {
        for h in 0..=u16::MAX {
            let f = h2f(h);
            if f.is_nan() {
                assert!(f2h(f) & 0x7c00 == 0x7c00 && f2h(f) & 0x3ff != 0, "{h:#06x}");
            } else {
                assert_eq!(f2h(f), h, "{h:#06x} -> {f:e}");
            }
        }
    }

    #[test]
    fn rounding_matches_the_nearest_half() {
        // For floats between two adjacent finite halves, the result must be the nearer
        // one (ties to the even one).
        for h in 0u16..0x7bff {
            let (lo, hi) = (h2f(h) as f64, h2f(h + 1) as f64);
            for k in 1..8 {
                let x = (lo + (hi - lo) * k as f64 / 8.0) as f32;
                let (dl, dh) = ((x as f64 - lo).abs(), (hi - x as f64).abs());
                let want = if dl < dh { h } else if dh < dl { h + 1 } else if h & 1 == 0 { h } else { h + 1 };
                assert_eq!(f2h(x), want, "{x:e} between {h:#06x} and {:#06x}", h + 1);
            }
        }
    }
}
