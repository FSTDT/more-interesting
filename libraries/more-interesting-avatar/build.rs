#![allow(unused)]

include!("data.rs");

use std::fs::File;
use std::io::{Read, Write};
use std::env;

pub fn main() {
  let mut f = File::create(format!("{}/{}", env::var("OUT_DIR").unwrap(), "data.rs")).unwrap();
  write!(&mut f, "const PATCH_SHAPES: [u128; 16] = {:?};", generate_shapes()).unwrap();
  write!(&mut f, "const PATCH_OUTLINES: [u128; 16] = {:?};", generate_outlines()).unwrap();
}

fn generate_outlines() -> [u128; 16] {
  let mut ret_val = [0; 16];
  for (i, patch_type) in PATCH_TYPES[..16].iter().copied().enumerate() {
    let mut patch_shape: u128 = 0;
    for p in 0..100 {
      let x = p % 10;
      let y = (p - x) / 10;
      if is_point_in_path(x, y, patch_type) {
        if x > 0 && !is_point_in_path(x - 1, y, patch_type) {
          patch_shape |= 1 << p;
          patch_shape |= 1 << (p - 1);
        } else if x < 10 && !is_point_in_path(x + 1, y, patch_type) {
          patch_shape |= 1 << p;
          patch_shape |= 1 << (p + 1);
        }
        if y > 0 && !is_point_in_path(x, y - 1, patch_type) {
          patch_shape |= 1 << p;
          patch_shape |= 1 << (p - 10);
        } else if y < 10 && !is_point_in_path(x, y + 1, patch_type) {
          patch_shape |= 1 << p;
          patch_shape |= 1 << (p + 10);
        }
      }
    }
    ret_val[i] = patch_shape;
  }
  ret_val
}

fn generate_shapes() -> [u128; 16] {
  let mut ret_val = [0; 16];
  for (i, patch_type) in PATCH_TYPES[..16].iter().copied().enumerate() {
    let mut patch_shape: u128 = 0;
    for p in 0..100 {
      let x = p % 10;
      let y = (p - x) / 10;
      if is_point_in_path(x, y, patch_type) {
        patch_shape |= 1 << p;
      }
    }
    ret_val[i] = patch_shape;
  }
  ret_val
}

// https://en.wikipedia.org/wiki/Even%E2%80%93odd_rule
fn is_point_in_path(x: u8, y: u8, path: &[u8]) -> bool {
  // we need to count to 100
  // 2^7 is 128, so it's fine to use i8 here
  let num = path.len();
  let mut j = num - 1;
  let mut c = false;
  let x = x as i8;
  let y = y as i8;
  for (i, v) in path.iter().copied().enumerate() {
    let vx = v as i8 % 10;
    let vy = v as i8 / 10;
    let wx = path[j] as i8 % 10;
    let wy = path[j] as i8 / 10;
    println!("x={}, y={}, vx={}, vy={}, wx={}, wy={}", x, y, vx, vy, wx, wy);
    if ((vy > y) != (wy > y)) &&
       (x < vx + (wx - vx) * (y - vy) /
                 (wy - vy)) {
        c = !c;
    }
    j = i;
  }
  c
}

