use serde::Deserialize;
use std::{env, fs, time::{SystemTime, UNIX_EPOCH}};
use toml;

use macroquad::prelude::*;

pub mod simulation;
pub mod graphing;

use simulation::*;
use graphing::*;

#[derive(Deserialize)]
struct SimulationConfig {
  cell_count: i32,
  speed: u32,
  vaccination_rate: f32,
  quarantine: bool,
}

#[derive(Deserialize)]
struct VirusConfig {
  infectivity: f32,
  lethality: f32,
  incubation: Days,
  duration: Days,
}

#[derive(Deserialize)]
struct Config {
  simulation: SimulationConfig,
  virus: VirusConfig,
}

const BACKGROUND: Color = Color::new(0.08, 0.08, 0.1, 1.0);
const WIDTH: f32 = 800.;

fn conf() -> Conf {
  Conf {
    window_width: WIDTH as i32,
    window_height: WIDTH as i32,
    window_title: "Contagion".to_string(),
    sample_count: 8,
    ..Default::default()
  }
}

fn entity_color(entity: &Entity) -> Color {
  match entity.status {
    Status::Healthy => if entity.vaccinated { Color::new(0.3, 0.9, 0.75, 1.) } else { GREEN },
    Status::Incubating(_, _) => ORANGE,
    Status::Infected(_, _) => RED,
    Status::Recovered => BLUE,
    Status::Dead => GRAY,
  }
}

fn building_color(kind: &BuildingKind) -> Color {
  match kind {
    BuildingKind::House => BROWN,
    BuildingKind::Workplace => GRAY,
    BuildingKind::Store => Color::new(0.6, 0.3, 0.3, 1.),
  }
}

#[macroquad::main(conf)]
async fn main() {
  rand::srand(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);

  let args = env::args().collect::<Vec<_>>();
  let Some(config_path) = args.get(1) else {
    println!("Config path must be passed as an argument.");
    return;
  };
  let Ok(source) = fs::read_to_string(config_path) else {
    println!("Config file `{config_path}` does not exist.");
    return;
  };
  let config = match toml::from_str::<Config>(&source) {
    Ok(config) => config,
    Err(err) => {
      println!("{err}");
      return;
    },
  };
  let cell_count = config.simulation.cell_count;
  let scale = WIDTH / CELL_WIDTH / cell_count as f32;

  let mut simulation = Simulation::new(config.simulation.quarantine);
  let mut entity_ids = Vec::new();
  let mut house_ids = Vec::new();
  let mut workplace_ids = Vec::new();
  let mut graphing = false;
  let mut paused = false;

  for x in 0..cell_count {
    // houses
    for y in 0..(cell_count / 4) {
      if rand::gen_range(0., 1.) < 0.75 {
        let house_id = simulation.build(Building::new(ivec2(x, y), BuildingKind::House));
        house_ids.push(house_id);
      }
    }

    // workplaces
    for y in (cell_count * 7 / 8)..cell_count {
      if rand::gen_range(0., 1.) < 0.75 {
        let workplace_id = simulation.build(Building::new(ivec2(x, y), BuildingKind::Workplace));
        workplace_ids.push(workplace_id);
      }
    }
  }

  while simulation.store_ids.is_empty() {
    let height = cell_count * 1 / 8;
    for x in 0..cell_count {
      for dy in -height..=height {
        if rand::gen_range(0., 1.) < 0.15 {
          simulation.build(Building::new(ivec2(x, cell_count / 2 + dy), BuildingKind::Store));
        }
      }
    }
  }

  for house_id in house_ids.iter() {
    for _ in 0..rand::gen_range(1, 5) {
      let pos = fuzz(block_to_pos(simulation.buildings[*house_id].block));
      let workplace_id = workplace_ids[rand::gen_range(0, workplace_ids.len())];
      let vaccinated = rand::gen_range(0., 1.) < config.simulation.vaccination_rate;
      let entity_id = simulation.spawn(Entity::new(pos, None, vaccinated, *house_id, workplace_id));
      entity_ids.push(entity_id); 
    } 
  }

  let virus = Virus {
    infectivity: config.virus.infectivity,
    lethality: config.virus.lethality,
    incubation: config.virus.incubation,
    duration: config.virus.duration,
    radius: 13.,
  };

  for _ in 0..2 {
    let i = rand::gen_range(0, entity_ids.len());
    simulation.entities[entity_ids[i]].status = Status::Incubating(virus, virus.incubation);
  }

  let initial_simulation = simulation.clone();

  loop {
    if is_key_pressed(KeyCode::G) {
      graphing = !graphing;
    }
    if is_key_pressed(KeyCode::Space) {
      paused = !paused;
    }
    if is_key_pressed(KeyCode::R) {
      simulation = initial_simulation.clone();
      paused = false;
    }

    if !paused {
      for _ in 0..((1. / scale).ceil() as i32 * config.simulation.speed as i32) {
        simulation.update();
      }
    }

    clear_background(BACKGROUND);

    if graphing {
      draw_graph(&simulation.data, WIDTH, WIDTH); 
    } else {
      // grid lines
      for i in 1..=cell_count {
        draw_line(0., scale * CELL_WIDTH * i as f32, WIDTH, scale * CELL_WIDTH * i as f32, 0.3, DARKGRAY);
        draw_line(scale * CELL_WIDTH * i as f32, 0., scale * CELL_WIDTH * i as f32, WIDTH, 0.3, DARKGRAY);
      }

      // buildings
      for building in simulation.buildings.iter() {
        let corner = scale * (building.block.as_vec2() + 0.15) * CELL_WIDTH;
        let color = building_color(&building.kind);
        draw_rectangle(corner.x, corner.y, scale * 0.7 * CELL_WIDTH, scale * 0.7 * CELL_WIDTH, Color { a: 0.05, ..color }); 
        draw_rectangle_lines(corner.x, corner.y, scale * 0.7 * CELL_WIDTH, scale * 0.7 * CELL_WIDTH, scale * 2., color); 
      }

      // entity contagious ranges
      for entity in simulation.entities.iter() {
        match entity.status {
          Status::Infected(virus, _) | Status::Incubating(virus, _) => {
            let color = Color {
              a: 0.05,
              ..entity_color(&entity)
            };
            let scaled = scale * entity.pos;
            draw_circle(scaled.x, scaled.y, scale * virus.radius, color);
          },
          _ => {},
        }
      }
      
      // entities
      for entity in simulation.entities.iter() {
        let scaled = scale * entity.pos;
        draw_circle(scaled.x, scaled.y, scale * ENTITY_RADIUS, entity_color(&entity));
      }
    }

    next_frame().await;
  }
}
