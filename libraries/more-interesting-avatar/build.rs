#![allow(unused)]

include!("data.rs");

use std::fs::File;
use std::io::{Read, Write};
use std::env;

pub fn main() {
  let mut f = File::create(format!("{}/{}", env::var("OUT_DIR").unwrap(), "data.rs")).unwrap();
  write!(&mut f, "const PATCH_SHAPES: [BitImage; 16] = {:?};", generate_shapes()).unwrap();
  write!(&mut f, "const PATCH_OUTLINES: [BitImage; 16] = {:?};", generate_outlines()).unwrap();
}

fn generate_outlines() -> [BitImage; 16] {
  let mut ret_val = [BitImage::default(); 16];
  for (i, patch_type) in PATCH_TYPES[..16].iter().copied().enumerate() {
    let patch_type: Vec<(i32, i32)> = patch_type.iter().copied().map(|p| ((p as i32 % 5) * 3, (p as i32 / 5) * 3)).collect();
    let mut patch_shape: BitImage = BitImage::default();
    for p in 0..225 {
      let x = p % 15;
      let y = p / 15;
      if is_point_in_path(x, y, &patch_type) {
        if x > 0 && !is_point_in_path(x - 1, y, &patch_type) {
          patch_shape.set(x, y, true);
          patch_shape.set(x - 1, y, true);
        } else if x < 15 && !is_point_in_path(x + 1, y, &patch_type) {
          patch_shape.set(x, y, true);
          patch_shape.set(x + 1, y, true);
        }
        if y > 0 && !is_point_in_path(x, y - 1, &patch_type) {
          patch_shape.set(x, y, true);
          patch_shape.set(x, y - 1, true);
        } else if y < 15 && !is_point_in_path(x, y + 1, &patch_type) {
          patch_shape.set(x, y, true);
          patch_shape.set(x, y + 1, true);
        }
      }
    }
    ret_val[i] = patch_shape;
  }
  ret_val
}

fn generate_shapes() -> [BitImage; 16] {
  let mut ret_val = [BitImage::default(); 16];
  for (i, patch_type) in PATCH_TYPES[..16].iter().copied().enumerate() {
    let patch_type: Vec<(i32, i32)> = patch_type.iter().copied().map(|p| ((p as i32 % 5) * 3, (p as i32 / 5) * 3)).collect();
    let mut patch_shape: BitImage = BitImage::default();
    for p in 0..225 {
      let x = p % 15;
      let y = p / 15;
      if is_point_in_path(x, y, &patch_type) {
        patch_shape.set(x, y, true);
      }
    }
    ret_val[i] = patch_shape;
  }
  ret_val
}

// https://en.wikipedia.org/wiki/Even%E2%80%93odd_rule
fn is_point_in_path(x: u8, y: u8, path: &[(i32, i32)]) -> bool {
  let num = path.len();
  let mut j: usize = num as usize - 1;
  let mut c = false;
  let x = x as i32;
  let y = y as i32;
  for (i, (vx, vy)) in path.iter().copied().enumerate() {
    let wx = path[j].0;
    let wy = path[j].1;
    if ((vy > y) != (wy > y)) &&
       (x < vx + (wx - vx) * (y - vy) /
                 (wy - vy)) {
        c = !c;
    }
    j = i;
  }
  c
}

