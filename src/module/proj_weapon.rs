#[cfg(client)]
use graphics::Context;
#[cfg(client)]
use opengl_graphics::Gl;

use assets::{WEAPON_TEXTURE, LASER_TEXTURE, EXPLOSION_TEXTURE, TextureId};
use battle_state::{BattleContext, TICKS_PER_SECOND};
use module::{IModule, Module, ModuleRef, ModuleBase, ModuleType, ModuleTypeStore, ProjectileWeapon, Weapon};
use net::{ClientId, InPacket, OutPacket};
use ship::{ShipRef, ShipState};
use sim::{SimEvent, SimEventAdder};
use vec::{Vec2, Vec2f};

#[cfg(client)]
use sim::{SimVisuals, SimVisual};
#[cfg(client)]
use sprite_sheet::{SpriteSheet, Loop, PlayOnce, Stay};
#[cfg(client)]
use asset_store::AssetStore;

#[deriving(Encodable, Decodable)]
pub struct ProjectileWeaponModule {
    pub base: ModuleBase,
    
    projectiles: Vec<Projectile>,
    
    target: Option<(ShipRef, ModuleRef)>,
}

impl ProjectileWeaponModule {
    pub fn new(mod_store: &ModuleTypeStore, mod_type: ModuleType) -> Module {
        let projectile = Projectile {
            phase: FireToOffscreen,
            damage: 1,
            hit: false,
            
            fire_tick: 0,
            offscreen_tick: 0,
            hit_tick: 0,
            
            fire_pos: Vec2{x: 0f64, y: 0f64},
            to_offscreen_pos: Vec2{x: 0f64, y: 0f64},
            from_offscreen_pos: Vec2{x: 0f64, y: 0f64},
            hit_pos: Vec2{x: 0f64, y: 0f64},
        };
    
        ProjectileWeapon(ProjectileWeaponModule {
            base: ModuleBase::new(mod_store, mod_type, 2, 2, 3),
            projectiles: Vec::from_elem(4, projectile),
            target: None,
        })
    }
}

impl IModule for ProjectileWeaponModule {
    fn server_preprocess(&mut self, ship_state: &mut ShipState) {
        for projectile in self.projectiles.iter_mut() {
            projectile.hit = true;
        }
    }

    fn before_simulation(&mut self, ship_state: &mut ShipState, events: &mut SimEventAdder) {    
        match self.target {
            Some((ref target_ship, ref target_module)) => {
                for (i, projectile) in self.projectiles.iter_mut().enumerate() {
                    projectile.phase = FireToOffscreen;
                    
                    let start = (i*10) as u32;
                    
                    projectile.fire_tick = start;
                    projectile.offscreen_tick = start + 20;
                    projectile.hit_tick = start + 40;
                    
                    projectile.fire_pos = self.base.get_render_position();
                    projectile.to_offscreen_pos = projectile.fire_pos + Vec2{x: 1500.0, y: 0.0};
                    projectile.from_offscreen_pos = Vec2{x: 1500.0, y: 0.0};
                    projectile.hit_pos = target_module.borrow().get_base().get_render_position();
                    
                    events.add(projectile.hit_tick, box DamageEvent::new(target_ship.clone(), target_module.clone(), 1));
                }
            }
            None => { },
        }
    }
    
    #[cfg(client)]
    fn add_plan_visuals(&self, asset_store: &AssetStore, visuals: &mut SimVisuals, ship: &ShipRef) {
        let mut weapon_sprite = SpriteSheet::new(asset_store.get_sprite_info(WEAPON_TEXTURE));
        weapon_sprite.add_animation(Stay(0.0, 5.0, 0));
    
        visuals.add(ship.borrow().id, 0, box SpriteVisual {
            position: self.base.get_render_position().clone(),
            sprite_sheet: weapon_sprite,
        });
    }
    
