use roxmltree::Node;

use crate::scene::vfx::VisualEffectProperty;

#[derive(Debug)]
pub struct ContinuousEmitter {
    pub name: String, // names arent unique so these cant be used as a hashmap key
    pub position_x: VisualEffectProperty,
    pub position_y: VisualEffectProperty,
    pub position_z: VisualEffectProperty,
    pub position_offset_x: VisualEffectProperty,
    pub position_offset_y: VisualEffectProperty,
    pub position_offset_z: VisualEffectProperty,
    pub velocity_min: VisualEffectProperty,
    pub velocity_max: VisualEffectProperty,
    pub lifetime_min: VisualEffectProperty,
    pub lifetime_max: VisualEffectProperty,
    pub particle: u8,
}

impl ContinuousEmitter {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let name = node.attribute("name").ok_or_else(|| {
            anyhow::anyhow!("expected <ContinuousEmitter> node to have a 'name' attribute")
        })?;

        let mut position_x: Option<VisualEffectProperty> = None;
        let mut position_y: Option<VisualEffectProperty> = None;
        let mut position_z: Option<VisualEffectProperty> = None;
        let mut position_offset_x: Option<VisualEffectProperty> = None;
        let mut position_offset_y: Option<VisualEffectProperty> = None;
        let mut position_offset_z: Option<VisualEffectProperty> = None;
        let mut velocity_min: Option<VisualEffectProperty> = None;
        let mut velocity_max: Option<VisualEffectProperty> = None;
        let mut lifetime_min: Option<VisualEffectProperty> = None;
        let mut lifetime_max: Option<VisualEffectProperty> = None;
        let mut particle: Option<u8> = None;

        for child in node.children().filter(|n| n.is_element()) {
            // seems like the names are consistently cased but the 'value' attribute is not?

            let child_name = child.tag_name().name();
            match child_name {
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
                "LifeTimeMin" => {
                    lifetime_min = Some(VisualEffectProperty::read(child)?);
                }
                "LifeTimeMax" => {
                    lifetime_max = Some(VisualEffectProperty::read(child)?);
                }
                "Particle" => {
                    let child_value = child
                        .attributes()
                        .find(|attr| attr.name().eq_ignore_ascii_case("value"))
                        .map(|attr| attr.value())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "expected <{child_name}> node to have a 'value' attribute"
                            )
                        })?;
                    particle = Some(child_value.parse()?);
                }
                _ => {} // TODO
            }
        }

        let Some(particle) = particle else {
            anyhow::bail!("expected <ContinuousEmitter> node to have a <Particle> child")
        };

        let position_x = position_x.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_y = position_y.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_z = position_z.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_x = position_offset_x.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_y = position_offset_y.unwrap_or(VisualEffectProperty::Constant(0.0));
        let position_offset_z = position_offset_z.unwrap_or(VisualEffectProperty::Constant(0.0));
        let velocity_min = velocity_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let velocity_max = velocity_max.unwrap_or(VisualEffectProperty::Constant(0.0));
        let lifetime_min = lifetime_min.unwrap_or(VisualEffectProperty::Constant(0.0));
        let lifetime_max = lifetime_max.unwrap_or(VisualEffectProperty::Constant(0.0));

        Ok(ContinuousEmitter {
            name: name.into(),
            position_x,
            position_y,
            position_z,
            position_offset_x,
            position_offset_y,
            position_offset_z,
            velocity_min,
            velocity_max,
            lifetime_min,
            lifetime_max,
            particle,
        })
    }
}
