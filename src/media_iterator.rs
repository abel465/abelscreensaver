use crate::Options;
use auto_enums::auto_enum;
use mime_guess::mime;
use rand::{thread_rng, Rng};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::{fs, thread};
use walkdir::WalkDir;

pub struct RandomMediaIterator {
    rx: Receiver<PathBuf>,
}

impl RandomMediaIterator {
    pub fn new(opts: Options) -> Self {
        let (tx, rx) = sync_channel(3);

        thread::spawn(move || populate(opts, tx));

        Self { rx }
    }
}

impl std::iter::Iterator for RandomMediaIterator {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}

fn populate(opts: Options, tx: SyncSender<PathBuf>) {
    let mut dirs = VecDeque::from(opts.paths);
    let mut paths = vec![];
    let mut next = None;
    let mut rng = thread_rng();

    while let Some(dir) = dirs.pop_front() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(|x| x.ok()) {
                let file_name = entry.file_name();
                if opts.hidden || !is_hidden(file_name.as_os_str()) {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            dirs.push_back(entry.path());
                        } else if ft.is_file() && is_valid_media(file_name, opts.video) {
                            paths.push(entry.path());
                            if paths.len() > 9 {
                                if next.is_none() {
                                    next = loop {
                                        let i = rng.gen_range(0..paths.len());
                                        let target = paths.swap_remove(i);
                                        if ffprobe::ffprobe(&target).is_ok() {
                                            break Some(target);
                                        }
                                    }
                                }
                                match tx.try_send(next.take().unwrap()) {
                                    Ok(()) => {}
                                    Err(TrySendError::Full(x)) => next = Some(x),
                                    Err(TrySendError::Disconnected(_)) => return,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(target) = next {
        if ffprobe::ffprobe(&target).is_ok() {
            if tx.send(target).is_err() {
                return;
            }
        }
    }
    while !paths.is_empty() {
        let i = rng.gen_range(0..paths.len());
        let target = paths.swap_remove(i);
        if ffprobe::ffprobe(&target).is_ok() {
            if tx.send(target).is_err() {
                return;
            }
        }
    }
}

fn is_valid_media<P: AsRef<Path>>(file_name: P, include_video: bool) -> bool {
    mime_guess::from_path(file_name)
        .first()
        .is_some_and(|x| match (x.type_(), x.subtype()) {
            (_, mime::SVG) => false,
            (mime::VIDEO, _) => include_video,
            (mime::IMAGE, _) => true,
            _ => false,
        })
}

fn is_hidden(str: &OsStr) -> bool {
    str.to_str().unwrap().starts_with('.')
}

pub fn unspecified_media_iterator(opts: Options) -> impl Iterator<Item = PathBuf> {
    opts.paths.into_iter().flat_map(move |dir| {
        WalkDir::new(dir)
            .into_iter()
            .filter_entry(move |x| opts.hidden || !is_hidden(x.file_name()))
            .filter_map(|x| x.ok())
            .filter(move |x| {
                is_valid_media(x.file_name(), opts.video) && ffprobe::ffprobe(x.path()).is_ok()
            })
            .map(|x| x.into_path())
    })
}

pub fn random_media_iterator(opts: Options) -> RandomMediaIterator {
    RandomMediaIterator::new(opts)
}

#[auto_enum(Iterator)]
pub fn media_iterator(mut opts: Options) -> impl Iterator<Item = PathBuf> {
    let users_dirs = directories::UserDirs::new().unwrap();
    for path in &mut opts.paths {
        if path.starts_with("~/") {
            *path = PathBuf::from(
                users_dirs
                    .home_dir()
                    .join(&path.to_str().unwrap().to_string()[2..])
                    .to_str()
                    .unwrap(),
            );
        }
    }
    if opts.random {
        random_media_iterator(opts)
    } else {
        unspecified_media_iterator(opts)
    }
}