    #[cfg(client)]
    fn add_simulation_visuals(&self, asset_store: &AssetStore, visuals: &mut SimVisuals, ship: &ShipRef) {
        let ship_id = ship.borrow().id;
    
        let mut weapon_sprite = SpriteSheet::new(asset_store.get_sprite_info(WEAPON_TEXTURE));
        
        let mut last_weapon_anim_end = 0.0;
    
        match self.target {
            Some((ref target_ship, ref target_module)) => {
                let target_ship_id = target_ship.borrow().id;
            
                for projectile in self.projectiles.iter() {
                    // Set up interpolation stuff to send projectile from weapon to offscreen
                    let start_time = (projectile.fire_tick as f64)/(TICKS_PER_SECOND as f64);
                    let end_time = (projectile.offscreen_tick as f64)/(TICKS_PER_SECOND as f64);
                    let start_pos = projectile.fire_pos.clone();
                    let end_pos = projectile.to_offscreen_pos.clone();
                    
                    let mut laser_sprite = SpriteSheet::new(asset_store.get_sprite_info(LASER_TEXTURE));
                    laser_sprite.add_animation(Loop(0.0, 5.0, 0, 4, 0.05));
                    
                    let weapon_anim_start = start_time;
                    let weapon_anim_end = start_time+0.15;
                    
                    // Add weapon fire animations for this projectile
                    weapon_sprite.add_animation(Stay(last_weapon_anim_end, weapon_anim_start, 0));
                    weapon_sprite.add_animation(PlayOnce(weapon_anim_start, weapon_anim_end, 0, 5));
                    
                    // Set the last end for the next projectile
                    last_weapon_anim_end = weapon_anim_end;
                
                    // Add the simulation visual for projectile leaving
                    visuals.add(ship_id, 1, box LerpVisual {
                        start_time: start_time,
                        end_time: end_time,
                        start_pos: start_pos,
                        end_pos: end_pos,
                        start_rot: 0.0,
                        end_rot: 0.0,
                        sprite_sheet: laser_sprite,
                    });
                    
                    // Set up interpolation stuff to send projectile from offscreen to target
                    let start_time = (projectile.offscreen_tick as f64)/(TICKS_PER_SECOND as f64);
                    let end_time = (projectile.hit_tick as f64)/(TICKS_PER_SECOND as f64);
                    let start_pos = projectile.from_offscreen_pos.clone();
                    let end_pos = projectile.hit_pos.clone();

                    let mut laser_sprite = SpriteSheet::new(asset_store.get_sprite_info(LASER_TEXTURE));
                    laser_sprite.add_animation(Loop(0.0, 5.0, 0, 4, 0.05));
                    
                    // Add the simulation visual for projectile entering target screen
                    visuals.add(target_ship_id, 1, box LerpVisual {
                        start_time: start_time,
                        end_time: end_time,
                        start_pos: start_pos,
                        end_pos: end_pos,
                        start_rot: 0.0,
                        end_rot: 0.0,
                        sprite_sheet: laser_sprite,
                    });
                    
                    // Set up explosion visual
                    let start_time = (projectile.hit_tick as f64)/(TICKS_PER_SECOND as f64);
                    let end_time = start_time + 0.7;
                    
                    let mut explosion_sprite =  SpriteSheet::new(asset_store.get_sprite_info(EXPLOSION_TEXTURE));
                    explosion_sprite.add_animation(PlayOnce(start_time, end_time, 0, 10));
                    
                    visuals.add(target_ship_id, 1, box SpriteVisual {
                        position: projectile.hit_pos.clone(),
                        sprite_sheet: explosion_sprite,
                    });
                }
            },
            None => {},
        }
        
        // Add last stay animation
        weapon_sprite.add_animation(Stay(last_weapon_anim_end, 5.0, 0));
        
        visuals.add(ship_id, 0, box SpriteVisual {
            position: self.base.get_render_position().clone(),
            sprite_sheet: weapon_sprite,
        });
    }
    
    fn after_simulation(&mut self, ship_state: &mut ShipState) {
    }
    
