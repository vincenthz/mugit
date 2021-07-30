use git2::Repository;
use semver::Version;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::githelp;
use super::manifest::{Manifest, Manifests, Project};
use super::project::{on_project, on_project_repos};
use super::util;
use super::ver::*;

#[derive(Clone, Debug)]
pub struct AppParams {
    pub git_exec: bool,
    pub sys_manifests: Arc<Option<Manifests>>,
    pub manifest_selector: Option<String>,
    pub manifest_file: Option<PathBuf>,
    pub manifest_dest: Option<PathBuf>,
}

pub enum Selector {
    One(String),
    Two(String, String),
}

impl AppParams {
    fn selector(&self) -> Option<Selector> {
        match &self.manifest_selector {
            None => None,
            Some(sel) => {
                let x = sel.split("::").collect::<Vec<_>>();
                if x.len() == 1 {
                    Some(Selector::One(x[0].to_string()))
                } else if x.len() == 2 {
                    Some(Selector::Two(x[0].to_string(), x[1].to_string()))
                } else {
                    None
                }
            }
        }
    }

    pub fn get_manifest(&self) -> Manifest {
        match &self.manifest_file {
            None => match self.selector() {
                None => {
                    panic!("cannot have no selector")
                }
                Some(Selector::One(selector)) => match self.sys_manifests.as_ref() {
                    None => panic!("no loaded manifests"),
                    Some(manifests) => manifests.get(&selector).expect("existing manifest").clone(),
                },
                Some(Selector::Two(selector, _)) => match self.sys_manifests.as_ref() {
                    None => panic!("no loaded manifests"),
                    Some(manifests) => manifests.get(&selector).expect("existing manifest").clone(),
                },
            },
            Some(manifest_file) => {
                let manifest = Manifest::from_file(manifest_file)
                    .unwrap()
                    .expect("valid manifest file");
                manifest
            }
        }
    }

    pub fn get_destpath(&self) -> PathBuf {
        match &self.manifest_dest {
            None => {
                let current_dir = std::env::current_dir().unwrap();
                current_dir
            }
            Some(out_dir) => {
                if !out_dir.exists() {
                    panic!("out directory doesn't exist {:?}", out_dir)
                }
                if !out_dir.is_dir() {
                    panic!("out directory {:?} is not a directory", out_dir)
                }
                out_dir.clone()
            }
        }
    }

    pub fn get_project(&self) -> (Manifest, Project) {
        let manifest = self.get_manifest();
        let project = match self.selector().unwrap() {
            Selector::Two(_, x) => manifest.get_project(Some(&x)).unwrap().into_owned(),
            Selector::One(_) => manifest.get_project(None).unwrap().into_owned(),
        };
        (manifest, project)
    }

    pub fn get_destpath_create(&self) -> PathBuf {
        match &self.manifest_dest {
            None => {
                let current_dir = std::env::current_dir().unwrap();
                current_dir
            }
            Some(dest) => {
                if dest.exists() {
                    if !dest.is_dir() {
                        panic!("{:?} is not a directory", dest.as_os_str())
                    }
                    dest.clone()
                } else {
                    std::fs::create_dir(dest).expect("create directory");
                    dest.clone()
                }
            }
        }
    }
}

fn repo_report_error(name: &str, s: &str) {
    println!("{:40} : {}", name, ansi_term::Color::Purple.paint(s))
}

fn filter_matches<'a, I>(specified: Spec, it: I) -> impl Iterator<Item = &'a Version>
where
    I: Iterator<Item = &'a Version>,
{
    it.filter(move |v| specified.fullfill(v.major, v.minor, v.patch))
}

pub fn git_clone(app_params: &AppParams, url: &str, dest_repo: &Path) -> Repository {
    if app_params.git_exec {
        let _out = Command::new("git")
            .arg("clone")
            .arg(url)
            .arg(dest_repo.to_str().expect("dest repo is utf8"))
            .output()
            .expect("git failed to start");
        let repo = Repository::open(&dest_repo).expect("git repository");
        repo
    } else {
        let callbacks = githelp::remote_callbacks();
        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fo);

        builder.clone(url, dest_repo).expect("cloning error")
    }
}

