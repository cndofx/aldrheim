use std::{
    f32::consts::{PI, TAU},
    rc::Rc,
};

use glam::{Mat4, Quat, Vec3};
use rand::Rng;

use crate::{
    asset_manager::vfx::{ParticleEmitter, SpreadType, VisualEffectAsset},
    renderer::{DrawCommands, pipelines::particles::ParticleInstance},
};

pub struct VisualEffectNode {
    pub effect: Rc<VisualEffectAsset>,
    pub particles: Vec<Particle>,
    /// each item in this list corresponds to the same index in `effect.emitters`
    pub emit_timers: Box<[f32]>,
    pub animation_timer: f32,
    pub animation_fps: u32,
    pub last_translation: Option<Vec3>,
}

impl VisualEffectNode {
    pub fn new(effect: Rc<VisualEffectAsset>) -> Self {
        VisualEffectNode {
            particles: Vec::new(),
            emit_timers: vec![0.0; effect.emitters.len()].into_boxed_slice(),
            animation_timer: 0.0,
            animation_fps: effect.keyframes_per_second,
            last_translation: None,
            effect,
        }
    }

    pub fn update(&mut self, dt: f32, transform: Mat4) {
        let mut rng = rand::rng();

        for timer in self.emit_timers.iter_mut() {
            *timer += dt;
        }

        self.animation_timer += dt;
        if self.animation_timer >= self.effect.duration {
            self.animation_timer -= self.effect.duration;
        }

        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        let delta_translation = if let Some(last_translation) = self.last_translation {
            translation - last_translation
        } else {
            Vec3::ZERO
        };
        self.last_translation = Some(translation);

        // update existing particles
        for i in (0..self.particles.len()).rev() {
            let expired = self.particles[i].update(dt);
            if expired {
                self.particles.swap_remove(i);
            }
        }

        // spawn new particles
        for (emitter_i, emitter) in self.effect.emitters.iter().enumerate() {
            match emitter {
                ParticleEmitter::Continuous(emitter) => {
                    let particles_per_second = emitter
                        .particles_per_second
                        .interpolate(self.animation_timer, self.animation_fps);

                    let particles_to_emit =
                        (particles_per_second * self.emit_timers[emitter_i]) as i32;
                    if particles_to_emit < 1 {
                        continue;
                    } else {
                        self.emit_timers[emitter_i] = 0.0;
                    }

                    let position_x = emitter
                        .position_x
                        .interpolate(self.animation_timer, self.animation_fps);
                    let position_y = emitter
                        .position_y
                        .interpolate(self.animation_timer, self.animation_fps);
                    let position_z = emitter
                        .position_z
                        .interpolate(self.animation_timer, self.animation_fps);
                    let position = Vec3::new(position_x, position_y, position_z);

                    let position_offset_x = emitter
                        .position_offset_x
                        .interpolate(self.animation_timer, self.animation_fps);
                    let position_offset_y = emitter
                        .position_offset_y
                        .interpolate(self.animation_timer, self.animation_fps);
                    let position_offset_z = emitter
                        .position_offset_z
                        .interpolate(self.animation_timer, self.animation_fps);

                    let velocity_min = emitter
                        .velocity_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let velocity_max = emitter
                        .velocity_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let velocity_dist = emitter
                        .velocity_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let drag = emitter
                        .drag
                        .interpolate(self.animation_timer, self.animation_fps);
                    let gravity = emitter
                        .gravity
                        .interpolate(self.animation_timer, self.animation_fps);

                    let rotation_degrees_min = emitter
                        .rotation_degrees_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let rotation_degrees_max = emitter
                        .rotation_degrees_max
                        .interpolate(self.animation_timer, self.animation_fps);

                    let rotation_speed_degrees_min = emitter
                        .rotation_speed_degrees_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let rotation_speed_degrees_max = emitter
                        .rotation_speed_degrees_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let rotation_ccw_chance = emitter
                        .rotation_ccw_chance
                        .interpolate(self.animation_timer, self.animation_fps)
                        / 100.0; // ranges from 0..100, change to 0..1

                    let size_start_min = emitter
                        .size_start_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let size_start_max = emitter
                        .size_start_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let size_start_dist = emitter
                        .size_start_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let size_end_min = emitter
                        .size_end_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let size_end_max = emitter
                        .size_end_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let size_end_dist = emitter
                        .size_end_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let lifetime_min = emitter
                        .lifetime_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let lifetime_max = emitter
                        .lifetime_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let lifetime_dist = emitter
                        .lifetime_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let spread_arc_horizontal_angle_radians = emitter
                        .spread_arc_horizontal_angle_degrees
                        .interpolate(self.animation_timer, self.animation_fps)
                        .to_radians();
                    let spread_arc_horizontal_angle_dist = emitter
                        .spread_arc_horizontal_angle_dist
                        .interpolate(self.animation_timer, self.animation_fps);
                    let spread_arc_vertical_angle_radians_min = emitter
                        .spread_arc_vertical_angle_degrees_min
                        .interpolate(self.animation_timer, self.animation_fps)
                        .to_radians();
                    let spread_arc_vertical_angle_radians_max = emitter
                        .spread_arc_vertical_angle_degrees_max
                        .interpolate(self.animation_timer, self.animation_fps)
                        .to_radians();
                    let spread_arc_vertical_angle_dist = emitter
                        .spread_arc_vertical_angle_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let spread_cone_angle_radians = emitter
                        .spread_cone_angle_degrees
                        .interpolate(self.animation_timer, self.animation_fps)
                        .to_radians();
                    let spread_cone_angle_dist = emitter
                        .spread_cone_angle_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let hue_min = emitter
                        .hue_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let hue_max = emitter
                        .hue_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let hue_dist = emitter
                        .hue_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let saturation_min = emitter
                        .saturation_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let saturation_max = emitter
                        .saturation_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let saturation_dist = emitter
                        .saturation_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let value_min = emitter
                        .value_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let value_max = emitter
                        .value_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let value_dist = emitter
                        .value_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    let alpha_min = emitter
                        .alpha_min
                        .interpolate(self.animation_timer, self.animation_fps);
                    let alpha_max = emitter
                        .alpha_max
                        .interpolate(self.animation_timer, self.animation_fps);
                    let alpha_dist = emitter
                        .alpha_dist
                        .interpolate(self.animation_timer, self.animation_fps);

                    for _ in 0..particles_to_emit {
                        let size_start =
                            random_distribution(size_start_min, size_start_max, size_start_dist);
                        let size_end =
                            random_distribution(size_end_min, size_end_max, size_end_dist);
                        let lifetime =
                            random_distribution(lifetime_min, lifetime_max, lifetime_dist);

                        let velocity =
                            random_distribution(velocity_min, velocity_max, velocity_dist);
                        let velocity = match emitter.spread_type {
                            SpreadType::Arc => {
                                random_direction_in_arc(
                                    rotation,
                                    spread_arc_horizontal_angle_radians,
                                    spread_arc_horizontal_angle_dist,
                                    spread_arc_vertical_angle_radians_min,
                                    spread_arc_vertical_angle_radians_max,
                                    spread_arc_vertical_angle_dist,
                                ) * velocity
                            }
                            SpreadType::Cone => {
                                random_direction_in_cone(
                                    rotation,
                                    spread_cone_angle_radians,
                                    spread_cone_angle_dist,
                                ) * velocity
                            }
                        };
                        // TODO: handle relative velocity

                        let rotation_radians =
                            random_distribution(rotation_degrees_min, rotation_degrees_max, 1.0)
                                .to_radians();

                        let rotation_speed_radians = (random_distribution(
                            rotation_speed_degrees_min,
                            rotation_speed_degrees_max,
                            1.0,
                        ) * random_sign(rotation_ccw_chance))
                        .to_radians();

                        let position_offset_x =
                            position_offset_x * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset_y =
                            position_offset_y * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset_z =
                            position_offset_z * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset =
                            Vec3::new(position_offset_x, position_offset_y, position_offset_z);

                        let hue_rotation =
                            (random_distribution(hue_min, hue_max, hue_dist) * 0.159155 + 0.5)
                                .fract()
                                * TAU
                                - PI;
                        let saturation =
                            random_distribution(saturation_min, saturation_max, saturation_dist);
                        let value = random_distribution(value_min, value_max, value_dist);
                        let alpha = random_distribution(alpha_min, alpha_max, alpha_dist);

                        let particle = Particle {
                            position: translation + position + position_offset,
                            velocity,
                            drag,
                            gravity,
                            rotation: rotation_radians,
                            rotation_speed: rotation_speed_radians,
                            lifetime,
                            lifetime_remaining: lifetime,
                            size_start,
                            size_end,
                            sprite: emitter.sprite,
                            additive: emitter.additive_blend,
                            hsv: emitter.hsv,
                            colorize: emitter.colorize,
                            hue_rotation,
                            saturation,
                            value,
                            alpha,
                        };

                        self.particles.push(particle);
                    }
                }
            }
        }
    }