    fn write_plans(&self, packet: &mut OutPacket) {
        match self.target.as_ref() {
            Some(&(ref ship, ref module)) => {
                packet.write(&true).unwrap();
                packet.write(&ship.borrow().id).unwrap();
                packet.write(&module.borrow().get_base().index).unwrap();
            },
            None => {packet.write(&false).unwrap()},
        }
    }
    
    fn read_plans(&mut self, context: &BattleContext, packet: &mut InPacket) {
        let some: bool = packet.read().unwrap();
        if some {
            let ship_id = packet.read().unwrap();
            let module_index: u32 = packet.read().unwrap();
            
            let ship = context.get_ship(ship_id);
            let module = ship.borrow().modules[module_index as uint].clone();
            
            self.target = Some((ship.clone(), module.clone()));
        }
    }
    
    fn write_results(&self, packet: &mut OutPacket) {
    }
    
    fn read_results(&mut self, packet: &mut InPacket) {
    }
    
    fn on_activated(&mut self, ship_state: &mut ShipState) {
    }
    
    fn on_deactivated(&mut self, ship_state: &mut ShipState) {
        println!("weapon deactivated");
    }
    
    fn on_icon_clicked(&mut self) -> bool {
        println!("Clicked a weapon");
        true
    }
    
    fn on_module_clicked(&mut self, ship: &ShipRef, module: &ModuleRef) -> bool {
        self.target = Some((ship.clone(), module.clone()));
        println!("Targeted module");
        false
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[deriving(Encodable, Decodable, Clone)]
enum ProjectilePhase {
    FireToOffscreen,
    OffscreenToTarget,
    Detonate
}

#[deriving(Encodable, Decodable, Clone)]
struct Projectile {
    phase: ProjectilePhase,
    damage: u8,
    hit: bool,
    
    // Simulation times that the projectile changes phases at
    fire_tick: u32,       // Tick that the projectile fires at
    offscreen_tick: u32,  // Tick that the projectile starts travelling from offscreen to target at
    hit_tick: u32,        // Tick that projectile hits target at
    
    // Render stuff

    // Interpolation points for drawing
    fire_pos: Vec2f,
    to_offscreen_pos: Vec2f,
    from_offscreen_pos: Vec2f,
    hit_pos: Vec2f,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct DamageEvent {
    ship: ShipRef,
    module: ModuleRef,
    damage: u8,
}

impl DamageEvent {
    pub fn new(ship: ShipRef, module: ModuleRef, damage: u8) -> DamageEvent {
        DamageEvent {
            ship: ship,
            module: module,
            damage: damage,
        }
    }
}

impl SimEvent for DamageEvent {
    fn apply(&mut self, module: &mut Module) {
        self.ship.borrow_mut().state.deal_damage(self.module.borrow_mut().deref_mut(), self.damage);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

// Basic linear interpolation sim visual
#[cfg(client)]
pub struct LerpVisual {
    start_time: f64,
    end_time: f64,
    start_pos: Vec2f,
    end_pos: Vec2f,
    start_rot: f64,
    end_rot: f64,
    sprite_sheet: SpriteSheet,
}

#[cfg(client)]
impl SimVisual for LerpVisual {
    fn draw(&mut self, context: &Context, gl: &mut Gl, time: f64) {
        if time >= self.start_time && time <= self.end_time {
            let interp = (time-self.start_time)/(self.end_time-self.start_time);
            let pos = self.start_pos + (self.end_pos-self.start_pos)*interp;
            let rot = self.start_rot + (self.start_rot-self.end_rot)*interp;
            self.sprite_sheet.draw(context, gl, pos.x, pos.y, rot, time);
        }
    }
}

// Sprite sheet sim visual
#[cfg(client)]
pub struct SpriteVisual {
    position: Vec2f,
    sprite_sheet: SpriteSheet,
}

#[cfg(client)]
impl SimVisual for SpriteVisual {
    fn draw(&mut self, context: &Context, gl: &mut Gl, time: f64) {
        self.sprite_sheet.draw(context, gl, self.position.x, self.position.y, 0.0, time);
    }
}