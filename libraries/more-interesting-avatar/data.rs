
const CENTER_PATCH_TYPES: [u32; 4] = [0, 4, 8, 15];

// These patches are defined on a 5x5 grid.
// For example, 3 is x=3, y=0 and 7 is x=2, y=1
const PATCH0: &[u8] = &[ 0, 4, 24, 20 ];
const PATCH1: &[u8] = &[ 0, 4, 20 ];
const PATCH2: &[u8] = &[ 2, 24, 20 ];
const PATCH3: &[u8] = &[ 0, 2, 20, 22 ];
const PATCH4: &[u8] = &[ 2, 14, 22, 10 ];
const PATCH5: &[u8] = &[ 0, 14, 24, 22 ];
const PATCH6: &[u8] = &[ 2, 24, 22, 13, 11, 22, 20 ];
const PATCH7: &[u8] = &[ 0, 14, 22 ];
const PATCH8: &[u8] = &[ 6, 8, 18, 16 ];
const PATCH9: &[u8] = &[ 4, 20, 10, 12, 2 ];
const PATCH10: &[u8] = &[ 0, 2, 12, 10 ];
const PATCH11: &[u8] = &[ 10, 14, 22 ];
const PATCH12: &[u8] = &[ 20, 12, 24 ];
const PATCH13: &[u8] = &[ 10, 2, 12 ];
const PATCH14: &[u8] = &[ 0, 2, 10 ];

const PATCH_SYMMETRIC: u8 = 1;
const PATCH_INVERTED: u8 = 2;

const PATCH_FLAGS: [u8; 16] = [ PATCH_SYMMETRIC, 0, 0, 0,
			PATCH_SYMMETRIC, 0, 0, 0, PATCH_SYMMETRIC, 0, 0, 0, 0, 0, 0,
			PATCH_SYMMETRIC + PATCH_INVERTED ];

const PATCH_TYPES: [&[u8]; 16] = [PATCH0, PATCH1, PATCH2,
			PATCH3, PATCH4, PATCH5, PATCH6, PATCH7, PATCH8, PATCH9, PATCH10,
			PATCH11, PATCH12, PATCH13, PATCH14, PATCH0];

// Each identicon is conceptually up of a 3x3 grid of "patches".
// They are defined above as 5x5 "pixels", but are rendered to 15x15
// monochrome images in build.rs. To fit all 225 pixels, we use a (u128, u128)
// wrapper struct.
#[derive(Clone, Copy, Debug, Default)]
struct BitImage {
    top: u128,
    bottom: u128,
}

#[allow(dead_code)]
impl BitImage {
    fn get<C: std::convert::TryInto<u32>>(self, x: C, y: C) -> bool where C::Error: std::fmt::Debug {
        let p = x.try_into().unwrap() + (y.try_into().unwrap() * 15);
        let ret = if p > 127 {
            self.bottom & (1 << (p - 127))
        } else {
            self.top & (1 << p)
        };
        ret != 0
    }
    fn set<C: std::convert::TryInto<u32>>(&mut self, x: C, y: C, bit: bool) where C::Error: std::fmt::Debug {
        let p = x.try_into().unwrap() + (y.try_into().unwrap() * 15);
        if p > 127 {
            if bit {
                self.bottom |= 1 << (p - 127);
            } else {
                self.bottom &= !(1 << (p - 127));
            }
        } else {
            if bit {
                self.top |= 1 << p;
            } else {
                self.top &= !(1 << p);
            }
        }
    }
}

// These are generated by build.rs
//const PATCH_SHAPES: [BitImage; 16] = generate_shapes();
//const PATCH_OUTLINES: [BitImage; 16] = generate_outlines();
