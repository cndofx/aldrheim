use anyhow::Context;
use roxmltree::Node;

#[derive(Debug)]
pub struct Trigger {
    name: String,
    repeat: TriggerRepeat,
    autorun: bool,
    conditions: Vec<TriggerCondition>,
    actions: Vec<TriggerAction>,
}

impl Trigger {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let name = node
            .attribute("id")
            .ok_or_else(|| anyhow::anyhow!("expected <Trigger> node to have an 'id' attribute"))?
            .to_string();

        let repeat = if let Some(repeat_attr) = node.attribute("repeat") {
            TriggerRepeat::from_str(repeat_attr)?
        } else {
            anyhow::bail!("expected <Trigger> node to have a 'repeat' attribute");
        };

        let autorun = if let Some(autorun_attr) = node.attribute("autorun") {
            if autorun_attr.eq_ignore_ascii_case("true") {
                true
            } else if autorun_attr.eq_ignore_ascii_case("false") {
                false
            } else {
                anyhow::bail!(
                    "expected trigger autorun attribute to be 'true' or 'false', got '{autorun_attr}'"
                );
            }
        } else {
            true // TODO: is this the correct default?
        };

        let mut conditions = Vec::new();
        if let Some(conditions_node) = node
            .children()
            .find(|n| n.tag_name().name().eq_ignore_ascii_case("if"))
        {
            for condition_node in conditions_node.children().filter(|n| n.is_element()) {
                match TriggerCondition::read(condition_node) {
                    Ok(v) => conditions.push(v),
                    Err(e) => log::error!("{e}"),
                }
            }
        }

        let actions = Vec::new();

        Ok(Trigger {
            name,
            repeat,
            autorun,
            conditions,
            actions,
        })
    }
}

#[derive(Debug)]
pub enum TriggerRepeat {
    False,
    True,
    Count(f32),
}

impl TriggerRepeat {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        if s.eq_ignore_ascii_case("true") {
            Ok(TriggerRepeat::True)
        } else if s.eq_ignore_ascii_case("false") {
            Ok(TriggerRepeat::False)
        } else {
            let count = s
                .parse::<f32>()
                .with_context(|| format!("unable to parse trigger repeat value {s}"))?;
            Ok(TriggerRepeat::Count(count))
        }
    }
}

#[derive(Debug)]
pub enum TriggerCondition {
    Present(TriggerConditionPresent),
}

impl TriggerCondition {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let name = node.tag_name().name().to_ascii_lowercase();

        match name.as_str() {
            "present" => Ok(TriggerCondition::Present(TriggerConditionPresent::read(
                node,
            )?)),
            _ => anyhow::bail!("unknown trigger condition '{name}'"),
        }
    }
}

#[derive(Debug)]
pub struct TriggerConditionPresent {
    kind: String,
    area: String,
    compare_method: CompareMethod,
    number: i32,
}

impl TriggerConditionPresent {
    pub fn read(node: Node) -> anyhow::Result<Self> {
        let Some(kind) = node.attribute("type") else {
            anyhow::bail!("expected <Present> node to have a 'type' attribute");
        };

        let Some(area) = node.attribute("area") else {
            anyhow::bail!("expected <Present> node to have an 'area' attribute");
        };

        let compare_method = if let Some(method_attr) = node.attribute("compareMethod") {
            if method_attr.eq_ignore_ascii_case("less") {
                CompareMethod::Less
            } else if method_attr.eq_ignore_ascii_case("equal") {
                CompareMethod::Equal
            } else if method_attr.eq_ignore_ascii_case("greater") {
                CompareMethod::Greater
            } else {
                anyhow::bail!(
                    "expected <Present> node 'compareMethod' attribute value to be 'less', 'equal', or 'greater', got '{method_attr}'"
                );
            }
        } else {
            anyhow::bail!("expected <Present> node to have a 'compareMethod' attribute");
        };

        let number = if let Some(number_attr) = node.attribute("nr") {
            number_attr.parse::<i32>().with_context(|| {
                format!("unable to parse <Present> node 'nr' attribute value {number_attr}")
            })?
        } else {
            anyhow::bail!("expected <Present> node to have a 'nr' attribute");
        };

        Ok(TriggerConditionPresent {
            kind: kind.into(),
            area: area.into(),
            compare_method,
            number,
        })
    }
}

#[derive(Debug)]
pub enum TriggerAction {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMethod {
    Less,
    Equal,
    Greater,
}
