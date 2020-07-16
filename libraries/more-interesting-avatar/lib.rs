/*!
Identicon renderer
*/

use image::{Rgb, RgbImage};

const CENTER_PATCH_TYPES: [u32; 4] = [0, 4, 8, 15];

const patch0: &[u8] = &[ 0, 4, 24, 20 ];

const patch1: &[u8] = &[ 0, 4, 20 ];

const patch2: &[u8] = &[ 2, 24, 20 ];

const patch3: &[u8] = &[ 0, 2, 20, 22 ];

const patch4: &[u8] = &[ 2, 14, 22, 10 ];

const patch5: &[u8] = &[ 0, 14, 24, 22 ];

const patch6: &[u8] = &[ 2, 24, 22, 13, 11, 22, 20 ];

const patch7: &[u8] = &[ 0, 14, 22 ];

const patch8: &[u8] = &[ 6, 8, 18, 16 ];

const patch9: &[u8] = &[ 4, 20, 10, 12, 2 ];

const patch10: &[u8] = &[ 0, 2, 12, 10 ];

const patch11: &[u8] = &[ 10, 14, 22 ];

const patch12: &[u8] = &[ 20, 12, 24 ];

const patch13: &[u8] = &[ 10, 2, 12 ];

const patch14: &[u8] = &[ 0, 2, 10 ];

const PATCH_TYPES: [&[u8]; 15] = [PATCH0, PATCH1, PATCH2,
			PATCH3, PATCH4, PATCH5, PATCH6, PATCH7, PATCH8, PATCH9, PATCH10,
			PATCH11, PATCH12, PATCH13, PATCH14, PATCH0];

pub fn render(code: u32, size: u32) -> RgbImage {
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
  let middle_type = CENTER_PATCH_TYPES[code & 0x3];
  let middle_invert = ((code >> 2) & 0x1) != 0;
  let corner_type = (code >> 3) & 0xf;
  let corner_invert = ((code >> 7) & 0x1) != 0;
  let corner_turn = (code >> 8) & 0x3;
  let side_type = (code >> 10) & 0x0f;
  let side_invert = ((code >> 14) & 0x1) != 0;
  let mut side_turn = (code >> 15) & 0x3;
  let blue = (code >> 16) & 0x1f;
  let green = (code >> 21) & 0x1f;
  let red = (code >> 27) & 0x1f;

  // color components are used at top of range for color difference
  // use white background for now
  let fill_color: Rgb<u8> = Rgb([(red << 3) as u8, (green << 3) as u8, (blue << 3) as u8]);

  // outline shapes with a noticeable color (complementary will do) if
  // shape color and background color are too similar (measured by color
  // distance)
  let stroke_color = None;
  if get_color_distance(fill_color, background_color) < 32.0 {
    stroke_color = Some(get_complementary_color(fill_color));
  }

  let mut target_image = RgbImage::new(size, size);

  let block_size = size / 3.0;
  let block_size_2 = size / 2.0;

  // fill with white background
  for p in target_image.pixels_mut() {
    *p = Rgb([255, 255, 255]);
  }

  // middle patch
  draw_patch(&mut target_image, block_size, block_size, block_size, middle_type,
      0, middle_invert, fill_color, stroke_color);

  // side patches, starting from top and moving clock-wise
  draw_patch(&mut target_image, block_size, 0, block_size, side_type, side_turn, side_invert,
      fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size2, block_size, block_size, side_type, side_turn,
      side_invert, fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size, block_size2, block_size, side_type, side_turn,
      side_invert, fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, 0, block_size, block_size, side_type, side_turn, side_invert,
      fill_color, stroke_color);
  side_turn += 1;

  // corner patches, starting from top left and moving clock-wise

  draw_patch(&mut target_image, 0, 0, block_size, corner_type, corner_turn, corner_invert,
      fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size2, 0, block_size, corner_type, corner_turn,
      corner_invert, fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, block_size2, block_size2, block_size, corner_type,
      corner_turn, corner_invert, fill_color, stroke_color);
  side_turn += 1;
  draw_patch(&mut target_image, 0, block_size2, block_size, corner_type, corner_turn,
      corner_invert, fill_color, stroke_color);
  side_turn += 1;

  return target_image;
}

fn draw_patch(target_image: &mut RgbImage, x: f64, y: f64, size: f64, patch: i32, turn: i32, mut invert: bool, fill_color: Rgb<u8>, stroke_color: Option<Rgb<u8>>) {
  assert!(patch >= 0);
  assert!(turn >= 0);

  patch %= PATCH_TYPES.len();
  turn %= 4;

  if patch_flags[patch] & PATCH_INVERTED != 0 {
    invert = !invert;
  }

  //
}

