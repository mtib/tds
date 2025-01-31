use crate::{
    util::{Point2, Vector2},
    io::tex::{Assets, Sprite},
    obj::{
        player::Player,
        enemy::Enemy,
        health::Health,
        bullet::Bullet,
        weapon::{WeaponInstance, WeaponDrop, WEAPONS},
        pickup::Pickup,
        decoration::DecorationObj,
    }
};
use ggez::{
    Context, GameResult,
    graphics,
    error::GameError,
};

use std::path::Path;
use std::fs::File;
use std::io::{Write, BufRead, BufReader};

use ::bincode;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

#[derive(Debug)]
/// All the objects in the current world
pub struct World {
    pub player: Player,
    pub grid: Grid,
    pub exit: Option<Point2>,
    pub intels: Vec<Point2>,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet<'static>>,
    pub weapons: Vec<WeaponDrop<'static>>,
    pub decorations: Vec<DecorationObj>,
    pub pickups: Vec<Pickup>,
}

impl World {
    pub fn enemy_pickup(&mut self) {
        for enemy in &mut self.enemies {
            let mut dead = None;
            for (w, weapon) in self.weapons.iter().enumerate() {
                if (weapon.pos - enemy.pl.obj.pos).norm() <= 16. {
                    dead = Some(w);
                    break;
                }
            }
            if let Some(i) = dead {
                enemy.pl.wep = Some(WeaponInstance::from_drop(self.weapons.remove(i)));
            }
            let mut deads = Vec::new();
            for (p, pickup) in self.pickups.iter().enumerate() {
                if (pickup.pos - enemy.pl.obj.pos).norm() <= 16. {
                    deads.push(p);
                    break;
                }
            }
            for i in deads.into_iter() {
                let pickup = self.pickups.remove(i);
                pickup.apply(&mut enemy.pl.health);
            }
        }
    }
    pub fn player_pickup(&mut self) {
        let player = &mut self.player;
        if player.wep.is_none() {
            let mut dead = None;
            for (w, weapon) in self.weapons.iter().enumerate() {
                if (weapon.pos - player.obj.pos).norm() <= 16. {
                    dead = Some(w);
                    break;
                }
            }
            if let Some(i) = dead {
                player.wep = Some(WeaponInstance::from_drop(self.weapons.remove(i)));
            }
        }

        let mut deads = Vec::new();
        for (p, pickup) in self.pickups.iter().enumerate() {
            if (pickup.pos - player.obj.pos).norm() <= 16. {
                deads.push(p);
                break;
            }
        }
        for i in deads.into_iter() {
            let pickup = self.pickups.remove(i);
            pickup.apply(&mut player.health);
        }
    }
}

pub struct Statistics {
    pub hits: usize,
    pub misses: usize,
    pub enemies_left: usize,
    pub health_left: Health,
    pub level: Level,
    pub weapon: Option<WeaponInstance<'static>>,
}

include!("material_macro.rs");

mat!{
    MISSING = Missing
    Grass = 0, Grass, false,
    Wall = 1, Wall, true,
    Floor = 2, Floor, false,
    Dirt = 3, Dirt, false,
    Asphalt = 4, Asphalt, false,
    Sand = 5, Sand, false,
    Concrete = 6, Concrete, true,
    WoodFloor = 7, WoodFloor, false,
    Stairs = 8, Stairs, false,
    Missing = 255, Missing, true,
}

#[derive(Debug, Clone)]
pub struct Level {
    pub grid: Grid,
    pub start_point: Option<Point2>,
    pub enemies: Vec<Enemy>,
    pub exit: Option<Point2>,
    pub intels: Vec<Point2>,
    pub pickups: Vec<(Point2, u8)>,
    pub decorations: Vec<DecorationObj>,
    pub weapons: Vec<WeaponDrop<'static>>,
}

impl Level {
    pub fn new(width: u16, height: u16) -> Self {
        Level {
            grid: Grid::new(width, height),
            start_point: None,
            enemies: Vec::new(),
            exit: None,
            intels: Vec::new(),
            pickups: Vec::new(),
            decorations: Vec::new(),
            weapons: Vec::new(),
        }
    }
    pub fn load<P: AsRef<Path>>(path: P) -> GameResult<Self> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut ret = Level::new(0, 0);

