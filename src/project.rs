use super::commands::AppParams;
use super::manifest::{Manifest, Project};
use std::path::{Path, PathBuf};

/// Read the manifest file, do some basic checks and call a function with the right parameters
pub(crate) fn on_project<F, R>(
    app_params: &AppParams,
    /*
    manifest_file: &str,
    out_dir: &str,
    project_name: Option<&str>,
    */
    f: F,
) -> anyhow::Result<R>
where
    F: FnOnce(&Path, &Manifest, &Project) -> anyhow::Result<R>,
{
    let (manifest, project) = app_params.get_project();
    let out_dir = app_params.get_destpath();

    f(&out_dir, &manifest, &project)
}

/// Read the manifest file but also iterate over each repositories
/// composing this project
pub(crate) fn on_project_repos<F, R>(app_params: &AppParams, f: F) -> anyhow::Result<Vec<R>>
where
    F: Fn(&Project, PathBuf, &str) -> anyhow::Result<R>,
{
    let (_manifest, project) = app_params.get_project();
    let out_dir = app_params.get_destpath();

    let mut returns = Vec::new();
    for repo in project.repos.iter() {
        let p = Path::new(repo);
        let dest_repo = match p.file_name() {
            None => {
                println!("ignoring {}", repo);
                continue;
            }
            Some(s) => {
                let mut x = out_dir.to_path_buf();
                x.push(s);
                x
            }
        };

        if !dest_repo.exists() {
            panic!(
                "run manifest-sync first, directory {:?} missing",
                &dest_repo
            )
        }

        let name = dest_repo
            .file_name()
            .expect("file name")
            .to_str()
            .expect("git with valid UTF8")
            .to_string();
        let r = f(&project, dest_repo, &name)?;
        returns.push(r)
    }
    Ok(returns)
}
