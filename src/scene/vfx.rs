use anyhow::Context;
use roxmltree::{Document, Node};

use crate::scene::vfx::continuous_emitter::ContinuousEmitter;

pub mod continuous_emitter;

#[derive(Debug)]
pub struct VisualEffect {
    pub kind: VisualEffectKind,
    pub emitters: Vec<ParticleEmitter>,
}

impl VisualEffect {
    pub fn read_xml(xml: &str) -> anyhow::Result<Self> {
        VisualEffect::read_xml_inner(xml, true)
    }

    fn read_xml_inner(xml: &str, allow_retry: bool) -> anyhow::Result<Self> {
        let doc = match Document::parse(xml) {
            Ok(v) => v,
            Err(roxmltree::Error::MalformedEntityReference(_)) if allow_retry => {
                log::warn!("found malformed entity reference in xml, stripping and trying again");
                let stripped = strip_malformed_reference(xml);
                return VisualEffect::read_xml_inner(&stripped, false);
            }
            Err(e) => Err(e)?,
        };

        let root = doc.root_element();
        if root.tag_name().name() != "Effect" {
            anyhow::bail!("expected root element to be an <Effect> node");
        }

        let kind = if let Some(kind_attr) = root.attribute("type") {
            match kind_attr {
                "Single" => VisualEffectKind::Single,
                "Looping" => VisualEffectKind::Looping,
                "Infinite" => VisualEffectKind::Infinite,
                _ => {
                    anyhow::bail!("unsupported <Effect> node 'type' attribute value '{kind_attr}'");
                }
            }
        } else {
            anyhow::bail!("expected <Effect> node to have a 'type' attribute");
        };

        let mut emitters: Vec<ParticleEmitter> = Vec::new();

        for child in root.children().filter(|n| n.is_element()) {
            let child_name = child.tag_name().name();

            match child_name {
                "ContinuousEmitter" => {
                    let emitter = ContinuousEmitter::read(child)?;
                    emitters.push(ParticleEmitter::Continuous(emitter));
                }
                _ => {
                    log::error!("unsupported <Effect> child node <{child_name}>");
                }
            }
        }

        Ok(VisualEffect { kind, emitters })
    }
}

#[derive(Debug)]
pub enum VisualEffectKind {
    Single,
    Looping,
    Infinite,
}

#[derive(Debug)]
pub struct VisualEffectPropertyKeyframe {
    pub time: i32, // looks like an integer? probably a keyframe index instead of a "seconds" or similar value?
    pub value: f32,
}

impl VisualEffectPropertyKeyframe {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let time = if let Some(time_attr) = node.attribute("time") {
            time_attr.parse::<i32>().with_context(|| {
                format!("unable to parse vfx property keyframe time from '{time_attr}'")
            })?
        } else {
            anyhow::bail!("expected <Key> node to have a 'time' attribute");
        };

        let value = if let Some(value_attr) = node.attribute("value") {
            value_attr.parse::<f32>().with_context(|| {
                format!("unable to parse vfx property keyframe value from '{value_attr}'")
            })?
        } else {
            anyhow::bail!("expected <Key> node to have a 'value' attribute");
        };

        Ok(VisualEffectPropertyKeyframe { time, value })
    }
}

#[derive(Debug)]
pub enum VisualEffectProperty {
    Constant(f32),
    Animated(Vec<VisualEffectPropertyKeyframe>),
}

impl VisualEffectProperty {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let name = node.tag_name().name();
        let value = node
            .attributes()
            .find(|attr| attr.name().eq_ignore_ascii_case("value"))
            .map(|attr| attr.value());

        if let Some(value) = value {
            return Ok(VisualEffectProperty::Constant(value.parse()?));
        }

        let mut keyframes = Vec::new();

        for child in node.children().filter(|n| n.is_element()) {
            let child_name = child.tag_name().name();
            if child_name != "Key" {
                continue;
            }

            let keyframe = VisualEffectPropertyKeyframe::read(child)?;
            keyframes.push(keyframe);
        }

        if keyframes.is_empty() {
            anyhow::bail!(
                "expected <{name}> node to have <Key> children because it does not have a 'value' attribute"
            );
        }

        Ok(VisualEffectProperty::Animated(keyframes))
    }
}

#[derive(Debug)]
pub enum ParticleEmitter {
    Continuous(ContinuousEmitter),
}

// technically stripping all references but close enough unless it causes problems later
fn strip_malformed_reference(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());

    let mut chars = xml.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() || next == ';' {
                    chars.next();
                    break;
                }
                chars.next();
            }
        } else {
            out.push(c);
        }
    }

    out
}
