use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{arg, Command, Arg};
use pathdiff::diff_paths;
use ignore::gitignore;

#[macro_use]
extern crate log;

fn main() {
    env_logger::init();

    let matches = cli().get_matches();
    let src = matches.get_one::<String>("SRC").unwrap();
    let dest = matches.get_one::<String>("DEST").unwrap();

    let src_path = PathBuf::from(src);
    let dest_path = PathBuf::from(dest);
    if !src_path.exists() {
        error!("\"{}\" not exists", src);
        exit(1);
    }
    if dest_path.exists() {
        error!("\"{}\" already exists", dest);
        exit(1);
    }

    let enable_gitignore = !matches
        .get_flag("no-gitignore");
    let mut gitignore_matcher = None;
    if enable_gitignore {
        let mut ignore_path = src_path.clone();
        ignore_path.push(".gitignore");
        if !ignore_path.exists() {
            warn!("cannot find .gitignore file");
        } else {
            info!("enabled .gitignore");
            let mut builder = gitignore::GitignoreBuilder::new(src_path.clone());
            builder.add(ignore_path);
            gitignore_matcher = Some(builder.build().unwrap());
        }
    }

    if !dest_path.exists() {
        fs::create_dir(dest_path.clone()).unwrap();
        info!("created destination directory");
    }
    let dest_full_path = dest_path.canonicalize().unwrap();

    let mut dir_queue: VecDeque<PathBuf> = VecDeque::from([src_path.clone()]);
    while let Some(ref path) = dir_queue.pop_front() {
        // don't copy if dest directory was inside src directory
        if path.canonicalize().unwrap() == dest_full_path {
            debug!("skipped dest folder {:?} from copying", path);
            continue;
        }

        match fs::read_dir(path) {
            Ok(files) => {
                debug!("reading directory: {:?}", path);

                for file in files {
                    match file {
                        Ok(file) => {
                            match file.file_type() {
                                Ok(file_type) => {
                                    // println!("Processing {:?}", file);

                                    let file_path = file.path();
                                    let mut is_ignored = false;
                                    // use ref to avoid moving issue
                                    if let Some(ref gitignore_matcher) = gitignore_matcher {
                                        let ignored =
                                            gitignore_matcher.matched(file_path.clone(), file_type.is_dir());
                                            // println!("Check ignoring: {:?}", ignored);
                                        match ignored {
                                            ignore::Match::Ignore(_) => {
                                                is_ignored = true;
                                                debug!("ignored {:?}", file_path);
                                            }
                                            _ => {}
                                        }
                                    }

                                    if !is_ignored {
                                        if file_type.is_dir() {
                                            // println!("Pushed dir {:?}", file_path);
                                            dir_queue.push_back(file_path.clone());
                                        } else {
                                            copy_file(src_path.clone(), file_path.clone(), dest_path.clone());
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!("getting file type error: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            warn!("iterating error: {err}");
                        }
                    }
                }
            }
            Err(err) => {
                panic!("error while reading directory of {src}: {err}");
            }
        }
    }

}

fn cli() -> Command {
    Command::new("cpi")
        .about("Copy files with ignore-files applied")
        .arg(Arg::new("no-gitignore").long("no-gitignore").help("Disable using .gitignore file for excluding").default_value("false").action(clap::ArgAction::SetFalse))
        .arg(arg!(<SRC> "Srcource directory path"))
        .arg(arg!(<DEST> "Destination directory path"))
}

fn copy_file<P, B, C>(src_base_dir: P, src_file_full_path: B, dest_base_dir: C)
where
    P: AsRef<Path> + Debug,
    B: AsRef<Path> + Debug,
    C: AsRef<Path> + Debug,
{
    let relatve_path = diff_paths(src_file_full_path.as_ref(), src_base_dir.as_ref());

    match relatve_path {
        Some(relatve_path) => {
            let dest = PathBuf::from(dest_base_dir.as_ref()).join(relatve_path);
            let dest_parent = dest.parent().unwrap();

            let copy_file = |dest: PathBuf| {
                match fs::copy(src_file_full_path.as_ref(), dest) {
                    Ok(_) => {
                        // println!("Copied {:?}", src_file_full_path.as_ref());
                    }
                    Err(err) => {
                        warn!("copy file error: {err}");
                    }
                };
            };
            if !dest_parent.exists() {
                match fs::create_dir_all(dest_parent) {
                    Ok(()) => {
                        copy_file(dest);
                    }
                    Err(err) => {
                        warn!("failed creating directory for \"{:?}\": {err}", dest_parent);
                    }
                }
            } else {
                copy_file(dest);
            }
        }
        None => {
            warn!("getting relative path failed");
        }
    }
}