    pub fn render(&mut self, draw_commands: &mut DrawCommands) {
        if self.particles.is_empty() {
            return;
        }

        let instances = self.particles.iter().map(|particle| {
            let lifetime = 1.0 - (particle.lifetime_remaining / particle.lifetime);
            ParticleInstance {
                position: particle.position,
                lifetime,
                // seems like "size" refers to the distance from the center to a corner?
                // the particle vertices define a 1x1 quad,
                // but a "size" of 1 means a roughly 2x2 quad?
                size: lerp(particle.size_start, particle.size_end, lifetime) * 2.0,
                rotation: particle.rotation,
                sprite: particle.sprite as u32,
                additive: if particle.additive { 1 } else { 0 },
                hsv: if particle.hsv { 1 } else { 0 },
                colorize: if particle.colorize { 1 } else { 0 },
                hue: particle.hue_rotation,
                saturation: particle.saturation,
                value: particle.value,
                alpha: particle.alpha,
            }
        });

        draw_commands.add_particles(instances);
    }
}

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub drag: f32,
    pub gravity: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub lifetime: f32,
    pub lifetime_remaining: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub sprite: u8,
    pub additive: bool,
    pub hsv: bool,
    pub colorize: bool,
    pub hue_rotation: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
}

impl Particle {
    /// returns true if this particle has expired
    pub fn update(&mut self, dt: f32) -> bool {
        self.lifetime_remaining -= dt;
        if self.lifetime_remaining <= 0.0 {
            return true;
        }

        self.velocity = Vec3::new(
            self.velocity.x,
            self.velocity.y + self.gravity * dt,
            self.velocity.z,
        );
        self.velocity *= self.drag.powf(dt);

        self.position += self.velocity * dt;

        self.rotation = wrap_radians(self.rotation + self.rotation_speed * dt);

        false
    }
}

