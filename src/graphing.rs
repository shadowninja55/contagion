use macroquad::{color::{hsl_to_rgb, rgb_to_hsl}, prelude::*};
use struct_iterable::Iterable;

#[derive(Clone, Copy, Default, Iterable)]
pub struct Datum {
  pub healthy: u32,
  pub vaccinated: u32,
  pub recovered: u32,
  pub incubating: u32,
  pub infected: u32,
  pub dead: u32
}

pub fn draw_graph(data: &Vec<Datum>, width: f32, height: f32) {
  let w = width  / data.len() as f32;
  let mut prev_ys = [height; 6];

  for (i, datum) in data.into_iter().enumerate() {
    let total = datum.iter().map(|(_, v)| v.downcast_ref::<u32>().unwrap()).sum::<u32>() as f32;

    let x = w * i as f32;
    let mut y = 0.;

    for (j, (key, value)) in datum.iter().enumerate() {
      let stroke = match key {
        "healthy" => GREEN,
        "vaccinated" => Color::new(0.3, 0.9, 0.75, 1.),
        "recovered" => BLUE,
        "incubating" => ORANGE,
        "infected" => RED,
        "dead" => GRAY,
        _ => WHITE
      };
      let (h, s, l) = rgb_to_hsl(stroke);
      let background = hsl_to_rgb(h, s, l * 0.2);

      let count = value.downcast_ref::<u32>().unwrap();
      let h = height * (*count as f32) / total;
      let prev_y = if i > 0 { prev_ys[j] } else { y };

      draw_triangle(
        vec2(x + w, y),
        vec2(x, prev_y),
        if y > prev_y { vec2(x, y) } else { vec2(x + w, prev_y) },
        background,
      );
      let rect_y = y.max(prev_y);
      draw_rectangle(x, rect_y, w, height - rect_y, background);
      draw_line(x, prev_y, x + w, y, 1.0, stroke);
      prev_ys[j] = y;
      y += h;
    }
  }
}
