/*!
Identicon renderer
*/

include!("data.rs");
include!(concat!(env!("OUT_DIR"), "/data.rs"));

use std::io::Cursor;
use image::{Rgb, RgbImage, SubImage, DynamicImage, ImageOutputFormat};

pub fn to_png(image: RgbImage) -> Vec<u8> {
  let mut ret_val = Vec::new();
  DynamicImage::ImageRgb8(image)
    .write_to(&mut Cursor::new(&mut ret_val), ImageOutputFormat::Png).expect("vec is infallibe");
  ret_val
}

pub fn render(code: u32) -> RgbImage {
  let size = 30;

  // decode the code into parts
  // bit 0-1: middle patch type
  // bit 2: middle invert
  // bit 3-6: corner patch type
  // bit 7: corner invert
  // bit 8-9: corner turns
  // bit 10-13: side patch type
  // bit 14: side invert
  // bit 15: corner turns
  // bit 16-20: blue color component
  // bit 21-26: green color component
  // bit 27-31: red color component
  let middle_type = CENTER_PATCH_TYPES[code as usize & 0x3];
  let middle_invert = ((code >> 2) & 0x1) != 0;
  let corner_type = (code >> 3) & 0xf;
  let corner_invert = ((code >> 7) & 0x1) != 0;
  let mut corner_turn = (code >> 8) & 0x3;
  let side_type = (code >> 10) & 0x0f;
  let side_invert = ((code >> 14) & 0x1) != 0;
  let mut side_turn = (code >> 15) & 0x3;
  let blue = (code >> 16) & 0x1f;
  let green = (code >> 21) & 0x1f;
  let red = (code >> 27) & 0x1f;

  // color components are used at top of range for color difference
  // use white background for now
  let background_color: Rgb<u8> = Rgb([255, 255, 255]);
  let fill_color: Rgb<u8> = Rgb([(red << 3) as u8, (green << 3) as u8, (blue << 3) as u8]);

  // outline shapes with a noticeable color (complementary will do) if
  // shape color and background color are too similar (measured by color
  // distance)
  let stroke_color = if get_color_distance(fill_color, background_color) < 32.0 {
    Some(get_complementary_color(fill_color))
  } else {
    None
  };

  let mut target_image = RgbImage::new(size, size);

  let block_size = size / 3;
  let block_size_2 = size / 2;

  // fill with white background
  for p in target_image.pixels_mut() {
    *p = Rgb([255, 255, 255]);
  }

  // middle patch
  draw_patch(&mut target_image, block_size, block_size, middle_type,
      0, middle_invert, fill_color, stroke_color, background_color);

  // side patches, starting from top and moving clock-wise
  draw_patch(&mut target_image, block_size, 0, side_type, side_turn, side_invert,
      fill_color, stroke_color, background_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size_2, block_size, side_type, side_turn,
      side_invert, fill_color, stroke_color, background_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size, block_size_2, side_type, side_turn,
      side_invert, fill_color, stroke_color, background_color);
  side_turn += 1;
  draw_patch(&mut target_image, 0, block_size, side_type, side_turn, side_invert,
      fill_color, stroke_color, background_color);

  // corner patches, starting from top left and moving clock-wise

  draw_patch(&mut target_image, 0, 0, corner_type, corner_turn, corner_invert,
      fill_color, stroke_color, background_color);
  corner_turn += 1;
  draw_patch(&mut target_image, block_size_2, 0, corner_type, corner_turn,
      corner_invert, fill_color, stroke_color, background_color);
  corner_turn += 1;
  draw_patch(&mut target_image, block_size_2, block_size_2, corner_type,
      corner_turn, corner_invert, fill_color, stroke_color, background_color);
  corner_turn += 1;
  draw_patch(&mut target_image, 0, block_size_2, corner_type, corner_turn,
      corner_invert, fill_color, stroke_color, background_color);

  target_image
}

fn draw_patch(target_image: &mut RgbImage, x: u32, y: u32, patch: u32, turn: u32, mut invert: bool, fill_color: Rgb<u8>, stroke_color: Option<Rgb<u8>>, background_color: Rgb<u8>) {

  let patch = patch % PATCH_TYPES.len() as u32;
  let turn = turn % 4;

  if PATCH_FLAGS[patch as usize] & PATCH_INVERTED != 0 {
    invert = !invert;
  }

  let shape = PATCH_SHAPES[patch as usize];

  let rendered_shape = SubImage::new(target_image, x, y, 10, 10);
  let mut rendered_shape = rendered_shape.to_image();

  let fill = if invert { background_color } else { fill_color };
  let background = if invert { fill_color } else { background_color };
  for (x, y, pixel) in rendered_shape.enumerate_pixels_mut() {
    let p = match turn {
      0 => x + (y * 10),
      1 => y + ((9 - x) * 10),
      2 => (9 - x) + ((9 - y) * 10),
      3 => (9 - y) + (x * 10),
      _ => unreachable!(),
    };
    let on = (1 << p) & shape != 0;
    *pixel = if on { fill } else { background };
  }

  if let Some(stroke) = stroke_color {
    let outline = PATCH_OUTLINES[patch as usize];
    for (x, y, pixel) in rendered_shape.enumerate_pixels_mut() {
      let p = match turn {
        0 => x + (y * 10),
        1 => y + ((9 - x) * 10),
        2 => (9 - x) + ((9 - y) * 10),
        3 => (9 - y) + (x * 10),
        _ => unreachable!(),
      };
      let on = (1 << p) & outline != 0;
      if on { *pixel = stroke; }
    }
  }
}

fn get_color_distance(a: Rgb<u8>, b: Rgb<u8>) -> f64 {
  let b1 = a[2] as f64;
  let g1 = a[1] as f64;
  let r1 = a[0] as f64;
  let b2 = b[2] as f64;
  let g2 = b[1] as f64;
  let r2 = b[0] as f64;
  fn sq(x: f64) -> f64 {
    x*x
  }
  (sq(r1+r2)+sq(g1+g2)+sq(b1+b2)).sqrt()
}

fn get_complementary_color(color: Rgb<u8>) -> Rgb<u8> {
  let b = color[2];
  let g = color[1];
  let r = color[0];
  Rgb([255 - r, 255 - g, 255 - b])
}