pub fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + ((b - a) * f)
}

pub fn wrap_radians(angle: f32) -> f32 {
    let a = angle.rem_euclid(TAU);
    if a > PI { a - TAU } else { a }
}

fn random_distribution(min: f32, max: f32, dist: f32) -> f32 {
    let base: f32 = rand::rng().random();
    base.powf(dist) * (max - min) + min
}

fn random_sign(negative_chance: f32) -> f32 {
    if rand::rng().random::<f32>() <= negative_chance {
        -1.0
    } else {
        1.0
    }
}

fn random_direction_in_arc(
    orientation: Quat,
    horizontal_angle_radians: f32,
    horizontal_angle_dist: f32,
    vertical_angle_radians_min: f32,
    vertical_angle_radians_max: f32,
    vertical_angle_dist: f32,
) -> Vec3 {
    let h_base = rand::rng().random::<f32>() * 2.0 - 1.0;
    let h_angle =
        h_base.abs().powf(horizontal_angle_dist) * h_base.signum() * horizontal_angle_radians;

    let v_base = rand::rng().random::<f32>() * 2.0 - 1.0;
    let v_angle = (v_base.abs().powf(vertical_angle_dist)
        * v_base.signum()
        * (vertical_angle_radians_max - vertical_angle_radians_min)
        + (vertical_angle_radians_min + vertical_angle_radians_max))
        * 0.5;

    let x = h_angle.sin() * v_angle.cos();
    let y = v_angle.sin();
    let z = h_angle.cos() * v_angle.cos();

    (orientation * Vec3::new(x, y, z)).normalize()
}

fn random_direction_in_cone(orientation: Quat, angle_radians: f32, angle_dist: f32) -> Vec3 {
    let base = rand::rng().random::<f32>();
    let cos_theta = base.powf(angle_dist) * (angle_radians / PI) * 2.0 - 1.0;
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    let azimuth = rand::rng().random::<f32>() * TAU;

    let x = sin_theta * azimuth.cos();
    let y = sin_theta * azimuth.sin();
    let z = -cos_theta;

    (orientation * Vec3::new(x, y, z)).normalize()
}