        loop {
            let mut buf = String::with_capacity(16);
            reader.read_line(&mut buf)?;
            match &*buf.trim_right() {
                "" => continue,
                "GRD" => ret.grid = bincode::deserialize_from(&mut reader)
                .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "GRID" => {
                    let (w, grid): (usize, Vec<u16>) = bincode::deserialize_from(&mut reader)
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
                    ret.grid = Grid {
                        mats: grid.into_iter().map(|n| Material::from(n as u8)).collect(),
                        width: w as u16
                    }
                }
                "START" => ret.start_point = Some(
                    bincode::deserialize_from(&mut reader)
                        .map(|(x, y)| Point2::new(x, y))
                        .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?
                ),
                "ENEMIES" => ret.enemies = bincode::deserialize_from(&mut reader)
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "POINT GOAL" => ret.exit = Some(bincode::deserialize_from(&mut reader)
                    .map(|(x, y)| Point2::new(x, y))
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?),
                "INTELS" => ret.intels = bincode::deserialize_from(&mut reader)
                    .map(|l: Vec<(f32, f32)>| l.into_iter().map(|(x, y)| Point2::new(x, y)).collect())
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "DECORATIONS" => ret.decorations = bincode::deserialize_from(&mut reader)
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "PICKUPS" => ret.pickups = bincode::deserialize_from(&mut reader)
                    .map(|l: Vec<((f32, f32), u8)>| l.into_iter().map(|((x, y), i)| (Point2::new(x, y), i)).collect())
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "WEAPONS" => ret.weapons = bincode::deserialize_from(&mut reader)
                    .map(|l: Vec<((f32, f32), u8)>| l.into_iter().map(|((x, y), i)| WEAPONS[i as usize].make_drop(Point2::new(x, y))).collect())
                    .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?,
                "END" => break,
                _ => return Err("Bad section".to_string())?
            }
        }