fn git_fetch_all(app_params: &AppParams, repo: &Repository) {
    let source = "origin";
    if app_params.git_exec {
        let workdir = repo.workdir().expect("workdir exists");
        let path = repo.path();
        let _out = Command::new("git")
            .arg(&format!("--git-dir={}", path.to_str().unwrap()))
            .arg(&format!("--work-tree={}", workdir.to_str().unwrap()))
            .arg("fetch")
            .arg(source)
            .output()
            .expect("git failed to start");
    } else {
        let callbacks2 = githelp::remote_callbacks();

        let mut mfo = git2::FetchOptions::new();
        mfo.remote_callbacks(callbacks2);

        let mfo = Some(&mut mfo);
        let refspecs: &[&str] = &[];
        repo.find_remote(source)
            .expect("cannot find origin")
            .fetch(refspecs, mfo, None)
            .expect("fetch error")
    }
}

pub enum PushSpecifier<'a> {
    Tag(&'a str),
    Branch(&'a str),
}

fn git_push_to<'a>(
    app_params: &AppParams,
    project: &Project,
    repo: &Repository,
    spec: PushSpecifier<'a>,
) {
    if app_params.git_exec {
        let workdir = repo.workdir().expect("workdir exists");
        let path = repo.path();
        let spec_str = match spec {
            PushSpecifier::Tag(s) => s,
            PushSpecifier::Branch(s) => s,
        };

        let _out = Command::new("git")
            .arg(&format!("--git-dir={}", path.to_str().unwrap()))
            .arg(&format!("--work-tree={}", workdir.to_str().unwrap()))
            .arg("push")
            .arg(&project.remote_name)
            .arg(spec_str)
            .output()
            .expect("git failed to start");
    } else {
        let mut remote = repo
            .find_remote(&project.remote_name)
            .expect("remote exists");
        let callbacks = githelp::remote_callbacks();

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let ref_to_push = match spec {
            PushSpecifier::Tag(tag) => format!("refs/tags/{}", tag),
            PushSpecifier::Branch(branch) => format!("refs/heads/{}", branch),
        };

        remote
            .push(&[ref_to_push], Some(&mut push_options))
            .expect("push error");
    }
}

