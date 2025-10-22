use std::path::Path;

use roxmltree::Document;

use crate::{asset_manager::AssetManager, scene::Scene};

impl Scene {
    // TODO: passing renderer here feels bad
    pub fn load_level(
        xml_path: &Path,
        base_path: Option<&Path>,
        asset_manager: &mut AssetManager,
    ) -> anyhow::Result<Self> {
        let xml = asset_manager.read_to_string(xml_path, base_path)?;
        let doc = Document::parse(&xml)?;

        let root = doc.root_element();
        if root.tag_name().name() != "Scene" {
            anyhow::bail!("expected root element to be an <Scene> node");
        }

        let mut model_path: Option<&str> = None;

        for child in root.children().filter(|child| child.is_element()) {
            let child_name = child.tag_name().name();

            if child_name.eq_ignore_ascii_case("Model") {
                if model_path.is_none() {
                    model_path =
                        Some(child.text().ok_or_else(|| {
                            anyhow::anyhow!("expected <Model> node to contain text")
                        })?);
                } else {
                    log::warn!("duplicate <Model> node in scene xml");
                }

                continue;
            }
        }

        let Some(model_path) = model_path else {
            anyhow::bail!("xml does not have a <Model> node");
        };

        let model_node = asset_manager.load_level_model(Path::new(model_path), Some(xml_path))?;

        let mut scene = Scene::new();

        scene.root_node.children.push(model_node);

        Ok(scene)
    }
}
