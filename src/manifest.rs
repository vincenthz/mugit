use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use thiserror::*;
use toml;

#[derive(Debug)]
pub struct Manifests {
    manifests: HashMap<String, Manifest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    projects: HashMap<String, Project>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Project {
    pub prefix: String,
    pub repos: Vec<String>,
    #[serde(rename = "remote-name")]
    pub remote_name: String,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("deserialization error {0}")]
    DeserializationError(toml::de::Error),
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("no project is defined in this manifest")]
    NoDefined,
    #[error("cannot merge project due to prefix difference between {0} and {1}")]
    CannotMergePrefix(String, String),
    #[error("cannot merge project due to remote name difference between {0} and {1}")]
    CannotMergeRemoteName(String, String),
    #[error("project {0} not found in this manifest")]
    CannotFind(String),
}

impl Manifests {
    pub fn get(&self, name: &str) -> Option<&Manifest> {
        self.manifests.get(name)
    }
}

impl Manifest {
    pub fn from_file<P: AsRef<Path>>(file: P) -> io::Result<Result<Manifest, ManifestError>> {
        let file = file.as_ref();
        let content = std::fs::read_to_string(file)?;
        Ok(toml::from_str(&content).map_err(ManifestError::DeserializationError))
    }

    pub fn merge_project(&self) -> Result<Project, ProjectError> {
        let mut projects_iter = self.projects.iter();
        let (iname, mut overall_proj) = match projects_iter.next() {
            None => Err(ProjectError::NoDefined),
            Some((init_name, project)) => Ok((init_name, project.clone())),
        }?;

        for (proj_name, proj) in projects_iter {
            if proj.prefix != overall_proj.prefix {
                return Err(ProjectError::CannotMergePrefix(
                    iname.clone(),
                    proj_name.clone(),
                ));
            }
            if proj.remote_name != overall_proj.remote_name {
                return Err(ProjectError::CannotMergeRemoteName(
                    iname.clone(),
                    proj_name.clone(),
                ));
            }

            overall_proj.repos.extend_from_slice(&proj.repos)
        }
        Ok(overall_proj)
    }

    pub fn get_project<'a>(
        &'a self,
        project_name: Option<&str>,
    ) -> Result<std::borrow::Cow<'a, Project>, ProjectError> {
        match project_name {
            None => self.merge_project().map(std::borrow::Cow::Owned),
            Some(project_name) => {
                let project = self
                    .projects
                    .get(project_name)
                    .ok_or(ProjectError::CannotFind(project_name.to_string()))?;
                Ok(std::borrow::Cow::Borrowed(project))
            }
        }
    }
}

pub(crate) fn read_manifests<P: AsRef<Path>>(p: P) -> Result<Manifests, std::io::Error> {
    let mut known_files = HashMap::new();
    for entry in std::fs::read_dir(p)? {
        let entry = entry?;
        let path = entry.path();
        match path.file_name() {
            None => continue,
            Some(os_str) => match os_str.to_str() {
                None => continue,
                Some(s) => {
                    if let Some(name) = s.strip_suffix(".toml") {
                        match Manifest::from_file(&path)? {
                            Ok(m) => {
                                let _: Option<_> = known_files.insert(name.to_string(), m);
                                ()
                            }
                            Err(_) => continue,
                        }
                    } else {
                        continue;
                    }
                }
            },
        }
    }
    Ok(Manifests {
        manifests: known_files,
    })
}
