use anyhow::Result;
use std::{
    collections::HashMap,
    fs,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};

pub struct Merger {
    indexed: usize,
    merger_type: MergerType,
    pos: usize,
    size: usize,
    stored_bytes: usize,
    flushed_bytes: usize,
    state_path: Option<PathBuf>,
}

enum MergerType {
    Directory(PathBuf),
    File((fs::File, HashMap<usize, Vec<u8>>)),
}

impl Merger {
    pub fn new_file(size: usize, path: &PathBuf) -> Result<Self> {
        let state_path = path.with_extension(format!(
            "{}.vsdstate",
            path.extension().unwrap().to_string_lossy()
        ));
        let mut pos = 0;
        let mut flushed_bytes = 0;

        if state_path.exists() {
            if let Ok(data) = fs::read(&state_path)
                && data.len() == 16
            {
                pos = u64::from_be_bytes(data[0..8].try_into().unwrap()) as usize;
                flushed_bytes = u64::from_be_bytes(data[8..16].try_into().unwrap()) as usize;
            }
        }

        let file = if pos > 0 {
            if let Ok(mut f) = fs::OpenOptions::new().write(true).open(path) {
                let _ = f.seek(SeekFrom::Start(flushed_bytes as u64));
                Some(f)
            } else {
                pos = 0;
                flushed_bytes = 0;
                None
            }
        } else {
            None
        };

        let file = if let Some(f) = file {
            f
        } else {
            if pos > 0 && state_path.exists() {
                let _ = fs::remove_file(&state_path);
            }
            pos = 0;
            flushed_bytes = 0;
            fs::File::create(path)?
        };

        Ok(Self {
            indexed: pos,
            merger_type: MergerType::File((file, HashMap::new())),
            pos,
            size: size - 1,
            stored_bytes: flushed_bytes,
            flushed_bytes,
            state_path: Some(state_path),
        })
    }

    pub fn new_directory(size: usize, path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        Ok(Self {
            indexed: 0,
            merger_type: MergerType::Directory(path.to_owned()),
            pos: 0,
            stored_bytes: 0,
            flushed_bytes: 0,
            size: size - 1,
            state_path: None,
        })
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn buffered(&self) -> bool {
        let buffers_empty = match &self.merger_type {
            MergerType::Directory(_) => true,
            MergerType::File((_, buffers)) => buffers.is_empty(),
        };
        buffers_empty && self.pos >= (self.size + 1)
    }

    pub fn flush(&mut self) -> Result<()> {
        if let MergerType::File((file, buffers)) = &mut self.merger_type {
            let mut updated = false;

            while self.pos <= self.size {
                if let Some(buf) = buffers.remove(&self.pos) {
                    file.write_all(&buf)?;
                    file.flush()?;
                    self.pos += 1;
                    self.flushed_bytes += buf.len();
                    updated = true;
                } else {
                    break;
                }
            }

            if updated {
                self.update_state()?;

                if self.pos > self.size && let Some(state_path) = &self.state_path {
                    let _ = fs::remove_file(state_path);
                }
            }
        }

        Ok(())
    }

    fn update_state(&self) -> Result<()> {
        if let Some(state_path) = &self.state_path {
            let mut data = [0u8; 16];
            data[0..8].copy_from_slice(&(self.pos as u64).to_be_bytes());
            data[8..16].copy_from_slice(&(self.flushed_bytes as u64).to_be_bytes());
            fs::write(state_path, data)?;
        }
        Ok(())
    }

    pub fn estimate(&self) -> usize {
        if self.indexed == 0 {
            0
        } else {
            (self.stored_bytes / self.indexed) * (self.size + 1)
        }
    }

    pub fn stored(&self) -> usize {
        self.stored_bytes
    }

    pub fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        match &mut self.merger_type {
            MergerType::Directory(path) => {
                let mut file = fs::File::create(path.join(format!(
                    "{}.{}",
                    pos,
                    path.extension().unwrap().to_string_lossy()
                )))?;
                file.write_all(buf)?;
                self.pos += 1;
                self.stored_bytes += buf.len();
            }
            MergerType::File((file, buffers)) => {
                if pos == 0 || (self.pos != 0 && self.pos == pos) {
                    file.write_all(buf)?;
                    file.flush()?;
                    self.pos += 1;
                    self.stored_bytes += buf.len();
                    self.flushed_bytes += buf.len();
                    self.update_state()?;
                } else {
                    buffers.insert(pos, buf.to_vec());
                    self.stored_bytes += buf.len();
                }
            }
        };

        self.indexed += 1;
        Ok(())
    }
}
