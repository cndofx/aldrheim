use roxmltree::Node;

use crate::asset_manager::vfx::{SpreadType, VisualEffectProperty};

#[derive(Debug)]
pub struct ContinuousEmitter {
    pub name: String, // names arent unique so these cant be used as a hashmap key
    pub particles_per_second: VisualEffectProperty,

    pub spread_type: SpreadType,
    pub spread_arc_horizontal_angle_degrees: VisualEffectProperty,
    pub spread_arc_horizontal_angle_dist: VisualEffectProperty,
    pub spread_arc_vertical_angle_degrees_min: VisualEffectProperty,
    pub spread_arc_vertical_angle_degrees_max: VisualEffectProperty,
    pub spread_arc_vertical_angle_dist: VisualEffectProperty,
    pub spread_cone_angle_degrees: VisualEffectProperty,
    pub spread_cone_angle_dist: VisualEffectProperty,
    pub position_x: VisualEffectProperty,
    pub position_y: VisualEffectProperty,
    pub position_z: VisualEffectProperty,
    pub position_offset_x: VisualEffectProperty,
    pub position_offset_y: VisualEffectProperty,
    pub position_offset_z: VisualEffectProperty,
    pub velocity_min: VisualEffectProperty,
    pub velocity_max: VisualEffectProperty,
    pub velocity_dist: VisualEffectProperty,
    pub drag: VisualEffectProperty,
    pub gravity: VisualEffectProperty,
    pub rotation_degrees_min: VisualEffectProperty,
    pub rotation_degrees_max: VisualEffectProperty,
    pub rotation_speed_degrees_min: VisualEffectProperty,
    pub rotation_speed_degrees_max: VisualEffectProperty,
    pub rotation_ccw_chance: VisualEffectProperty,
    pub size_start_min: VisualEffectProperty,
    pub size_start_max: VisualEffectProperty,
    pub size_start_dist: VisualEffectProperty,
    pub size_end_min: VisualEffectProperty,
    pub size_end_max: VisualEffectProperty,
    pub size_end_dist: VisualEffectProperty,
    pub lifetime_min: VisualEffectProperty,
    pub lifetime_max: VisualEffectProperty,
    pub lifetime_dist: VisualEffectProperty,

    pub additive_blend: bool,
    pub hsv: bool,
    pub colorize: bool,
    pub hue_min: VisualEffectProperty,
    pub hue_max: VisualEffectProperty,
    pub hue_dist: VisualEffectProperty,
    pub saturation_min: VisualEffectProperty,
    pub saturation_max: VisualEffectProperty,
    pub saturation_dist: VisualEffectProperty,
    pub value_min: VisualEffectProperty,
    pub value_max: VisualEffectProperty,
    pub value_dist: VisualEffectProperty,
    pub alpha_min: VisualEffectProperty,
    pub alpha_max: VisualEffectProperty,
    pub alpha_dist: VisualEffectProperty,
    pub sprite: u8,
}

impl ContinuousEmitter {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let name = node.attribute("name").ok_or_else(|| {
            anyhow::anyhow!("expected <ContinuousEmitter> node to have a 'name' attribute")
        })?;

