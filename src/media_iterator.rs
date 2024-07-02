use crate::Opt;
use auto_enums::auto_enum;
use mime_guess::mime;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, thread};
use walkdir::WalkDir;

enum Message {
    Continue,
    Done,
}

#[derive(Clone, Copy)]
struct RandomMediaOpts {
    all: bool,
}

impl RandomMediaOpts {
    pub fn new(all: bool) -> Self {
        Self { all }
    }
}

struct RandomMediaData {
    keys: Vec<usize>,
    values: Vec<PathBuf>,
    indices: Vec<usize>,
    count: usize,
}

impl RandomMediaData {
    pub fn new() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            indices: vec![],
            count: 0,
        }
    }

    fn get_random(&mut self, rng: &mut ThreadRng) -> (usize, PathBuf) {
        let i = rng.gen_range(0..self.count);
        self.count -= 1;
        let n = self.indices.swap_remove(i);
        let key = bisection::bisect_right(&self.keys, &n) - 1;
        (n - self.keys[key], self.values[key].clone())
    }
}

pub struct RandomMediaIterator {
    data: Arc<Mutex<RandomMediaData>>,
    opts: RandomMediaOpts,
    rx: Receiver<Message>,
    rng: ThreadRng,
}

impl RandomMediaIterator {
    pub fn new(opts: Opt) -> Self {
        let (tx, rx) = channel();
        let data = Arc::new(Mutex::new(RandomMediaData::new()));
        let data_copy = data.clone();
        let media_opts = RandomMediaOpts::new(opts.all);

        thread::spawn(move || populate(data_copy, opts, tx));

        Self {
            data,
            opts: media_opts,
            rx,
            rng: thread_rng(),
        }
    }
}

impl std::iter::Iterator for RandomMediaIterator {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        'a: loop {
            while self.data.lock().unwrap().count == 0 {
                if let Ok(Message::Done) = self.rx.recv() {
                    break 'a;
                }
            }
            let (target, dir) = self.data.lock().unwrap().get_random(&mut self.rng);
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut count = 0;
                for entry in entries.filter_map(|x| x.ok()) {
                    let file_name = entry.file_name();
                    if (self.opts.all || !is_hidden(file_name.as_os_str()))
                        && entry.file_type().map_or(false, |x| x.is_file())
                        && is_valid_media(file_name)
                    {
                        if count == target {
                            return Some(entry.path());
                        }
                        count += 1;
                    }
                }
            }
        }
        None
    }
}

fn populate(data: Arc<Mutex<RandomMediaData>>, opts: Opt, tx: Sender<Message>) {
    let mut dirs = VecDeque::from(opts.paths);

    while let Some(dir) = dirs.pop_front() {
        if let Ok(entries) = fs::read_dir(&dir) {
            let mut count = 0;
            for entry in entries.filter_map(|x| x.ok()) {
                let file_name = entry.file_name();
                if opts.all || !is_hidden(file_name.as_os_str()) {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            dirs.push_back(entry.path());
                        } else if ft.is_file() && is_valid_media(file_name) {
                            count += 1;
                        }
                    }
                }
            }
            let mut data = data.lock().unwrap();
            let current_count = data.count;
            data.keys.push(current_count);
            data.values.push(dir);
            data.indices.extend(current_count..current_count + count);
            data.count += count;

            tx.send(Message::Continue).ok();
        }
    }

    tx.send(Message::Done).ok();
}

fn is_valid_media<P: AsRef<Path>>(path: P) -> bool {
    mime_guess::from_path(path)
        .first()
        .map_or(false, |x| matches!(x.type_(), mime::IMAGE | mime::VIDEO))
}

fn is_hidden(str: &OsStr) -> bool {
    str.to_str().map_or(true, |s| s.starts_with('.'))
}

pub fn unspecified_media_iterator(opts: Opt) -> impl Iterator<Item = PathBuf> {
    opts.paths.into_iter().flat_map(move |dir| {
        WalkDir::new(dir)
            .into_iter()
            .filter_entry(move |x| opts.all || !is_hidden(x.file_name()))
            .filter_map(|x| x.ok())
            .filter(|x| is_valid_media(x.file_name()))
            .map(|x| x.into_path())
    })
}

pub fn random_media_iterator(opts: Opt) -> RandomMediaIterator {
    RandomMediaIterator::new(opts)
}

#[auto_enum(Iterator)]
pub fn media_iterator(opts: Opt) -> impl Iterator<Item = PathBuf> {
    if opts.random {
        random_media_iterator(opts)
    } else {
        unspecified_media_iterator(opts)
    }
}
