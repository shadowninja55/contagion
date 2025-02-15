use macroquad::prelude::*;

type Days = u32;

#[derive(Copy, Clone)]
pub struct Virus {
  pub infectivity: f32,
  pub severity: f32,
  pub lethality: f32,
  pub duration: Days,
  pub radius: f32,
}

#[derive(Copy, Clone)]
pub enum Status {
  Healthy,
  Infected(Virus, Days),
  Recovered,
  Dead,
}

pub struct Entity {
  pub pos: Vec2,
  pub waypoints: Vec<Vec2>,
  pub walk_delay: u32,
  pub status: Status,
  pub vaccinated: bool,
  pub house_id: usize,
  pub workplace_id: usize,
}

pub enum BuildingKind {
  House,
  Workplace,
}

pub struct Building {
  pub block: IVec2,
  pub kind: BuildingKind,
}

pub struct Simulation {
  pub entities: Vec<Entity>,
  pub buildings: Vec<Building>,
  pub days: Days,
  pub morning: bool,
}

pub const ENTITY_RADIUS: f32 = 2.;
pub const CONTAGION_RADIUS: f32 = 13.;
const CONTAGION_RADIUS_SQ: f32 = CONTAGION_RADIUS * CONTAGION_RADIUS;
const VACCINE_EFFECTIVITY: f32 = 0.8;
const GRID_EPSILON: f32 = 2.;
const GRID_EPSILON_SQ: f32 = GRID_EPSILON * GRID_EPSILON;
const ENTITY_VELOCITY: f32 = 3.;
pub const CELL_SIZE: f32 = 100.;

pub fn block_to_pos(block: IVec2) -> Vec2 {
  (block.as_vec2() + 0.5) * CELL_SIZE
}

pub fn recenter(pos: Vec2) -> Vec2 {
  ((pos / CELL_SIZE).floor() + 0.5) * CELL_SIZE
}

pub fn snap(pos: Vec2) -> Vec2 {
  (pos / CELL_SIZE).floor() * CELL_SIZE
}

pub fn fuzz(pos: Vec2) -> Vec2 {
  pos + vec2(rand::gen_range(-0.3, 0.3), rand::gen_range(-0.3, 0.3)) * CELL_SIZE
}

fn manhattan(from: Vec2, to: Vec2) -> Vec<Vec2> {
  vec![from, vec2(to.x, from.y), to]
}

fn contact(entity: &mut Entity, virus: Virus, pos: Vec2) {
  let dist_sq = (entity.pos - pos).length_squared();
  if dist_sq > virus.radius * virus.radius {
    return;
  }
  let r = rand::gen_range(0., 1.);
  if r < virus.infectivity / (2. * virus.radius / ENTITY_VELOCITY) * (1. - if entity.vaccinated { VACCINE_EFFECTIVITY } else { 0. }) {
    entity.status = Status::Infected(virus, virus.duration);
  }
}

impl Virus {
  pub fn new(infectivity: f32, severity: f32, lethality: f32, duration: u32, radius: f32) -> Self {
    Virus { 
      infectivity, 
      severity, 
      lethality, 
      duration: (duration as f32 * rand::gen_range(0.6, 1.4)).round() as u32,
      radius,
    }
  }
}

impl Entity {
  pub fn new(pos: Vec2, virus: Option<Virus>, vaccinated: bool, house_id: usize, workplace_id: usize) -> Self {
    Entity { 
      pos, 
      waypoints: Vec::new(),
      walk_delay: 0,
      status: match virus {
        Some(v) => Status::Infected(v, v.duration),
        None => Status::Healthy,
      },
      vaccinated,
      house_id,
      workplace_id,
    }
  }

  pub fn day(&mut self) {
    match self.status {
      Status::Infected(virus, days) => {
        if days == 0 {
          self.status = Status::Recovered;
        } else {
          let dead = rand::gen_range(0., 1.) < virus.lethality * if self.vaccinated { 0.5 } else { 1. };
          self.status = if dead { Status::Dead } else { Status::Infected(virus, days - 1) };
        }
      },
      _ => {},
    }
  }

