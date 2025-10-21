use std::rc::Rc;

use glam::{Mat4, Quat, Vec3};
use rand::Rng;

use crate::{
    asset_manager::vfx::{ParticleEmitter, SpreadType, VisualEffectAsset},
    renderer::{DrawCommand, RenderContext, Renderable, pipelines::particles::ParticleInstance},
};

pub struct VisualEffectNode {
    pub effect: Rc<VisualEffectAsset>,
    pub particles: Vec<Particle>,
    /// each item in this list corresponds to the same index in `effect.emitters`
    pub emit_timers: Box<[f32]>,
    pub animation_timer: f32,
    pub last_translation: Option<Vec3>,

    pub instance_buffer: Option<wgpu::Buffer>,
    pub instance_buffer_capacity: u64,
}

impl VisualEffectNode {
    pub fn new(effect: Rc<VisualEffectAsset>) -> Self {
        VisualEffectNode {
            particles: Vec::new(),
            emit_timers: vec![0.0; effect.emitters.len()].into_boxed_slice(),
            animation_timer: 0.0,
            last_translation: None,
            effect,

            instance_buffer: None,
            instance_buffer_capacity: 0,
        }
    }

    pub fn update(&mut self, dt: f32, transform: Mat4) {
        let mut rng = rand::rng();

        for timer in self.emit_timers.iter_mut() {
            *timer += dt;
        }

        self.animation_timer += dt as f32;
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
                        .interpolate(self.animation_timer);

                    let particles_to_emit =
                        (particles_per_second * self.emit_timers[emitter_i]) as i32;
                    if particles_to_emit < 1 {
                        continue;
                    } else {
                        self.emit_timers[emitter_i] = 0.0;
                    }

                    let position_x = emitter.position_x.interpolate(self.animation_timer);
                    let position_y = emitter.position_y.interpolate(self.animation_timer);
                    let position_z = emitter.position_z.interpolate(self.animation_timer);
                    let position = Vec3::new(position_x, position_y, position_z);

                    let position_offset_x =
                        emitter.position_offset_x.interpolate(self.animation_timer);
                    let position_offset_y =
                        emitter.position_offset_y.interpolate(self.animation_timer);
                    let position_offset_z =
                        emitter.position_offset_z.interpolate(self.animation_timer);
                    // let position_offset =
                    //     Vec3::new(position_offset_x, position_offset_y, position_offset_z);

                    let velocity_min = emitter.velocity_min.interpolate(self.animation_timer);
                    let velocity_max = emitter.velocity_max.interpolate(self.animation_timer);
                    let velocity_dist = emitter.velocity_dist.interpolate(self.animation_timer);

                    let size_start_min = emitter.size_start_min.interpolate(self.animation_timer);
                    let size_start_max = emitter.size_start_max.interpolate(self.animation_timer);
                    let size_start_dist = emitter.size_start_dist.interpolate(self.animation_timer);

                    let size_end_min = emitter.size_end_min.interpolate(self.animation_timer);
                    let size_end_max = emitter.size_end_max.interpolate(self.animation_timer);
                    let size_end_dist = emitter.size_end_dist.interpolate(self.animation_timer);

                    let lifetime_min = emitter.lifetime_min.interpolate(self.animation_timer);
                    let lifetime_max = emitter.lifetime_max.interpolate(self.animation_timer);
                    let lifetime_dist = emitter.lifetime_dist.interpolate(self.animation_timer);

                    if !matches!(emitter.spread_type, SpreadType::Arc) {
                        todo!("other spread types")
                    }

                    let horizontal_angle_radians = emitter
                        .spread_arc_horizontal_angle_degrees
                        .interpolate(self.animation_timer)
                        .to_radians();
                    let horizontal_angle_dist = emitter
                        .spread_arc_horizontal_angle_dist
                        .interpolate(self.animation_timer);
                    let vertical_angle_radians_min = emitter
                        .spread_arc_vertical_angle_degrees_min
                        .interpolate(self.animation_timer)
                        .to_radians();
                    let vertical_angle_radians_max = emitter
                        .spread_arc_vertical_angle_degrees_max
                        .interpolate(self.animation_timer)
                        .to_radians();
                    let vertical_angle_dist = emitter
                        .spread_arc_vertical_angle_dist
                        .interpolate(self.animation_timer);

                    for _ in 0..particles_to_emit {
                        let size_start =
                            random_distribution(size_start_min, size_start_max, size_start_dist);
                        let size_end =
                            random_distribution(size_end_min, size_end_max, size_end_dist);
                        let lifetime =
                            random_distribution(lifetime_min, lifetime_max, lifetime_dist);

                        let velocity =
                            random_distribution(velocity_min, velocity_max, velocity_dist);
                        let velocity = random_direction_in_arc(
                            rotation,
                            horizontal_angle_radians,
                            horizontal_angle_dist,
                            vertical_angle_radians_min,
                            vertical_angle_radians_max,
                            vertical_angle_dist,
                        ) * velocity;
                        // TODO: handle relative velocity

                        let position_offset_x =
                            position_offset_x * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset_y =
                            position_offset_y * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset_z =
                            position_offset_z * (rng.random::<f32>() * 2.0 - 1.0);
                        let position_offset =
                            Vec3::new(position_offset_x, position_offset_y, position_offset_z);

                        let particle = Particle {
                            position: position + position_offset,
                            velocity,
                            lifetime,
                            lifetime_remaining: lifetime,
                            size_start,
                            size_end,
                            sprite: emitter.sprite,
                        };

                        self.particles.push(particle);
                    }
                }
            }
        }
    }

    // TODO: i wanted to avoid passing the renderer to the scene's render methods, is it possible here?
    pub fn render(
        &mut self,
        transform: Mat4,
        render_context: &RenderContext,
    ) -> Option<DrawCommand> {
        if self.particles.is_empty() {
            return None;
        }

        let instances = self
            .particles
            .iter()
            .map(|particle| {
                let lifetime = 1.0 - (particle.lifetime_remaining / particle.lifetime);
                ParticleInstance {
                    position: particle.position,
                    lifetime,
                    // seems like "size" refers to the distance from the center to a corner?
                    // the particle vertices define a 1x1 quad,
                    // but a "size" of 1 means a roughly 2x2 quad?
                    size: lerp(particle.size_start, particle.size_end, lifetime) * 2.0,
                    sprite: particle.sprite as u32,
                }
            })
            .collect::<Vec<_>>();

        if self.instance_buffer.is_none() || self.instance_buffer_capacity < instances.len() as u64
        {
            let new_capacity = ((instances.len() as f32 * 1.5) as u64).max(100);
            log::debug!(
                "growing vfx instance buffer capacity from {} to {}",
                self.instance_buffer_capacity,
                new_capacity
            );
            let new_buffer = render_context
                .device
                .create_buffer(&wgpu::BufferDescriptor {
                    label: Some("VisualEffectNode Instance Buffer"),
                    size: new_capacity * std::mem::size_of::<ParticleInstance>() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            self.instance_buffer = Some(new_buffer);
            self.instance_buffer_capacity = new_capacity;
        }

        let instance_buffer = self.instance_buffer.as_ref().unwrap();

        render_context
            .queue
            .write_buffer(instance_buffer, 0, bytemuck::cast_slice(&instances));

        Some(DrawCommand {
            renderable: Renderable::VisualEffect(VisualEffectNodeRenderable {
                instance_buffer: instance_buffer.clone(),
                instance_count: instances.len() as u32,
            }),
            bounds: None,
            transform,
        })
    }
}

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub lifetime_remaining: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub sprite: u8,
}

impl Particle {
    /// returns true if this particle has expired
    pub fn update(&mut self, dt: f32) -> bool {
        self.lifetime_remaining -= dt;
        if self.lifetime_remaining <= 0.0 {
            return true;
        }

        self.position += self.velocity * dt;

        false
    }
}

pub struct VisualEffectNodeRenderable {
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: u32,
}

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + ((b - a) * f)
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
    let z = (h_angle.cos() * -1.0) * v_angle.cos();

    orientation * Vec3::new(x, y, z)
}
