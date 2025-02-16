use macroquad::prelude::*;
use crate::graphing::Datum;

pub type Days = u32;

#[derive(Copy, Clone)]
pub struct Virus {
  pub infectivity: f32,
  pub lethality: f32,
  pub incubation: Days,
  pub duration: Days,
  pub radius: f32,
}

#[derive(Copy, Clone)]
pub enum Status {
  Healthy,
  Incubating(Virus, Days),
  Infected(Virus, Days),
  Recovered,
  Dead,
}

#[derive(Clone)]
enum Waypoint {
  Delay(u32),
  Point(Vec2),
}

#[derive(Clone)]
pub struct Entity {
  pub pos: Vec2,
  waypoints: Vec<Waypoint>,
  pub status: Status,
  pub vaccinated: bool,
  pub house_id: usize,
  pub workplace_id: usize,
}

#[derive(Clone)]
pub enum BuildingKind {
  House,
  Workplace,
  Store,
}

#[derive(Clone)]
pub struct Building {
  pub block: IVec2,
  pub kind: BuildingKind,
}

#[derive(Clone)]
pub struct Simulation {
  pub entities: Vec<Entity>,
  pub buildings: Vec<Building>,
  pub store_ids: Vec<usize>,
  pub days: Days,
  pub morning: bool,
  pub quarantine: bool,
  pub data: Vec<Datum>,
}

const ENTITY_VELOCITY: f32 = 3.;
const GRID_EPSILON: f32 = 2.;
const GRID_EPSILON_SQ: f32 = GRID_EPSILON * GRID_EPSILON;
const VACCINE_EFFECTIVITY: f32 = 0.8;
pub const CELL_WIDTH: f32 = 100.;
pub const ENTITY_RADIUS: f32 = 2.;

pub fn block_to_pos(block: IVec2) -> Vec2 {
  (block.as_vec2() + 0.5) * CELL_WIDTH
}

pub fn recenter(pos: Vec2) -> Vec2 {
  ((pos / CELL_WIDTH).floor() + 0.5) * CELL_WIDTH
}

pub fn snap(pos: Vec2) -> Vec2 {
  (pos / CELL_WIDTH).floor() * CELL_WIDTH
}

pub fn fuzz(pos: Vec2) -> Vec2 {
  pos + vec2(rand::gen_range(-0.3, 0.3), rand::gen_range(-0.3, 0.3)) * CELL_WIDTH
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
    entity.status = Status::Incubating(virus, virus.incubation);
  }
}

impl Entity {
  pub fn new(pos: Vec2, virus: Option<Virus>, vaccinated: bool, house_id: usize, workplace_id: usize) -> Self {
    Entity { 
      pos, 
      waypoints: Vec::new(),
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
      Status::Incubating(virus, days) => {
        self.status = if days == 0 { 
          Status::Infected(virus, virus.duration) 
        } else { 
          Status::Incubating(virus, days - 1) 
        };
      }
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

    if self.waypoints.is_empty() {
      return;
    } 

    let waypoint = &self.waypoints[0];
    match waypoint {
      Waypoint::Delay(delay) => {
        if *delay == 0 {
          self.waypoints.remove(0);
        } else {
          self.waypoints[0] = Waypoint::Delay(delay - 1);
        }
      },
      Waypoint::Point(point) => {
        let axis = *point - self.pos;
        if axis.length_squared() <= GRID_EPSILON_SQ {
          self.pos = *point;
          self.waypoints.remove(0);
        } else {
          self.pos += axis.normalize_or_zero() * ENTITY_VELOCITY;
        }
      },
    }
  }

  pub fn walk_to(&mut self, to: Vec2) {
    self.walk(self.pos, to);
  }

  pub fn walk(&mut self, from: Vec2, to: Vec2) {
    let via_corner = snap(from + rand::gen_range(0.2, 0.8) * (to - from));
    let to_corner = to - 0.5 * CELL_WIDTH * (to - via_corner).signum();
    let from_center = recenter(from);
    let from_corner = from_center + 0.5 * CELL_WIDTH * (via_corner - from_center).signum();

    self.waypoints.push(Waypoint::Delay(60 + rand::gen_range(0, 30)));
    self.waypoints.push(Waypoint::Point(from));
    self.waypoints.extend(manhattan(from_corner, via_corner).into_iter().map(Waypoint::Point));
    self.waypoints.pop();
    self.waypoints.extend(manhattan(via_corner, to_corner).into_iter().map(Waypoint::Point));
    self.waypoints.push(Waypoint::Point(fuzz(to)));
  }
}

impl Building {
  pub fn new(block: IVec2, kind: BuildingKind) -> Self {
    Building { block, kind }
  }
}

impl Simulation {
  pub fn new(quarantine: bool) -> Self {
    Simulation { 
      entities: Vec::new(), 
      buildings: Vec::new(),
      store_ids: Vec::new(),
      days: 0,
      morning: false,
      quarantine,
      data: Vec::new(),
    }
  }

  pub fn spawn(&mut self, entity: Entity) -> usize {
    self.entities.push(entity);
    self.entities.len() - 1
  }

  pub fn build(&mut self, building: Building) -> usize {
    let id = self.buildings.len();
    if matches!(building.kind, BuildingKind::Store) {
      self.store_ids.push(id);
    }
    self.buildings.push(building);
    id
  }

  pub fn day(&mut self) {
    let mut datum: Datum = Default::default();
    for entity in self.entities.iter_mut() {
      match entity.status {
        Status::Healthy if entity.vaccinated => datum.vaccinated += 1,
        Status::Healthy => datum.healthy += 1,
        Status::Incubating(_, _) => datum.incubating += 1,
        Status::Infected(_, _) => datum.infected += 1,
        Status::Recovered => datum.recovered += 1,
        Status::Dead => datum.dead += 1,
      }
      entity.day();
    } 
    self.data.push(datum);
    self.days += 1;
  }

  pub fn update(&mut self) {
    let commuted = self.entities.iter().all(|e| e.waypoints.is_empty() || matches!(e.status, Status::Dead));
    if commuted {
      self.morning = !self.morning;
      if self.morning {
        self.day();
      }
      for entity in self.entities.iter_mut() {
        match entity.status {
          Status::Infected(_, days) => {
            if self.quarantine || days > 2 {
              continue;
            }
          },
          _ => {},
        }

        if self.morning {
          entity.walk_to(block_to_pos(self.buildings[entity.workplace_id].block));
        } else {
          if rand::gen_range(0., 1.) < 0.5 {
            let store_id = self.store_ids[rand::gen_range(0, self.store_ids.len())];
            entity.walk_to(block_to_pos(self.buildings[store_id].block));
            let Some(Waypoint::Point(store_pos)) = entity.waypoints.last() else {
              panic!("end of walk to store should be some point");
            };
            entity.walk(*store_pos, block_to_pos(self.buildings[entity.house_id].block));
          } else {
            entity.walk_to(block_to_pos(self.buildings[entity.house_id].block));
          }
        }
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
          (Status::Infected(virus, _) | Status::Incubating(virus, _), Status::Healthy) => contact(second, virus, first.pos),
          (Status::Healthy, Status::Infected(virus, _) | Status::Incubating(virus, _)) => contact(first, virus, second.pos),
          _ => {},
        }
      }
    }
  }
}

