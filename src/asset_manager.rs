use std::{
    collections::HashMap,
    io::BufReader,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Context;
use glam::{Mat4, Quat, Vec3};

use crate::{
    asset_manager::vfx::VisualEffectAsset,
    renderer::{Renderer, pipelines::render_deferred_effect::RenderDeferredEffectUniform},
    scene::{self, SceneNode, SceneNodeKind, vfx::VisualEffectNode},
    xnb::{BiTreeNode, Xnb, XnbContent, asset::XnbAsset},
};

pub mod vfx;

pub struct AssetManager {
    magicka_path: PathBuf,

    // using `Rc` instead of `Weak` so that resources arent immediately dropped
    // when no longer used. if all the "goblin" enemies died, the goblin mesh
    // would disappear, even though the game is likely to need the goblin mesh
    // again. i'm thinking all meshes should be loaded during a loading screen,
    // and all unneeded meshes are dropped during that same loading screen
    textures_2d: HashMap<PathBuf, Rc<Texture2DAsset>>,
    models: HashMap<PathBuf, Rc<ModelAsset>>,

    // visual effects are keyed by filename strings instead of full paths
    // because they are referenced by filename (unique, without extension)
    // in other data such as levels, rather than by path like other assets
    //
    // they are also preloaded up front as they can located in arbitrary subdirectories,
    // so locating the file would require a recursive search of the entire Content/Effect directory
    visual_effects: HashMap<String, Rc<VisualEffectAsset>>,
}

impl AssetManager {
    pub fn new(magicka_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let magicka_path = magicka_path.into();
        let visual_effects = preload_visual_effects(&magicka_path)?;

        Ok(AssetManager {
            magicka_path,
            visual_effects,
            textures_2d: HashMap::new(),
            models: HashMap::new(),
        })
    }

    pub fn read_to_string(&self, path: &Path, base: Option<&Path>) -> anyhow::Result<String> {
        let path = self.resolve_path(path, base, None)?;
        let string = std::fs::read_to_string(&path)?;
        log::debug!("loaded string data from file {}", path.display());
        Ok(string)
    }

    pub fn load_texture_2d(
        &mut self,
        path: &Path,
        base: Option<&Path>,
        renderer: &Renderer,
    ) -> anyhow::Result<Rc<Texture2DAsset>> {
        let path = self.resolve_path(path, base, Some("xnb"))?;
        if let Some(texture) = self.textures_2d.get(&path) {
            return Ok(texture.clone());
        }

        let content = self.load_xnb_content(&path)?;
        let XnbAsset::Texture2D(texture) = &content.primary_asset else {
            anyhow::bail!("expected Texture2D at path {}", path.display());
        };

        let texture = renderer.load_texture_2d(texture)?;
        let texture = Rc::new(texture);

        log::debug!("loaded Texture2D from file {}", path.display());

        self.textures_2d.insert(path, texture.clone());

        Ok(texture)
    }

    pub fn load_model(
        &mut self,
        path: &Path,
        base: Option<&Path>,
        renderer: &Renderer,
    ) -> anyhow::Result<Rc<ModelAsset>> {
        let path = self.resolve_path(path, base, Some("xnb"))?;
        if let Some(model) = self.models.get(&path) {
            return Ok(model.clone());
        }

        let model_content = self.load_xnb_content(&path)?;
        let XnbAsset::Model(model) = &model_content.primary_asset else {
            anyhow::bail!("expected Model at path {}", path.display());
        };
        let XnbAsset::RenderDeferredEffect(effect) = &model_content.shared_assets[0] else {
            anyhow::bail!(
                "expected RenderDeferredEffect at shared assets 0 at path {}",
                path.display()
            );
        };

        let texture = self.load_texture_2d(
            &fix_xnb_path(&effect.material_0.diffuse_texture),
            Some(&path),
            renderer,
        )?;

        let model = renderer.load_model(model, texture)?;
        let model = Rc::new(model);

        log::debug!("loaded Model from file {}", path.display());

        self.models.insert(path, model.clone());

        Ok(model)
    }

    pub fn load_level_model(
        &mut self,
        path: &Path,
        base: Option<&Path>,
        renderer: &Renderer,
    ) -> anyhow::Result<SceneNode> {
        let path = self.resolve_path(path, base, Some("xnb"))?;

        let model_content = self.load_xnb_content(&path)?;
        let XnbAsset::LevelModel(level_model) = &model_content.primary_asset else {
            anyhow::bail!("expected LevelModel at path {}", path.display());
        };

        let mut scene_node = SceneNode {
            name: "Level Model".into(),
            visible: true,
            transform: Mat4::IDENTITY,
            children: Vec::new(),
            kind: SceneNodeKind::Empty,
        };

        for tree in &level_model.model.trees {
            debug_assert_eq!(tree.vertex_stride as usize, tree.vertex_decl.stride());

            let XnbAsset::RenderDeferredEffect(effect) = &tree.effect else {
                if let XnbAsset::AdditiveEffect(_) = &tree.effect {
                    log::warn!("skipping unimplemented BiTree with AdditiveEffect");
                    continue;
                }

                anyhow::bail!(
                    "expected RenderDeferredEffect inside LevelModel BiTree, got {}",
                    tree.effect.as_ref()
                );
            };
            // dbg!(&tree.vertex_decl, effect);
            // println!("\n\n\n");

            let diffuse_texture_0 = if effect.material_0.diffuse_texture.len() > 0 {
                Some(self.load_texture_2d(
                    &fix_xnb_path(&effect.material_0.diffuse_texture),
                    Some(&path),
                    renderer,
                )?)
            } else {
                None
            };

            let diffuse_texture_1 = if let Some(material_1) = &effect.material_1 {
                if material_1.diffuse_texture.len() > 0 {
                    Some(self.load_texture_2d(
                        &fix_xnb_path(&material_1.diffuse_texture),
                        Some(&path),
                        renderer,
                    )?)
                } else {
                    None
                }
            } else {
                None
            };

            let effect_uniform = RenderDeferredEffectUniform::new(&effect, &tree.vertex_decl)?;

            let asset =
                renderer.load_bitree(tree, diffuse_texture_0, diffuse_texture_1, effect_uniform)?;
            load_level_model_bitree_node_recursive(&mut scene_node, &tree.node, Rc::new(asset))?;
        }

        for effect_storage in &level_model.effect_storages {
            let effect = self.load_visual_effect(&effect_storage.effect)?;

            let effect_node = SceneNode {
                name: effect_storage.name.clone(),
                visible: true,
                transform: Mat4::from_rotation_translation(
                    Quat::look_to_rh(effect_storage.forward, Vec3::Y),
                    effect_storage.position,
                ),
                children: Vec::new(),
                kind: SceneNodeKind::VisualEffect(VisualEffectNode::new(effect)),
            };

            scene_node.children.push(effect_node);
        }

        log::debug!("loaded LevelModel from file {}", path.display());

        Ok(scene_node)
    }

    pub fn load_visual_effect(&self, name: &str) -> anyhow::Result<Rc<VisualEffectAsset>> {
        if let Some(effect) = self.visual_effects.get(name) {
            return Ok(effect.clone());
        } else {
            anyhow::bail!("visual effect '{name}' not found");
        }
    }

    fn load_xnb_content(&self, path: &Path) -> anyhow::Result<XnbContent> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("failed to open file {}", path.display()))?;
        let mut reader = BufReader::new(file);
        let xnb = Xnb::read(&mut reader)?;
        let content = xnb
            .parse_content()
            .with_context(|| format!("failed to parse content from file {}", path.display()))?;
        Ok(content)
    }

    /// - `path` is a file path relative the magicka installation root.
    ///    the casing needn't match the filesystem, and an `xnb` extension will be added if not present.
    /// - `base` is the directory `path` is relative to. this path must exist on case sensitive filesystems.
    ///   - if `base` is `None`, the root Magicka installation directory is assumed.
    ///   - if `base` is a relative path, it is appended to the root Magicka installation directory.
    ///   - if `base` is a file path, the parent directory will be used.
    fn resolve_path(
        &self,
        path: &Path,
        base: Option<&Path>,
        ensure_extension: Option<&str>,
    ) -> anyhow::Result<PathBuf> {
        // default to magicka install dir
        let mut base = base
            .map(|b| b.to_owned())
            .unwrap_or(self.magicka_path.clone());

        // make base path absolute
        if !base.has_root() {
            base = self.magicka_path.join(base);
        }

        // make base path a directory
        if !base.is_dir() {
            base.pop();
        }

        // ensure path has an extension (relative paths stored inside XNBs dont have .xnb extensions)
        let path = if let Some(extension) = ensure_extension
            && path.extension().is_none()
        {
            path.with_extension(extension)
        } else {
            path.to_owned()
        };

        // short circuit if the casing is already correct
        let full_path = base.join(&path);
        if full_path.exists() {
            // canonicalize might be unnecessary but we're hashing paths
            return Ok(full_path.canonicalize()?);
        }

        // recursively match each component of the relative path case-insensitively
        let mut current_path = base;
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    current_path.pop();
                }
                std::path::Component::Normal(insensitive_component) => {
                    let lower_component = insensitive_component.to_ascii_lowercase();

                    let mut found: Option<std::ffi::OsString> = None;
                    for entry in std::fs::read_dir(&current_path)? {
                        let entry_name = entry?.file_name();
                        let lower_entry_name = entry_name.to_ascii_lowercase();

                        if lower_entry_name == lower_component {
                            found = Some(entry_name);
                            break;
                        }
                    }

                    if let Some(found) = found {
                        current_path.push(found);
                    } else {
                        current_path.push(insensitive_component);
                        anyhow::bail!("unable to find path {}", current_path.display());
                    }
                }
                _ => {}
            }
        }

        // canonicalize might be unnecessary but we're hashing paths
        Ok(current_path.canonicalize()?)
    }
}