pub fn version_find(repo_path: &str, spec: Option<&str>) {
    let repo = Repository::open(repo_path).expect("git repository");
    let tags = repo.tag_names(None).expect("tags working");

    let all_tags = tags
        .iter()
        .filter_map(|f| f.map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let all_versions = all_tags
        .iter()
        .filter_map(|t| Version::parse(t).ok())
        .collect::<BTreeSet<Version>>();

    // println!("===");
    match spec {
        None => {
            for v in all_versions.iter() {
                println!("{}", v)
            }
        }
        Some(s) => {
            let specified = Spec::from_str(s).expect("spec");
            for v in filter_matches(specified, all_versions.iter()) {
                println!("{}", v)
            }
        }
    }
}

pub fn has_remote_branch(repo_path: &str, remote_name: &str, branch: &str) {
    let repo = Repository::open(repo_path).expect("git repository");
    let has_branch = githelp::has_remote_branch(&repo, remote_name, branch);
    if has_branch {
        println!(
            "{:40} : {} branch {}   ✅",
            repo_path,
            branch,
            ansi_term::Color::Green.paint("found")
        )
    } else {
        println!(
            "{:40} : {} branch {} ❌",
            repo_path,
            branch,
            ansi_term::Color::Red.paint("missing")
        )
    }
}

pub fn manifest_debug(manifest_file: &str) {
    let manifest = Manifest::from_file(manifest_file)
        .unwrap()
        .expect("valid manifest file");
    println!("{:?}", manifest)
}

pub fn manifest_has_branch(app_params: &AppParams, branch: &str) -> anyhow::Result<()> {
    on_project(app_params, |out_dir, _, project| {
        for repo in project.repos.iter() {
            let _p = Path::new(repo);
            let dest_repo = match _p.file_name() {
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
            let repo = Repository::open(&dest_repo).expect("git repository");
            let has_branch = githelp::has_remote_branch(&repo, &project.remote_name, branch);

            let name = dest_repo
                .file_name()
                .expect("file name")
                .to_str()
                .expect("git with valid UTF8");
            if has_branch {
                println!(
                    "{:40} : {} branch {}   ✅",
                    name,
                    branch,
                    ansi_term::Color::Green.paint("found")
                )
            } else {
                println!(
                    "{:40} : {} branch {} ❌",
                    name,
                    branch,
                    ansi_term::Color::Red.paint("missing")
                )
            }
        }
        Ok(())
    })
}

pub fn manifest_has_tag(app_params: &AppParams, tag: &str) -> anyhow::Result<()> {
    on_project_repos(app_params, |_, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");
        let has_tag = githelp::has_remote_tag(&repo, tag);

        if has_tag {
            println!(
                "{:40} : {} tag {}   ✅",
                name,
                tag,
                ansi_term::Color::Green.paint("found")
            )
        } else {
            println!(
                "{:40} : {} tag {} ❌",
                name,
                tag,
                ansi_term::Color::Red.paint("missing")
            )
        }
        Ok(())
    })?;
    Ok(())
}

fn find_branch_commit<'a>(
    repo: &'a Repository,
    project: &'a Project,
    branch: &str,
    or_branch: Option<&str>,
) -> Result<git2::Commit<'a>, String> {
    match githelp::remote_resolve_branch(repo, &project.remote_name, branch) {
        Ok(commit) => Ok(commit),
        Err(_e1) => match or_branch {
            Some(or_branch) => {
                match githelp::remote_resolve_branch(repo, &project.remote_name, or_branch) {
                    Ok(commit2) => Ok(commit2),
                    Err(_e2) => Err(format!("branch {} or {} not available", branch, or_branch)),
                }
            }
            None => Err(format!("branch {} not available", branch)),
        },
    }
}

pub fn manifest_set_branch(
    app_params: &AppParams,
    name_branch: &str,
    commit: &str,
    skip_push: bool,
    continue_if_exists: bool,
) -> anyhow::Result<()> {
    on_project_repos(app_params, |project, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");

        let has_branch =
            match githelp::remote_resolve_branch(&repo, &project.remote_name, name_branch) {
                Ok(_commit) => true,
                Err(_e1) => false,
            };
        if has_branch {
            repo_report_error(name, &format!("branch {} already exist", name_branch));
            if !continue_if_exists {
                anyhow::bail!("branch exists")
            }
            return Ok(());
        }

        let target = match githelp::remote_resolve_branch(&repo, &project.remote_name, commit) {
            Ok(commit) => commit,
            Err(_e1) => {
                println!(
                    "fail to setup '{}' branch for {} : resolution of {} failed",
                    name_branch, project.remote_name, commit
                );
                return Ok(());
            }
        };

        match repo.branch(name_branch, &target, false) {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "fail to setup '{}' branch for {}: creating branch return error: {}",
                    name_branch, project.remote_name, e
                );
                return Ok(());
            }
        }

        println!(
            "{}: branching repo '{}' with commit {} (branch={})",
            name,
            name_branch,
            target.id(),
            commit
        );

        if skip_push {
            println!(
                "git --git-dir={}/.git push origin {}",
                dest_repo.into_os_string().into_string().unwrap(),
                name_branch
            );
        } else {
            git_push_to(
                &app_params,
                &project,
                &repo,
                PushSpecifier::Branch(name_branch),
            );
        }

        Ok(())
    })?;

    Ok(())
}

