use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::xnb::Xnb;

pub struct AssetManager {
    magicka_path: PathBuf,
}

impl AssetManager {
    pub fn new(magicka_path: impl Into<PathBuf>) -> Self {
        AssetManager {
            magicka_path: magicka_path.into(),
        }
    }

    /// load an xnb from a path relative to the magicka install directory
    pub fn load_xnb(&self, path: impl AsRef<Path>) -> anyhow::Result<Xnb> {
        let xnb = self.load_xnb_relative(&self.magicka_path, path)?;
        Ok(xnb)
    }

    /// load an xnb from a path relative to the given base path
    pub fn load_xnb_relative(
        &self,
        base: impl AsRef<Path>,
        relative: impl AsRef<Path>,
    ) -> anyhow::Result<Xnb> {
        let path = self.resolve_xnb_relative_path(base.as_ref(), relative.as_ref())?;
        let file = std::fs::File::open(&path)?;
        let mut reader = BufReader::new(file);
        let xnb = Xnb::read(&mut reader)?;
        Ok(xnb)
    }

    /// because the relative paths in xnb files rely on windows fs case insensitivity
    /// and this does not work on case sensitive filesystems
    ///
    /// - base path must exist on case sensitive filesystems
    /// - relative path is not required to have a .xnb extension
    pub fn resolve_xnb_relative_path(
        &self,
        base: &Path,
        relative: &Path,
    ) -> anyhow::Result<PathBuf> {
        let joined = base.join(&relative);
        if joined.exists() {
            return Ok(joined);
        }

        let relative = if relative.extension().is_none() {
            relative.with_extension("xnb")
        } else {
            relative.to_owned()
        };

        let mut current_path = if base.has_root() {
            base.to_owned()
        } else {
            self.magicka_path.join(base)
        };

        if current_path.is_file() {
            current_path.pop();
        }

        for component in relative.components() {
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

        Ok(current_path)
    }
}