pub struct Texture2DAsset {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub struct ModelAsset {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_buffer_bind_group: wgpu::BindGroup,
    pub vertex_layout_uniform_buffer: wgpu::Buffer,
    pub vertex_layout_uniform_bind_group: wgpu::BindGroup,
    pub index_buffer: wgpu::Buffer,
    pub index_format: wgpu::IndexFormat,
    pub index_count: u32,
    pub start_index: u32,
    pub base_vertex: u32,
    pub texture: Rc<Texture2DAsset>,
}

pub struct BiTreeAsset {
    pub visible: bool,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_buffer_bind_group: wgpu::BindGroup,
    pub vertex_layout_uniform_buffer: wgpu::Buffer,
    pub vertex_layout_uniform_bind_group: wgpu::BindGroup,
    pub index_buffer: wgpu::Buffer,
    pub index_format: wgpu::IndexFormat,
    pub texture_bind_group: wgpu::BindGroup,
    pub diffuse_texture_0: Option<Rc<Texture2DAsset>>,
    pub diffuse_texture_1: Option<Rc<Texture2DAsset>>,
}

fn load_level_model_bitree_node_recursive(
    parent: &mut SceneNode,
    bitree_node: &BiTreeNode,
    bitree_asset: Rc<BiTreeAsset>,
) -> anyhow::Result<()> {
    let mut node = SceneNode {
        name: "BiTree Node".into(),
        visible: bitree_asset.visible,
        transform: Mat4::IDENTITY,
        children: Vec::new(),
        kind: SceneNodeKind::BiTree(scene::BiTreeNode {
            tree: bitree_asset.clone(),
            start_index: bitree_node.start_index as u32,
            index_count: bitree_node.primitive_count as u32 * 3,
            bounding_box: bitree_node.bounding_box.clone(),
        }),
    };

    for child in bitree_node.iter_children() {
        load_level_model_bitree_node_recursive(&mut node, child, bitree_asset.clone())?;
    }

    parent.children.push(node);

    Ok(())
}

fn fix_xnb_path(path: &str) -> PathBuf {
    let path = path.replace('\\', "/");
    PathBuf::from(path)
}

fn preload_visual_effects(base: &Path) -> anyhow::Result<HashMap<String, Rc<VisualEffectAsset>>> {
    let path = base.join("Content/Effects");
    let mut map = HashMap::new();

    preload_visual_effects_inner(&path, &mut map)?;

    Ok(map)
}

fn preload_visual_effects_inner(
    path: &Path,
    map: &mut HashMap<String, Rc<VisualEffectAsset>>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(&path)? {
        // cursed closure to allow catching all errors at once
        // if one file failes to load, it will be logged and traversal will continue
        if let Err(e) = (|| -> anyhow::Result<()> {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();

            if metadata.is_file() {
                let xml_string = std::fs::read_to_string(&path)?;
                let effect = VisualEffectAsset::read_xml(&xml_string).with_context(|| {
                    format!("failed to read visual effect at path {}", path.display())
                })?;
                let name = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_ascii_lowercase();
                map.insert(name, Rc::new(effect));
            } else if metadata.is_dir() {
                preload_visual_effects_inner(&path, map)?;
            } else {
                unreachable!("vfx entry is not a file or a directory");
            }

            Ok(())
        })() {
            log::error!("{e}");
        }
    }

    Ok(())
}