pub fn manifest_set_tag(
    app_params: &AppParams,
    branch: &str,
    tag: &str,
    skip_push: bool,
    continue_if_exists: bool,
    or_branch: Option<&str>,
) -> anyhow::Result<()> {
    // first chunk test that all repos are ok
    let _r = on_project_repos(app_params, |project, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");
        let has_tag = githelp::has_remote_tag(&repo, tag);

        if has_tag {
            repo_report_error(name, &format!("tag {} already exist", tag));
            if continue_if_exists {
                return Ok(());
            }
            anyhow::bail!("tag exists")
        }

        let _commit =
            find_branch_commit(&repo, project, branch, or_branch).expect("resolve branch");

        Ok(())
    })?;

    // then we re-loop over all repos, and tag/push then.
    on_project_repos(app_params, |project, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");

        let commit = find_branch_commit(&repo, project, branch, or_branch).expect("resolve branch");

        println!("{}: tagging repo with commit {}", name, commit.id());

        let dry_run = false;
        let target = commit.into_object();
        if dry_run {
            ()
        } else {
            repo.tag_lightweight(tag, &target, false)
                .expect("tag failed");
        }

        if skip_push {
            println!(
                "git --git-dir={}/.git push origin {}",
                dest_repo.into_os_string().into_string().unwrap(),
                tag
            );
            Ok(())
        } else {
            git_push_to(&app_params, &project, &repo, PushSpecifier::Tag(tag));
            Ok(())
        }
    })?;

    Ok(())
}

pub fn manifest_has_change(
    app_params: &AppParams,
    tag: &str,
    branch: &str,
    continue_on_fail: bool,
) -> anyhow::Result<()> {
    on_project_repos(app_params, |project, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");
        let has_tag = githelp::has_remote_tag(&repo, tag);
        let has_branch = githelp::has_remote_branch(&repo, &project.remote_name, branch);

        if !has_tag {
            if continue_on_fail {
                repo_report_error(name, &format!("tag {} is missing", tag));
                return Ok(());
            }
            anyhow::bail!("{}: tag {} is missing", name, tag)
        }
        if !has_branch {
            if continue_on_fail {
                repo_report_error(name, &format!("branch {} is missing", branch));
                return Ok(());
            }
            anyhow::bail!("{}: branch {} is missing", name, branch)
        }

        let all_tags = githelp::all_tags(&repo);
        let tag_oid = all_tags.get(tag).expect("existing tag");

        let to_find = format!("{}/{}", project.remote_name, branch);
        let branch = githelp::remote_branches_get_name(&repo, &to_find).expect("branch found");
        let branch_ref = branch.into_reference();
        let branch_commit = branch_ref.peel_to_commit().expect("branch oid found");

        let has_modification = &branch_commit.id() != tag_oid;

        if !has_modification {
            println!(
                "{:40} : {} ✅",
                name,
                ansi_term::Color::Green.paint("unmodified")
            )
        } else {
            println!(
                "{:40} : {}    ❌",
                name,
                ansi_term::Color::Red.paint("changed")
            )
        }
        Ok(())
    })?;
    Ok(())
}

pub fn manifest_changelog(
    app_params: &AppParams,
    rev1: &str,
    rev2: &str,
    continue_on_fail: bool,
    show_no_diff: bool,
) -> anyhow::Result<()> {
    on_project_repos(app_params, |project, dest_repo, name| {
        let repo = Repository::open(&dest_repo).expect("git repository");

        let all_tags = githelp::all_tags(&repo);

        let rev1_resolve = all_tags.get(rev1).cloned();
        let rev2_resolve = all_tags.get(rev2).cloned();

        let rev1_resolve = rev1_resolve.or_else(|| {
            githelp::remote_resolve_branch(&repo, &project.remote_name, rev1)
                .map(|c| c.id())
                .ok()
        });
        let rev2_resolve = rev2_resolve.or_else(|| {
            githelp::remote_resolve_branch(&repo, &project.remote_name, rev2)
                .map(|c| c.id())
                .ok()
        });

        if rev1_resolve.is_none() {
            if continue_on_fail {
                repo_report_error(name, &format!("revision {} is missing", rev1));
                return Ok(());
            }
            anyhow::bail!("{}: revision {} is missing", name, rev1)
        }

        if rev2_resolve.is_none() {
            if continue_on_fail {
                repo_report_error(name, &format!("revision {} is missing", rev2));
                return Ok(());
            }
            anyhow::bail!("{}: revision {} is missing", name, rev2)
        }

        let rev1 = rev1_resolve.unwrap();
        let rev2 = rev2_resolve.unwrap();

        let command = Command::new("git")
            .arg(format!(
                "--git-dir={}/.git",
                dest_repo.into_os_string().into_string().unwrap()
            ))
            .arg("log")
            .arg("--pretty=format:* %t (%ar) %s")
            .arg(format!("{}..{}", rev1, rev2))
            .output()
            .expect("failed to execute git process");
        let output = std::str::from_utf8(&command.stdout).expect("output is utf8");

        if output.is_empty() {
            if show_no_diff {
                println!("## no differences for {}", name);
                println!("");
            }
        } else {
            println!("## differences for {}", name);
            println!("{}", output);
            println!("");
        }
        Ok(())
    })?;
    Ok(())
}