        let mut particles_per_second: Option<VisualEffectProperty> = None;
        let mut spread_type: Option<SpreadType> = None;
        let mut spread_arc_horizontal_angle_degrees: Option<VisualEffectProperty> = None;
        let mut spread_arc_horizontal_angle_dist: Option<VisualEffectProperty> = None;
        let mut spread_arc_vertical_angle_degrees_min: Option<VisualEffectProperty> = None;
        let mut spread_arc_vertical_angle_degrees_max: Option<VisualEffectProperty> = None;
        let mut spread_arc_vertical_angle_dist: Option<VisualEffectProperty> = None;
        let mut spread_cone_angle_degrees: Option<VisualEffectProperty> = None;
        let mut spread_cone_angle_dist: Option<VisualEffectProperty> = None;
        let mut position_x: Option<VisualEffectProperty> = None;
        let mut position_y: Option<VisualEffectProperty> = None;
        let mut position_z: Option<VisualEffectProperty> = None;
        let mut position_offset_x: Option<VisualEffectProperty> = None;
        let mut position_offset_y: Option<VisualEffectProperty> = None;
        let mut position_offset_z: Option<VisualEffectProperty> = None;
        let mut velocity_min: Option<VisualEffectProperty> = None;
        let mut velocity_max: Option<VisualEffectProperty> = None;
        let mut velocity_dist: Option<VisualEffectProperty> = None;
        let mut drag: Option<VisualEffectProperty> = None;
        let mut gravity: Option<VisualEffectProperty> = None;
        let mut rotation_degrees_min: Option<VisualEffectProperty> = None;
        let mut rotation_degrees_max: Option<VisualEffectProperty> = None;
        let mut rotation_speed_degrees_min: Option<VisualEffectProperty> = None;
        let mut rotation_speed_degrees_max: Option<VisualEffectProperty> = None;
        let mut rotation_ccw_chance: Option<VisualEffectProperty> = None;
        let mut size_start_min: Option<VisualEffectProperty> = None;
        let mut size_start_max: Option<VisualEffectProperty> = None;
        let mut size_start_dist: Option<VisualEffectProperty> = None;
        let mut size_end_min: Option<VisualEffectProperty> = None;
        let mut size_end_max: Option<VisualEffectProperty> = None;
        let mut size_end_dist: Option<VisualEffectProperty> = None;
        let mut lifetime_min: Option<VisualEffectProperty> = None;
        let mut lifetime_max: Option<VisualEffectProperty> = None;
        let mut lifetime_dist: Option<VisualEffectProperty> = None;

        let mut additive_blend: Option<bool> = None;
        let mut hsv: Option<bool> = None;
        let mut colorize: Option<bool> = None;
        let mut hue_min: Option<VisualEffectProperty> = None;
        let mut hue_max: Option<VisualEffectProperty> = None;
        let mut hue_dist: Option<VisualEffectProperty> = None;
        let mut saturation_min: Option<VisualEffectProperty> = None;
        let mut saturation_max: Option<VisualEffectProperty> = None;
        let mut saturation_dist: Option<VisualEffectProperty> = None;
        let mut value_min: Option<VisualEffectProperty> = None;
        let mut value_max: Option<VisualEffectProperty> = None;
        let mut value_dist: Option<VisualEffectProperty> = None;
        let mut alpha_min: Option<VisualEffectProperty> = None;
        let mut alpha_max: Option<VisualEffectProperty> = None;
        let mut alpha_dist: Option<VisualEffectProperty> = None;
        let mut sprite: Option<u8> = None;