  fn update(&mut self) {
    if matches!(self.status, Status::Dead) {
      return;
    }

    if self.walk_delay == 0 {
      if self.waypoints.is_empty() {
        return;
      } 

      let point = self.waypoints[0];
      self.pos += (point - self.pos).normalize_or_zero() * ENTITY_VELOCITY;

      let dist_sq = (point - self.pos).length_squared();
      if dist_sq <= GRID_EPSILON_SQ {
        self.pos = point;
        self.waypoints.remove(0);
      }
    } else {
      self.walk_delay -= 1;
    }
  }


  pub fn walk_to(&mut self, to: Vec2) {
    // let start_center = recenter(from);
    // let start_corner = start_center + 0.5 * CELL_SIZE * (to - start_center).signum();
    // let end_corner = to - 0.5 * CELL_SIZE * (to - start_corner).signum();
    // let midpoint = vec2(end_corner.x, start_corner.y);
    // vec![start_corner, midpoint, end_corner, fuzz(to)]

    let via_corner = snap(self.pos + rand::gen_range(0.2, 0.8) * (to - self.pos));
    let to_corner = to - 0.5 * CELL_SIZE * (to - via_corner).signum();
    let from_center = recenter(self.pos);
    let from_corner = from_center + 0.5 * CELL_SIZE * (via_corner - from_center).signum();

    self.waypoints.push(self.pos);
    self.waypoints.extend(manhattan(from_corner, via_corner));
    self.waypoints.pop();
    self.waypoints.extend(manhattan(via_corner, to_corner));
    self.waypoints.push(fuzz(to));
    self.walk_delay = 60 + rand::gen_range(0, 30);
  }
}

impl Building {
  pub fn new(block: IVec2, kind: BuildingKind) -> Self {
    Building { block, kind }
  }
}

impl Simulation {
  pub fn new() -> Self {
    Simulation { 
      entities: Vec::new(), 
      buildings: Vec::new(),
      days: 0,
      morning: false,
    }
  }

  pub fn spawn(&mut self, entity: Entity) -> usize {
    self.entities.push(entity);
    self.entities.len() - 1
  }

  pub fn build(&mut self, building: Building) -> usize {
    self.buildings.push(building);
    self.buildings.len() - 1
  }

  pub fn day(&mut self) {
    self.days += 1;
    for entity in self.entities.iter_mut() {
      entity.day();
    } 
  }

  pub fn update(&mut self) {
    let commuted = self.entities.iter().all(|e| e.waypoints.is_empty() || matches!(e.status, Status::Dead));
    if commuted {
      self.morning = !self.morning;
      if self.morning {
        self.day();
      }
      for entity in self.entities.iter_mut() {
        let id = if self.morning { entity.workplace_id } else { entity.house_id };
        entity.walk_to(block_to_pos(self.buildings[id].block));
      }
    }
    for entity in self.entities.iter_mut() {
      entity.update();
    }
    self.infections();
  }

  fn infections(&mut self) {
    for i in 0..self.entities.len() {
      for j in (i + 1)..self.entities.len() {
        let (left, right) = self.entities.split_at_mut(i + 1);
        let first = &mut left[i];
        let second = &mut right[j - i - 1];

        // both healthy or both infected
        match (first.status, second.status) {
          (Status::Recovered | Status::Dead, _) | (_, Status::Recovered | Status::Dead) => continue,
          (Status::Infected(_, _), Status::Infected(_, _)) => continue,
          (Status::Healthy, Status::Healthy) => continue,
          _ => {},
        }
        
        let dist_sq = (first.pos - second.pos).length_squared();

        if dist_sq <= CONTAGION_RADIUS_SQ {
          match (first.status, second.status) {
            (Status::Infected(virus, _), Status::Healthy) => contact(second, virus, first.pos),
            (Status::Healthy, Status::Infected(virus, _)) => contact(first, virus, second.pos),
            _ => panic!("impossible status combination"),
          }
        } 
      }
    }
  }
}