pub fn manifest_sync(app_params: &AppParams) -> anyhow::Result<()> {
    let (_manifest, project) = app_params.get_project();

    let dest = app_params.get_destpath_create();

    let mut synced = BTreeSet::new();

    let number_root_repos = project.repos.len();

    for (repo_nb, repo_path) in project.repos.iter().enumerate() {
        let p = Path::new(&repo_path);
        let dest_repo = match p.file_name() {
            None => {
                println!(
                    "[{}/{}] {} {}",
                    repo_nb + 1,
                    ansi_term::Color::Yellow.paint("ignoring"),
                    number_root_repos,
                    repo_path
                );
                continue;
            }
            Some(s) => {
                let mut x = dest.to_path_buf();
                x.push(s);
                x
            }
        };
        let url = format!("{}{}", project.prefix, repo_path);

        synced.insert(dest_repo.clone());
        let repo = if dest_repo.exists() {
            println!(
                "[{}/{}] {} {:?} at {:?}",
                repo_nb + 1,
                number_root_repos,
                ansi_term::Color::Green.paint("syncing"),
                url,
                dest_repo
            );
            let repo = Repository::open(&dest_repo).expect("repo working");

            git_fetch_all(&app_params, &repo);
            repo
        } else {
            println!(
                "[{}/{}] {} {:?} at {:?}",
                repo_nb + 1,
                number_root_repos,
                ansi_term::Color::Red.paint("cloning"),
                url,
                dest_repo
            );

            git_clone(&app_params, &url, dest_repo.as_path())
        };

        for mut submodule in repo.submodules().expect("cannot update submodules") {
            let mut sub_repo_path = p.to_path_buf();
            sub_repo_path.push(submodule.url().expect("submodule has url"));
            let sub_repo_path = util::path_canon(&sub_repo_path);

            let sub_url = format!("{}{}", project.prefix, &sub_repo_path.to_str().unwrap());
            let mut sub_repo = dest_repo.clone();
            sub_repo.push(submodule.path());

            let mut sub_repo_git = sub_repo.clone();
            sub_repo_git.push(".git");

            //
            //submodule.update(true, None).expect("cloning submodules");

            if sub_repo_git.exists() {
                println!(
                    "  {} {:?} at {:?}",
                    ansi_term::Color::Blue.paint("updating submodule"),
                    sub_url,
                    sub_repo,
                );
                submodule.update(false, None).expect("submodule update")
            } else {
                println!(
                    "  {} {:?} at {:?}",
                    ansi_term::Color::Blue.paint("cloning submodule"),
                    sub_url,
                    sub_repo,
                );
                let _repo = git_clone(&app_params, &sub_url, &sub_repo);

                submodule
                    .update(false, None)
                    .expect("submodule update after clone")
            }
        }
    }

    let current_dest_content = std::fs::read_dir(dest).expect("read directory works");

    for entry in current_dest_content {
        match entry {
            Err(e) => {
                println!("{}: diff error {}", ansi_term::Color::Red.paint("error"), e);
            }
            Ok(dirent) => {
                let p = dirent.path();
                if !synced.contains(&p) {
                    println!(
                        "{} : {}",
                        p.to_str().unwrap_or("non unicode path"),
                        ansi_term::Color::Red.paint("directory is not maintained by synced"),
                    )
                }
            }
        }
    }

    Ok(())
}