        for child in node.children().filter(|n| n.is_element()) {
            // seems like the names are consistently cased but the 'value' attribute is not?
            let child_name = child.tag_name().name();
            let child_value = child
                .attributes()
                .find(|attr| attr.name().eq_ignore_ascii_case("value"))
                .map(|attr| attr.value());

            match child_name {
                "BlendMode" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    match value.to_lowercase().as_str() {
                        "additive" => additive_blend = Some(true),
                        "alpha" => additive_blend = Some(false),
                        _ => {
                            anyhow::bail!(
                                "expected <{child_name}> node 'value' attribute to be 'alpha' or 'additive', got '{value}'"
                            );
                        }
                    }
                }
                "SpreadType" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    match value.to_lowercase().as_str() {
                        "arc" => {
                            spread_type = Some(SpreadType::Arc);
                        }
                        "cone" => {
                            spread_type = Some(SpreadType::Cone);
                        }
                        _ => {
                            anyhow::bail!(
                                "expected <{child_name}> node 'value' attribute value to be 'arc' or 'cone', got '{value}'"
                            );
                        }
                    }
                }
                "SpreadArcHorizontalAngle" => {
                    spread_arc_horizontal_angle_degrees = Some(VisualEffectProperty::read(child)?);
                }
                "SpreadArcHorizontalDistribution" => {
                    spread_arc_horizontal_angle_dist = Some(VisualEffectProperty::read(child)?);
                }
                "SpreadArcVerticalMin" => {
                    spread_arc_vertical_angle_degrees_min =
                        Some(VisualEffectProperty::read(child)?);
                }
                "SpreadArcVerticalMax" => {
                    spread_arc_vertical_angle_degrees_max =
                        Some(VisualEffectProperty::read(child)?);
                }
                "SpreadArcVerticalDistribution" => {
                    spread_arc_vertical_angle_dist = Some(VisualEffectProperty::read(child)?);
                }
                "SpreadConeAngle" => {
                    spread_cone_angle_degrees = Some(VisualEffectProperty::read(child)?);
                }
                "SpreadConeDistribution" => {
                    spread_cone_angle_dist = Some(VisualEffectProperty::read(child)?);
                }
                "PositionX" => {
                    position_x = Some(VisualEffectProperty::read(child)?);
                }
                "PositionY" => {
                    position_y = Some(VisualEffectProperty::read(child)?);
                }
                "PositionZ" => {
                    position_z = Some(VisualEffectProperty::read(child)?);
                }
                "PositionXOffset" => {
                    position_offset_x = Some(VisualEffectProperty::read(child)?);
                }
                "PositionYOffset" => {
                    position_offset_y = Some(VisualEffectProperty::read(child)?);
                }
                "PositionZOffset" => {
                    position_offset_z = Some(VisualEffectProperty::read(child)?);
                }
                "VelocityMin" => {
                    velocity_min = Some(VisualEffectProperty::read(child)?);
                }
                "VelocityMax" => {
                    velocity_max = Some(VisualEffectProperty::read(child)?);
                }
                "VelocityDist" => {
                    velocity_dist = Some(VisualEffectProperty::read(child)?);
                }
                "Drag" => {
                    drag = Some(VisualEffectProperty::read(child)?);
                }
                "Gravity" => {
                    gravity = Some(VisualEffectProperty::read(child)?);
                }
                "RotationMin" => {
                    rotation_degrees_min = Some(VisualEffectProperty::read(child)?);
                }
                "RotationMax" => {
                    rotation_degrees_max = Some(VisualEffectProperty::read(child)?);
                }
                "RotationSpeedMin" => {
                    rotation_speed_degrees_min = Some(VisualEffectProperty::read(child)?);
                }
                "RotationSpeedMax" => {
                    rotation_speed_degrees_max = Some(VisualEffectProperty::read(child)?);
                }
                "RotationPCCW" => {
                    rotation_ccw_chance = Some(VisualEffectProperty::read(child)?);
                }
                "SizeStartMin" => {
                    size_start_min = Some(VisualEffectProperty::read(child)?);
                }
                "SizeStartMax" => {
                    size_start_max = Some(VisualEffectProperty::read(child)?);
                }
                "SizeStartDist" => {
                    size_start_dist = Some(VisualEffectProperty::read(child)?);
                }
                "SizeEndMin" => {
                    size_end_min = Some(VisualEffectProperty::read(child)?);
                }
                "SizeEndMax" => {
                    size_end_max = Some(VisualEffectProperty::read(child)?);
                }
                "SizeEndDist" => {
                    size_end_dist = Some(VisualEffectProperty::read(child)?);
                }
                "LifeTimeMin" => {
                    lifetime_min = Some(VisualEffectProperty::read(child)?);
                }
                "LifeTimeMax" => {
                    lifetime_max = Some(VisualEffectProperty::read(child)?);
                }
                "LifeTimeDistribution" => {
                    lifetime_dist = Some(VisualEffectProperty::read(child)?);
                }
                // TODO: i dont really understand this yet but "HSV" and "ColorControlAlpha" seem to refer to
                // the same thing but with opposite values?
                "HSV" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    match value.to_lowercase().as_str() {
                        "true" => {
                            hsv = Some(true);
                        }
                        "false" => {
                            hsv = Some(false);
                        }
                        _ => {
                            anyhow::bail!(
                                "expected <{child_name}> node 'value' attribute value to be 'true' or 'false', got '{value}'"
                            );
                        }
                    }
                }
                "ColorControlAlpha" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    match value.to_lowercase().as_str() {
                        "true" => {
                            hsv = Some(false);
                        }
                        "false" => {
                            hsv = Some(true);
                        }
                        _ => {
                            anyhow::bail!(
                                "expected <{child_name}> node 'value' attribute value to be 'true' or 'false', got '{value}'"
                            );
                        }
                    }
                }
                "Colorize" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    match value.to_lowercase().as_str() {
                        "true" => {
                            colorize = Some(true);
                        }
                        "false" => {
                            colorize = Some(false);
                        }
                        _ => {
                            anyhow::bail!(
                                "expected <{child_name}> node 'value' attribute value to be 'true' or 'false', got '{value}'"
                            );
                        }
                    }
                }
                "HueMin" => {
                    hue_min = Some(VisualEffectProperty::read(child)?);
                }
                "HueMax" => {
                    hue_max = Some(VisualEffectProperty::read(child)?);
                }
                "HueDistribution" => {
                    hue_dist = Some(VisualEffectProperty::read(child)?);
                }
                "SatMin" => {
                    saturation_min = Some(VisualEffectProperty::read(child)?);
                }
                "SatMax" => {
                    saturation_max = Some(VisualEffectProperty::read(child)?);
                }
                "SatDistribution" => {
                    saturation_dist = Some(VisualEffectProperty::read(child)?);
                }
                "ValueMin" => {
                    value_min = Some(VisualEffectProperty::read(child)?);
                }
                "ValueMax" => {
                    value_max = Some(VisualEffectProperty::read(child)?);
                }
                "ValueDistribution" => {
                    value_dist = Some(VisualEffectProperty::read(child)?);
                }
                "AlphaMin" => {
                    alpha_min = Some(VisualEffectProperty::read(child)?);
                }
                "AlphaMax" => {
                    alpha_max = Some(VisualEffectProperty::read(child)?);
                }
                "AlphaDistribution" => {
                    alpha_dist = Some(VisualEffectProperty::read(child)?);
                }
                "Particle" => {
                    let value = child_value.ok_or_else(|| {
                        anyhow::anyhow!("expected <{child_name}> node to have a 'value' attribute")
                    })?;
                    sprite = Some(value.parse()?);
                }
                "ParticlesPerSecond" => {
                    particles_per_second = Some(VisualEffectProperty::read(child)?);
                }
                _ => {} // TODO
            }
        }

        let Some(additive_blend) = additive_blend else {
            anyhow::bail!("expected <ContinuousEmitter> node to have a <BlendMode> child");
        };

        let Some(spread_type) = spread_type else {
            anyhow::bail!("expected <ContinuousEmitter> node to have a <SpreadType> child");
        };

        let Some(particles_per_second) = particles_per_second else {
            anyhow::bail!("expected <ContinuousEmitter> node to have a <ParticlesPerSecond> child");
        };

        let Some(sprite) = sprite else {
            anyhow::bail!("expected <ContinuousEmitter> node to have a <Particle> child");
        };

        let spread_arc_horizontal_angle_degrees =
            spread_arc_horizontal_angle_degrees.unwrap_or(VisualEffectProperty::Constant(0.0));
        let spread_arc_horizontal_angle_dist =
            spread_arc_horizontal_angle_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let spread_arc_vertical_angle_degrees_min =
            spread_arc_vertical_angle_degrees_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let spread_arc_vertical_angle_degrees_max =
            spread_arc_vertical_angle_degrees_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let spread_arc_vertical_angle_dist =
            spread_arc_vertical_angle_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let spread_cone_angle_degrees =
            spread_cone_angle_degrees.unwrap_or(VisualEffectProperty::Constant(0.0));
        let spread_cone_angle_dist =
            spread_cone_angle_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let position_x = position_x.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_y = position_y.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_z = position_z.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_x = position_offset_x.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_y = position_offset_y.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_z = position_offset_z.unwrap_or(VisualEffectProperty::Constant(0.0));
        let velocity_min = velocity_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let velocity_max = velocity_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let velocity_dist = velocity_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let drag = drag.unwrap_or(VisualEffectProperty::Constant(0.0));
        let gravity = gravity.unwrap_or(VisualEffectProperty::Constant(0.0));
        let rotation_degrees_min =
            rotation_degrees_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let rotation_degrees_max =
            rotation_degrees_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let rotation_speed_degrees_min =
            rotation_speed_degrees_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let rotation_speed_degrees_max =
            rotation_speed_degrees_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let rotation_ccw_chance =
            rotation_ccw_chance.unwrap_or(VisualEffectProperty::Constant(50.0));
        let size_start_min = size_start_min.unwrap_or(VisualEffectProperty::Constant(1.0));
        let size_start_max = size_start_max.unwrap_or(VisualEffectProperty::Constant(1.0));
        let size_start_dist = size_start_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let size_end_min = size_end_min.unwrap_or(VisualEffectProperty::Constant(1.0));
        let size_end_max = size_end_max.unwrap_or(VisualEffectProperty::Constant(1.0));
        let size_end_dist = size_end_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let lifetime_min = lifetime_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let lifetime_max = lifetime_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let lifetime_dist = lifetime_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let hsv = hsv.unwrap_or(false);
        let colorize = colorize.unwrap_or(false);
        let hue_min = hue_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let hue_max = hue_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let hue_dist = hue_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let saturation_min = saturation_min.unwrap_or(VisualEffectProperty::Constant(1.0));
        let saturation_max = saturation_max.unwrap_or(VisualEffectProperty::Constant(1.0));
        let saturation_dist = saturation_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let value_min = value_min.unwrap_or(VisualEffectProperty::Constant(1.0));
        let value_max = value_max.unwrap_or(VisualEffectProperty::Constant(1.0));
        let value_dist = value_dist.unwrap_or(VisualEffectProperty::Constant(1.0));
        let alpha_min = alpha_min.unwrap_or(VisualEffectProperty::Constant(1.0));
        let alpha_max = alpha_max.unwrap_or(VisualEffectProperty::Constant(1.0));
        let alpha_dist = alpha_dist.unwrap_or(VisualEffectProperty::Constant(1.0));

        Ok(ContinuousEmitter {
            name: name.into(),
            particles_per_second,
            spread_type,
            spread_arc_horizontal_angle_degrees,
            spread_arc_horizontal_angle_dist,
            spread_arc_vertical_angle_degrees_min,
            spread_arc_vertical_angle_degrees_max,
            spread_arc_vertical_angle_dist,
            spread_cone_angle_degrees,
            spread_cone_angle_dist,
            position_x,
            position_y,
            position_z,
            position_offset_x,
            position_offset_y,
            position_offset_z,
            velocity_min,
            velocity_max,
            velocity_dist,
            drag,
            gravity,
            rotation_degrees_min,
            rotation_degrees_max,
            rotation_speed_degrees_min,
            rotation_speed_degrees_max,
            rotation_ccw_chance,
            size_start_min,
            size_start_max,
            size_start_dist,
            size_end_min,
            size_end_max,
            size_end_dist,
            lifetime_min,
            lifetime_max,
            lifetime_dist,
            additive_blend,
            hsv,
            colorize,
            hue_min,
            hue_max,
            hue_dist,
            saturation_min,
            saturation_max,
            saturation_dist,
            value_min,
            value_max,
            value_dist,
            alpha_min,
            alpha_max,
            alpha_dist,
            sprite,
        })
    }
}
