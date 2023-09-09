use path_absolutize::*;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{arg, Arg, Command};
use ignore::gitignore;
use pathdiff::diff_paths;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

#[macro_use]
extern crate log;

fn cli() -> Command {
    Command::new("cpi")
        .about("Copy files with ignore-files applied")
        .arg(
            Arg::new("no-gitignore")
                .long("no-gitignore")
                .help("Disable using .gitignore file for excluding")
                .default_value("false")
                .action(clap::ArgAction::SetFalse),
        )
        .arg(arg!(-f --force "Overwrite destination if existed"))
        .arg(arg!(<SRC> "Srcource directory path"))
        .arg(
            arg!(<DEST> r#"Destination directory path or zip file name if ends with .zip
Examples: - cpi . ./dest
          - cpi . ./dest.zip
"#),
        )
}

fn main() {
    env_logger::init();

    let matches = cli().get_matches();
    let src = matches.get_one::<String>("SRC").unwrap();
    let dest = matches.get_one::<String>("DEST").unwrap();

    let force_copy = matches.get_flag("force");

    let src_path = PathBuf::from(src);
    let dest_path = PathBuf::from(dest);
    if !src_path.exists() {
        error!("\"{}\" not exists", src);
        exit(1);
    }

    if dest_path.exists() {
        if force_copy {
            if dest_path.is_dir() {
                info!("removed existing directory");
                fs::remove_dir_all(&dest_path).unwrap();
            }
            if dest_path.is_file() {
                info!("removed existing zip file");
                fs::remove_file(&dest_path).unwrap();
            }
        } else {
            error!("\"{}\" already exists", dest);
            exit(1);
        }
    }

    let enable_gitignore = !matches.get_flag("no-gitignore");
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

    let dest_full_path = dest_path.absolutize().unwrap();

    let generate_zip = dest.ends_with(".zip");
    let mut zip_writer = None;
    if generate_zip {
        let zip_file = File::create(&dest_path).unwrap();
        zip_writer = Some(ZipWriter::new(zip_file));
        info!("zip enabled");
    } else {
        if !dest_path.exists() {
            fs::create_dir(dest_path.clone()).unwrap();
            info!("created destination directory");
        }
    }

    let mut dir_queue: VecDeque<PathBuf> = VecDeque::from([src_path.clone()]);
    while let Some(ref path) = dir_queue.pop_front() {
        if &path.canonicalize().unwrap() == dest_full_path.as_ref() {
            debug!("skipped destination item {:?} from copying", path);
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
                                        let ignored = gitignore_matcher
                                            .matched(file_path.clone(), file_type.is_dir());
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
                                            if file_path == dest_path {
                                                debug!("skipped self zip file from copying",);
                                                continue;
                                            }
                                            copy_file(
                                                src_path.clone(),
                                                file_path.clone(),
                                                dest_path.clone(),
                                                &mut zip_writer,
                                            );
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

    if let Some(ref mut zip_writer) = zip_writer {
        match zip_writer.finish() {
            Ok(_) => {}
            Err(err) => {
                error!("failed generatin zip: {err:?}");
                exit(1);
            }
        }
    }
}

fn copy_file<P, B, C>(
    src_base_dir: P,
    src_file_full_path: B,
    dest_base_dir: C,
    zip_writer: &mut Option<ZipWriter<File>>,
) where
    P: AsRef<Path> + Debug,
    B: AsRef<Path> + Debug,
    C: AsRef<Path> + Debug,
{
    let relatve_path = diff_paths(src_file_full_path.as_ref(), src_base_dir.as_ref());

    match relatve_path {
        Some(relatve_path) => {
            let dest = PathBuf::from(dest_base_dir.as_ref()).join(&relatve_path);
            let dest_parent = dest.parent().unwrap();

            match zip_writer {
                Some(ref mut zip_writer) => {
                    let options =
                        FileOptions::default().compression_method(CompressionMethod::DEFLATE);

                    let _ = zip_writer.start_file(relatve_path.to_string_lossy(), options);

                    match File::open(&src_file_full_path) {
                        Ok(mut file) => {
                            let mut buffer = Vec::new();
                            io::copy(&mut file, &mut buffer);
                            // fs::copy(from, to)
                            zip_writer.write_all(&buffer);
                        }
                        Err(err) => {
                            error!("cannot open {:?} for compressing", src_file_full_path);
                            exit(1)
                        }
                    }
                }
                None => {
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
            }
        }
        None => {
            warn!("getting relative path failed");
        }
    }
}

#[test]
fn cmd_test() {
    let cmd = cli();
    let matches = cmd
        .try_get_matches_from(["cpi", ".", "./dest.zip", "--force"])
        .unwrap();
    assert!(matches.contains_id("force"));
    assert_eq!(matches.get_flag("force"), true);

    let cmd = cli();
    let matches = cmd
        .try_get_matches_from(["cpi", ".", "./dest.zip"])
        .unwrap();
    assert_eq!(matches.get_flag("force"), false);
}
