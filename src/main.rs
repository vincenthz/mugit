//use anyhow::anyhow;
use clap::{App, Arg};

mod commands;
mod githelp;
mod manifest;
mod project;
mod util;
mod ver;

use commands::*;

fn init() -> Result<Option<manifest::Manifests>, std::io::Error> {
    #[allow(deprecated)]
    let home_dir = std::env::home_dir().expect("HOME is defined");

    let mugit_dir = home_dir.join(".mugit");
    if !mugit_dir.is_dir() {
        return Ok(None);
    }
    manifest::read_manifests(mugit_dir).map(Some)
}

fn main() {
    const ARG_GIT_EXEC: &str = "git-exec";
    const ARG_INIT_SKIP_LOAD: &str = "init-skip-load";
    const ARG_REPO: &str = "repo";
    const ARG_SPEC: &str = "spec";
    const ARG_PROJECT: &str = "project";
    const ARG_BRANCH: &str = "branch";
    const ARG_COMMIT: &str = "commit";
    const ARG_OR_BRANCH: &str = "or-branch";
    const ARG_TAG: &str = "tag";
    const ARG_CONTINUE_ON_FAIL: &str = "continue-on-fail";
    const ARG_CONTINUE_IF_EXISTS: &str = "continue-if-exists";
    const ARG_SKIP_PUSH: &str = "skip-push";
    const ARG_REMOTE_NAME: &str = "remote-name";
    const ARG_REV1: &str = "rev1";
    const ARG_REV2: &str = "rev2";
    const ARG_MANIFEST_FILE: &str = "manifest-file";
    const ARG_MANIFEST_DEST: &str = "manifest-dest";

    const SUBCMD_VERSION_FIND: &str = "version-find";
    const SUBCMD_HAS_BRANCH: &str = "has-branch";
    const SUBCMD_MANIFEST_SYNC: &str = "manifest-sync";
    const SUBCMD_MANIFEST_DEBUG: &str = "manifest-debug";
    const SUBCMD_MANIFEST_HAS_BRANCH: &str = "manifest-has-branch";
    const SUBCMD_MANIFEST_HAS_TAG: &str = "manifest-has-tag";
    const SUBCMD_MANIFEST_SET_BRANCH: &str = "manifest-set-branch";
    const SUBCMD_MANIFEST_SET_TAG: &str = "manifest-set-tag";
    const SUBCMD_MANIFEST_HAS_CHANGE: &str = "manifest-has-change";
    const SUBCMD_MANIFEST_CHANGELOG: &str = "manifest-changelog";

    let arg_repo = Arg::new(ARG_REPO)
        .short('r')
        .long("repo")
        .value_name("REPOSITORY-PATH")
        .default_value("./")
        .about("Sets the path to repository")
        .takes_value(true);

    let arg_project = Arg::new(ARG_PROJECT)
        .short('p')
        .long("project")
        .value_name("PROJECT")
        .about("Sets the project")
        .takes_value(true)
        .required(false);

    let arg_manifest_file = Arg::new(ARG_MANIFEST_FILE)
        .takes_value(true)
        .long("manifest")
        .value_name("MANIFEST-FILE")
        .required(false);
    let arg_manifest_dest = Arg::new(ARG_MANIFEST_DEST)
        .takes_value(true)
        .long("output-dir")
        .value_name("OUTPUT-DIR")
        .required(false);

    let arg_continue_on_fail = Arg::new(ARG_CONTINUE_ON_FAIL)
        .long("continue-on-fail")
        .about("continue if something fails")
        .takes_value(false);

    let arg_continue_if_exists = Arg::new(ARG_CONTINUE_IF_EXISTS)
        .long("continue-if-exists")
        .about("continue if tag already exists")
        .takes_value(false);

    let arg_skip_push = Arg::new(ARG_SKIP_PUSH)
        .long("skip-push")
        .about("Don't push, only print")
        .takes_value(false);

    let arg_tag = |s| {
        Arg::new(ARG_TAG)
            .about(s)
            .takes_value(true)
            .required(true)
            .multiple(false)
    };
    let arg_branch = |s| {
        Arg::new(ARG_BRANCH)
            .about(s)
            .takes_value(true)
            .required(true)
            .multiple(false)
    };

    let arg_commit = |s| {
        Arg::new(ARG_COMMIT)
            .about(s)
            .takes_value(true)
            .required(true)
            .multiple(false)
    };

    let app = App::new("hexgit")
        .version("1.0")
        .author("HexDev")
        .about("git helper")
        .arg(
            Arg::new(ARG_GIT_EXEC)
                .about("use git executable instead of libgit2 for cloning,fetching and pushing")
                .long("git-exec")
                .takes_value(false),
        )
        .arg(
            Arg::new(ARG_INIT_SKIP_LOAD)
                .long("skip-init-load")
                .about("skip initialiation loading of the config file"),
        )
        .subcommand(
            App::new(SUBCMD_VERSION_FIND)
                .arg(&arg_repo)
                .arg(Arg::new(ARG_SPEC).short('s').long("spec").takes_value(true)),
        )
        .subcommand(
            App::new(SUBCMD_HAS_BRANCH)
                .arg(&arg_repo)
                .arg(
                    Arg::new(ARG_REMOTE_NAME)
                        .about("specify remote name")
                        .takes_value(true)
                        .required(true)
                        .multiple(false),
                )
                .arg(
                    Arg::new(ARG_BRANCH)
                        .about("specify which branch to find")
                        .takes_value(true)
                        .required(true)
                        .multiple(false),
                ),
        )
        .subcommand(App::new(SUBCMD_MANIFEST_DEBUG).arg(&arg_manifest_file))
        .subcommand(
            App::new(SUBCMD_MANIFEST_HAS_BRANCH)
                .arg(&arg_project)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                /*
                .arg(
                    Arg::new(ARG_REMOTE_NAME)
                        .about("specify remote name")
                        .takes_value(true)
                        .multiple(false)
                        .required(true),
                )
                */
                .arg(arg_branch("specify which branch to find")),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_HAS_TAG)
                .arg(&arg_project)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                .arg(arg_tag("specify which tag to find")),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_SET_BRANCH)
                .arg(&arg_skip_push)
                .arg(&arg_project)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                .arg(&arg_continue_if_exists)
                .arg(arg_branch("specify which branch to set to the project"))
                .arg(arg_commit("specify which commit"))
                .arg(
                    Arg::new(ARG_OR_BRANCH)
                        .about("set a backup branch if branch is not found")
                        .long("or-branch")
                        .required(false)
                        .takes_value(true),
                ),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_SET_TAG)
                .arg(&arg_skip_push)
                .arg(&arg_project)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                .arg(&arg_continue_if_exists)
                .arg(arg_tag("specify which tag to set to the project"))
                .arg(arg_branch("specify which branch the tag apply to"))
                .arg(
                    Arg::new(ARG_OR_BRANCH)
                        .about("set a backup branch if branch is not found")
                        .long("or-branch")
                        .required(false)
                        .takes_value(true),
                ),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_HAS_CHANGE)
                .arg(&arg_project)
                .arg(&arg_continue_on_fail)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                .arg(arg_tag("specify which reference to use"))
                .arg(arg_branch("specify which branch to compare reference to")),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_CHANGELOG)
                .arg(&arg_project)
                .arg(&arg_continue_on_fail)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest)
                .arg(
                    Arg::new(ARG_REV1)
                        .about("old revision to start")
                        .required(true)
                        .takes_value(true)
                        .multiple(false),
                )
                .arg(
                    Arg::new(ARG_REV2)
                        .about("newer revision to end with")
                        .required(true)
                        .takes_value(true)
                        .multiple(false),
                ),
        )
        .subcommand(
            App::new(SUBCMD_MANIFEST_SYNC)
                .arg(&arg_project)
                .arg(&arg_manifest_file)
                .arg(&arg_manifest_dest),
        );

    let mut help_bytes = Vec::new();
    app.clone().write_help(&mut help_bytes).unwrap();
    let help = String::from_utf8(help_bytes).expect("help is utf8");

    let matches = app.get_matches();
    let git_exec = matches.is_present(ARG_GIT_EXEC);
    let skip_load = matches.is_present(ARG_INIT_SKIP_LOAD);
    let manifests = if skip_load { None } else { init().unwrap() };

    let mut app_params = AppParams {
        git_exec,
        sys_manifests: std::sync::Arc::new(manifests),
        manifest_file: None,
        manifest_selector: None,
        manifest_dest: None,
    };

    // helper commands unrelated to the main tool which is about multiple gits
    if let Some(m) = matches.subcommand_matches(SUBCMD_VERSION_FIND) {
        let repo_path = m.value_of(ARG_REPO).unwrap();
        let spec = m.value_of(ARG_SPEC);
        version_find(repo_path, spec);
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_HAS_BRANCH) {
        let repo_path = m.value_of(ARG_REPO).unwrap();
        let remote_name = m.value_of(ARG_REMOTE_NAME).unwrap();
        let branch = m.value_of(ARG_BRANCH).unwrap();
        has_remote_branch(repo_path, remote_name, branch)
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_DEBUG) {
        let manifest_file = m.value_of(ARG_MANIFEST_FILE).unwrap();
        manifest_debug(manifest_file)
    }

    fn set_manifest_options(app_params: &mut AppParams, m: &clap::ArgMatches) {
        app_params.manifest_file = m.value_of(ARG_MANIFEST_FILE).map(|x| x.into());
        app_params.manifest_dest = m.value_of(ARG_MANIFEST_DEST).map(|x| x.into());
        app_params.manifest_selector = m.value_of(ARG_PROJECT).map(|x| x.to_owned());
    }

    // multiple repositories commands
    if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_SYNC) {
        set_manifest_options(&mut app_params, m);
        manifest_sync(&app_params).unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_HAS_BRANCH) {
        set_manifest_options(&mut app_params, m);
        let branch = m.value_of(ARG_BRANCH).unwrap();
        manifest_has_branch(&app_params, branch).unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_HAS_TAG) {
        set_manifest_options(&mut app_params, m);
        let tag = m.value_of(ARG_TAG).unwrap();
        manifest_has_tag(&app_params, tag).unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_SET_TAG) {
        set_manifest_options(&mut app_params, m);
        let branch = m.value_of(ARG_BRANCH).unwrap();
        let or_branch = m.value_of(ARG_OR_BRANCH);
        let tag = m.value_of(ARG_TAG).unwrap();
        let skip_push = m.is_present(ARG_SKIP_PUSH);
        let continue_if_exists = m.is_present(ARG_CONTINUE_IF_EXISTS);
        manifest_set_tag(
            &app_params,
            branch,
            tag,
            skip_push,
            continue_if_exists,
            or_branch,
        )
        .unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_SET_BRANCH) {
        set_manifest_options(&mut app_params, m);
        let branch = m.value_of(ARG_BRANCH).unwrap();
        let commit = m.value_of(ARG_COMMIT).unwrap();
        let skip_push = m.is_present(ARG_SKIP_PUSH);
        let continue_if_exists = m.is_present(ARG_CONTINUE_IF_EXISTS);
        manifest_set_branch(&app_params, branch, commit, skip_push, continue_if_exists).unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_HAS_CHANGE) {
        set_manifest_options(&mut app_params, m);
        let tag = m.value_of(ARG_TAG).unwrap();
        let branch = m.value_of(ARG_BRANCH).unwrap();
        let continue_on_fail = m.is_present(ARG_CONTINUE_ON_FAIL);
        manifest_has_change(&app_params, tag, branch, continue_on_fail).unwrap()
    } else if let Some(m) = matches.subcommand_matches(SUBCMD_MANIFEST_CHANGELOG) {
        set_manifest_options(&mut app_params, m);
        let rev1 = m.value_of(ARG_REV1).unwrap();
        let rev2 = m.value_of(ARG_REV2).unwrap();
        let continue_on_fail = m.is_present(ARG_CONTINUE_ON_FAIL);
        manifest_changelog(&app_params, rev1, rev2, continue_on_fail).unwrap()
    } else if let Some(name) = matches.subcommand_name() {
        println!("error: unknown command {}\n\n{}", name, help)
    } else {
        println!("error: no subcommand specified\n\n{}", help)
    }
}
