use std::path::Path;

use roxmltree::Document;

use crate::{
    asset_manager::AssetManager,
    scene::{Scene, Skymap},
    xnb::asset::color::Color,
};

impl Scene {
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
        let mut skymap_path: Option<&str> = None;
        let mut skymap_color: Option<Color> = None;
        let mut indoors: Option<bool> = None;

        for child in root.children().filter(|child| child.is_element()) {
            let child_name = child.tag_name().name().to_lowercase();

            match child_name.as_str() {
                "model" => {
                    let text = child
                        .text()
                        .ok_or_else(|| anyhow::anyhow!("expected <Model> node to contain text"))?;
                    model_path = Some(text);
                }
                "skymap" => {
                    let text = child
                        .text()
                        .ok_or_else(|| anyhow::anyhow!("expected <Model> node to contain text"))?;

                    let color_str = child.attribute("color").ok_or_else(|| {
                        anyhow::anyhow!("expected <SkyMap> node to have a 'color' attribute")
                    })?;
                    let color = color_str
                        .split(',')
                        .map(|v| v.parse::<f32>())
                        .collect::<Result<Vec<f32>, _>>()?;
                    if color.len() != 3 {
                        anyhow::bail!(
                            "expected <SkyMap> node 'color' attribute to have 3 comma separated values"
                        );
                    }
                    let color = Color {
                        r: color[0],
                        g: color[1],
                        b: color[2],
                    };

                    skymap_path = Some(text);
                    skymap_color = Some(color);
                }
                "indoor" => {
                    let text = child
                        .text()
                        .ok_or_else(|| anyhow::anyhow!("expected <Model> node to contain text"))?;
                    if text.eq_ignore_ascii_case("true") {
                        indoors = Some(true);
                    } else if text.eq_ignore_ascii_case("false") {
                        indoors = Some(false);
                    } else {
                        anyhow::bail!(
                            "expected <Indoor> node to have the text 'true' or 'false', got '{text}'"
                        );
                    }
                }
                _ => {}
            }
        }

        let skymap_texture = skymap_path
            .map(|p| asset_manager.load_texture(Path::new(p), Some(xml_path)))
            .transpose()?;

        let Some(model_path) = model_path else {
            anyhow::bail!("xml does not have a <Model> node");
        };

        let model_node = asset_manager.load_level_model(Path::new(model_path), Some(xml_path))?;

        let mut scene = Scene::new();

        scene.root_node.children.push(model_node);
        scene.skymap = skymap_texture.map(|texture| Skymap {
            texture,
            color: skymap_color.unwrap_or(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            }),
        });
        scene.indoors = indoors.unwrap_or(false);

        Ok(scene)
    }
}
