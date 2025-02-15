use std::time::{SystemTime, UNIX_EPOCH};

use macroquad::prelude::*;

pub mod simulation;

use simulation::*;

const BACKGROUND: Color = Color::new(0.08, 0.08, 0.1, 1.0);
const WIDTH: f32 = 800.;
const HEIGHT: f32 = 800.;

fn conf() -> Conf {
  Conf {
    window_width: WIDTH as i32,
    window_height: HEIGHT as i32,
    window_title: "Contagion".to_string(),
    sample_count: 8,
    ..Default::default()
  }
}

fn entity_color(entity: &Entity) -> Color {
  match entity.status {
    Status::Healthy => if entity.vaccinated { Color::new(0.3, 0.9, 0.75, 1.) } else { GREEN },
    Status::Infected(_, _) => RED,
    Status::Recovered => BLUE,
    Status::Dead => GRAY,
  }
}

fn building_color(kind: &BuildingKind) -> Color {
  match kind {
    BuildingKind::House => BROWN,
    BuildingKind::Workplace => GRAY,
  }
}

#[macroquad::main(conf)]
async fn main() {
  rand::srand(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);
  let mut simulation = Simulation::new();
  let mut entity_ids = Vec::new();
  let mut house_ids = Vec::new();
  let mut workplace_ids = Vec::new();

  for x in 0..8 {
    let workplace_id = simulation.build(Building::new(ivec2(x, 7), BuildingKind::Workplace));
    workplace_ids.push(workplace_id);
    for y in 0..2 {
      let house_id = simulation.build(Building::new(ivec2(x, y), BuildingKind::House));
      house_ids.push(house_id);
    }
  }

  for house_id in house_ids.iter() {
    for _ in 0..rand::gen_range(1, 5) {
      let pos = fuzz(block_to_pos(simulation.buildings[*house_id].block));
      let workplace_id = workplace_ids[rand::gen_range(0, workplace_ids.len())];
      let vaccinated = rand::gen_range(0., 1.) < 0.95;
      let entity_id = simulation.spawn(Entity::new(pos, None, vaccinated, *house_id, workplace_id));
      entity_ids.push(entity_id); 
    } 
  }

  let plague = Virus::new(0.07, 0., 0.01, 5, 13.); // 0.01
  let i = rand::gen_range(0, entity_ids.len());
  simulation.entities[entity_ids[i]].status = Status::Infected(plague, plague.duration);

  loop {
    // let dt = get_frame_time();
    simulation.update();

    clear_background(BACKGROUND);

    // grid lines
    for i in 1..=7 {
      draw_line(0., CELL_SIZE * i as f32, WIDTH, CELL_SIZE * i as f32, 0.3, DARKGRAY);
      draw_line(CELL_SIZE * i as f32, 0., CELL_SIZE * i as f32, HEIGHT, 0.3, DARKGRAY);
    }

    // buildings
    for building in simulation.buildings.iter() {
      let corner = (building.block.as_vec2() + 0.15) * CELL_SIZE;
      let color = building_color(&building.kind);
      draw_rectangle(corner.x, corner.y, 0.7 * CELL_SIZE, 0.7 * CELL_SIZE, Color { a: 0.05, ..color }); 
      draw_rectangle_lines(corner.x, corner.y, 0.7 * CELL_SIZE, 0.7 * CELL_SIZE, 2., color); 
    }

    // entity contagious ranges
    for entity in simulation.entities.iter() {
      match entity.status {
        Status::Infected(virus, _) => {
          let color = Color {
            a: 0.05,
            ..entity_color(&entity)
          };
          draw_circle(entity.pos.x, entity.pos.y, virus.radius, color);
        },
        _ => {},
      }
    }
    

    // entities
    for entity in simulation.entities.iter() {
      draw_circle(entity.pos.x, entity.pos.y, ENTITY_RADIUS, entity_color(&entity));
    }

    next_frame().await;
  }
}