        Ok(ret)
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> GameResult<()> {
        let mut file = File::create(path)?;

        writeln!(file, "GRD")?;
        bincode::serialize_into(&mut file, &self.grid)
            .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        if let Some(start) = self.start_point {
            writeln!(file, "\nSTART")?;
            bincode::serialize_into(&mut file, &(start.x, start.y))
            .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if !self.enemies.is_empty() {
            writeln!(file, "\nENEMIES")?;
            bincode::serialize_into(&mut file, &self.enemies)
            .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if let Some(p) = self.exit {
            writeln!(file, "\nPOINT GOAL")?;
            bincode::serialize_into(&mut file, &(p.x, p.y))
            .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if !self.intels.is_empty() {
            writeln!(file, "\nINTELS")?;
            let intels: Vec<_> = self.intels.iter().map(|p| (p.x, p.y)).collect();
            bincode::serialize_into(&mut file, &intels)
                .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if !self.decorations.is_empty() {
            writeln!(file, "\nDECORATIONS")?;
            bincode::serialize_into(&mut file, &self.decorations)
            .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if !self.pickups.is_empty() {
            writeln!(file, "\nPICKUPS")?;
            let pickups: Vec<_> = self.pickups.iter().map(|&(p, i)| ((p.x, p.y), i)).collect();
            bincode::serialize_into(&mut file, &pickups)
                .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }
        if !self.weapons.is_empty() {
            writeln!(file, "\nWEAPONS")?;
            let pickups: Vec<((f32, f32), u8)> = self.weapons.iter().map(|w| ((w.pos.x, w.pos.y), {
                let mut index = 0;
                for (i, wep) in WEAPONS.iter().enumerate() {
                    if wep.name == w.weapon.name {
                        index = i;
                        break
                    }  
                }
                index as u8
            })).collect();
            bincode::serialize_into(&mut file, &pickups)
                .map_err(|e| GameError::UnknownError(format!("{:?}", e)))?;
        }

        writeln!(file, "\nEND")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid{
    width: u16,
    mats: Vec<Material>,
}

impl Grid {
    pub fn new(width: u16, height: u16) -> Self {
        Grid {
            width,
            mats: vec![Material::Grass; (width*height) as usize],
        }
    }
    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn height(&self) -> u16 {
        self.mats.len() as u16 / self.width
    }
    pub fn widen(&mut self) {
        let width = self.width as usize;
        let height = self.height() as usize;
        self.mats.reserve_exact(height);
        for i in (1..=height).rev().map(|i| i * width) {
            self.mats.insert(i, Material::Grass);
        }
        self.width += 1;
    }
    pub fn thin(&mut self) {
        if self.width <= 1 {
            return
        }
        let width = self.width;
        for i in (1..=self.height()).rev().map(|i| i * width - 1) {
            self.mats.remove(i as usize);
        }
        self.width -= 1;
    }
    pub fn heighten(&mut self) {
        let new_len = self.mats.len() + self.width as usize;
        self.mats.reserve_exact(self.width as usize);
        self.mats.resize(new_len, Material::Grass);
    }
    pub fn shorten(&mut self) {
        let new_len = self.mats.len() - self.width as usize;
        if new_len == 0 {
            return
        }
        self.mats.truncate(new_len);
    }
    #[inline]
    pub fn snap(c: Point2) -> (u16, u16) {
        Self::snap_coords(c.x, c.y)
    }
    #[inline]
    fn idx(&self, x: u16, y: u16) -> usize {
        x.saturating_add(y.saturating_mul(self.width)) as usize
    }
    pub fn snap_coords(x: f32, y: f32) -> (u16, u16) {
        fn db32omin(n: f32) -> u16 {
            if n < 0. {
                std::u16::MAX
            } else {
                (n / 32.) as u16
            }
        }

        (db32omin(x), db32omin(y))
    }
    pub fn get(&self, x: u16, y: u16) -> Option<Material> {
        if x < self.width {
            self.mats.get(self.idx(x, y)).cloned()
        } else {
            None
        }
    }
    #[inline(always)]
    pub fn is_solid_tuple(&self, (x, y): (u16, u16)) -> bool {
        self.is_solid(x, y)
    }
    pub fn is_solid(&self, x: u16, y: u16) -> bool {
        self.get(x, y).map(|m| m.solid()).unwrap_or(true)
    }
    pub fn insert(&mut self, x: u16, y: u16, mat: Material) {
        if x < self.width {
            let i = self.idx(x, y);
            if let Some(m) = self.mats.get_mut(i) {
                *m = mat;
            }
        }
    }
    pub fn ray_cast(&self, from: Point2, dist: Vector2, finite: bool) -> RayCast {
        let dest = from + dist;

        let mut cur = from;
        let (mut gx, mut gy) = Self::snap(cur);
        let x_dir = Direction::new(dist.x);
        let y_dir = Direction::new(dist.y);

        loop {
            if finite && (cur - dest).dot(&dist) / dist.norm() >= 0. {
                break RayCast::Full(dest);
            }

            let mat = self.get(gx, gy);

            if let Some(mat) = mat {
                if mat.solid() {
                    break RayCast::Half(cur);
                }
                if cur.x < 0. || cur.y < 0. {
                    break RayCast::OffEdge(cur); 
                }
            } else {
                break RayCast::OffEdge(cur);
            }

            let nearest_corner = Point2::new(x_dir.on(f32::from(gx) * 32.), y_dir.on(f32::from(gy) * 32.));
            let distance = nearest_corner - cur;

            let time = (distance.x/dist.x, distance.y/dist.y);

            if time.0 < time.1 {
                // Going along x
                cur.x = nearest_corner.x;
                cur.y += time.0 * dist.y;

                gx = if let Some(n) = x_dir.on_u16(gx) {
                    n
                } else {
                    break RayCast::OffEdge(cur);
                }
            } else {
                // Going along y
                cur.y = nearest_corner.y;
                cur.x += time.1 * dist.x;

                gy = if let Some(n) = y_dir.on_u16(gy) {
                    n
                } else {
                    break RayCast::OffEdge(cur);
                }
            }
        }
    }
    pub fn dist_line_circle(line_start: Point2, line_dist: Vector2, circle_center: Point2) -> f32 {
        let c = circle_center - line_start;

        let d_len = line_dist.norm();

        let c_on_d_len = c.dot(&line_dist) / d_len;
        let c_on_d = c_on_d_len / d_len * line_dist;

        let closest_point = if c_on_d_len < 0. {
            // Closest point is start point
            line_start
        } else if c_on_d_len <= d_len {
            // Closest point is betweeen start and end point
            line_start + c_on_d
        } else {
            // Closest point is end point
            line_start + line_dist
        };

        (circle_center - closest_point).norm()
    }
    pub fn draw(&self, ctx: &mut Context, assets: &Assets) -> GameResult<()> {
        for (i, mat) in self.mats.iter().enumerate() {
            let x = f32::from(i as u16 % self.width) * 32.;
            let y = f32::from(i as u16 / self.width) * 32.;

            mat.draw(ctx, assets, x, y)?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
enum Direction {
    Pos,
    Neg,
}

impl Direction {
    #[inline]
    fn new(n: f32) -> Self {
        if n.is_sign_negative() {
            Direction::Neg
        } else {
            Direction::Pos
        }
    }
    #[inline]
    fn on_u16(self, n: u16) -> Option<u16> {
        match self {
            Direction::Pos => Some(n + 1),
            Direction::Neg => n.checked_sub(1),
        }
    }
    #[inline]
    fn on(self, n: f32) -> f32 {
        match self {
            Direction::Pos => n + 32.,
            Direction::Neg => n,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RayCast {
    Full(Point2),
    Half(Point2),
    OffEdge(Point2),
}

impl RayCast {
    pub fn full(self) -> bool {
        match self {
            RayCast::Full(_) => true,
            _ => false,
        }
    }
    pub fn half(self) -> bool {
        match self {
            RayCast::Half(_) => true,
            _ => false,
        }
    }
    pub fn into_point(self) -> Point2 {
        match self {
            RayCast::Full(p) => p,
            RayCast::Half(p) => p,
            RayCast::OffEdge(p) => p,
        }
    }
}