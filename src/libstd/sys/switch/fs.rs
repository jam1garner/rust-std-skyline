use crate::ffi::{OsString, CStr};
use crate::fmt;
use crate::hash::Hash;
use crate::io::{self, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Path, PathBuf};
use crate::sys::time::{SystemTime, UNIX_EPOCH};
use crate::sys::{unsupported, Void};
use crate::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct FileAttr {
    size: AtomicU64,
    file_type: FileType
}

pub struct ReadDir(Void);

pub struct DirEntry(Void);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    read_only: bool
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FileType {
    Dir,
    File
}

#[derive(Debug)]
pub struct DirBuilder {}

impl Clone for FileAttr {
    fn clone(&self) -> Self {
        Self {
            size: AtomicU64::new(self.size.load(Ordering::SeqCst)),
            file_type: self.file_type
        }
    }
}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.size.load(Ordering::SeqCst)
    }

    pub fn set_size(&self, size: u64) {
        self.size.store(size, Ordering::SeqCst);
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions { read_only: false }
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.read_only
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        self.read_only = readonly;
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        if let FileType::Dir = self {
            true
        } else {
            false
        }
    }

    pub fn is_file(&self) -> bool {
        if let FileType::File = self {
            true
        } else {
            false
        }
    }

    pub fn is_symlink(&self) -> bool {
        false
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {}
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        match self.0 {}
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        match self.0 {}
    }

    pub fn file_name(&self) -> OsString {
        match self.0 {}
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        match self.0 {}
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        match self.0 {}
    }
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    flags: u64,
    truncate: bool
}

const READ_MODE: u64 = 1;
const WRITE_MODE: u64 = 2;
const APPEND_MODE: u64 = 4;

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            flags: 0,
            truncate: false
        }
    }

    pub fn read(&mut self, read: bool) {
        if read {
            self.flags |= READ_MODE;
        } else {
            self.flags &= !READ_MODE;
        }
    }
    pub fn write(&mut self, write: bool) {
        if write {
            self.flags |= WRITE_MODE;
        } else {
            self.flags &= !WRITE_MODE;
        }
    }
    pub fn append(&mut self, append: bool) {
        if append {
            self.flags |= APPEND_MODE;
        } else {
            self.flags &= !APPEND_MODE;
        }
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, _create: bool) {
        
    }

    pub fn create_new(&mut self, _create_new: bool) {
        panic!("File create new not supported yet")
    }
}

use nnsdk::fs::FileHandle;

pub struct File {
    inner: FileHandle,
    pos: AtomicU64,
    attr: FileAttr
}

use crate::ffi::CString;

impl crate::ops::Drop for File {
    fn drop(&mut self) {
        unsafe {
            nnsdk::fs::CloseFile(
                self.inner
            );
        }
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let path = CString::new(
            path.to_str()
                .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?
                .as_bytes()
        ).map_err(io::Error::from)?;


        let mut inner = FileHandle { handle: 0 as _ };

        let res = unsafe { 
            nnsdk::fs::OpenFile(
                &mut inner,
                path.as_ptr() as _,
                opts.flags as _
            )
        };

        if res != 0 {
            Err(io::Error::from_raw_os_error(res as _))
        } else if inner.handle.is_null() {
            Err(io::Error::new(io::ErrorKind::NotFound, "Returned file handle was null"))
        } else {
            let mut size = 0;
            let rc = unsafe { nnsdk::fs::GetFileSize(&mut size, inner) };
            if rc != 0 {
                return Err(io::Error::from_raw_os_error(rc as _));
            }
            let pos;
            if opts.flags & APPEND_MODE != 0 {
                pos = AtomicU64::new(size as u64);
            } else {
                pos = AtomicU64::new(0);
            }
            
            let attr = stat_internal(&path, size as u64)?;

            let file = File { inner, pos, attr };

            if opts.truncate {
                file.truncate(0)?;
            }

            Ok(file)
        }
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        Ok(self.attr.clone())
    }

    pub fn fsync(&self) -> io::Result<()> {
        let rc = unsafe { nnsdk::fs::FlushFile(self.inner) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(rc as _))
        }
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.fsync()
    }

    pub fn truncate(&self, size: u64) -> io::Result<()> {
        let rc = unsafe {
            nnsdk::fs::SetFileSize(self.inner, size as _)
        };

        self.attr.set_size(size);

        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(rc as _))
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut out_size = 0;
        let rc = unsafe {
            nnsdk::fs::ReadFile1(
                &mut out_size,
                self.inner,
                self.pos() as _,
                buf.as_ptr() as _,
                buf.len() as _
            )
        };

        if rc == 0 {
            self.pos.fetch_add(out_size, Ordering::SeqCst);
            Ok(out_size as usize)
        } else {
            Err(io::Error::from_raw_os_error(rc as _))
        }
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let mut read = 0;
        for mut buf in bufs {
            let amt = self.read(&mut buf)?;
            read += amt;
            if amt != buf.len() {
                break
            }
        }
        Ok(read)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let rc = unsafe {
            nnsdk::fs::WriteFile(
                self.inner,
                self.pos() as _,
                buf.as_ptr() as _,
                buf.len() as u64,
                &nnsdk::fs::WriteOption { flags: 0 }
            )
        };

        if rc == 0 {
            self.pos.fetch_add(buf.len() as u64, Ordering::SeqCst);
            if self.pos() > self.attr.size() {
                self.attr.set_size(self.pos());
            }
            Ok(buf.len())
        } else {
            Err(io::Error::from_raw_os_error(rc as _))
        }
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let mut written = 0;
        for buf in bufs {
            let amt = self.write(&buf)?;
            written += amt;
            if amt != buf.len() {
                break
            }
        }
        Ok(written)
    }

    pub fn flush(&self) -> io::Result<()> {
        self.fsync()
    }

    fn pos(&self) -> u64 {
        self.pos.load(Ordering::SeqCst)
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                self.pos.store(offset, Ordering::SeqCst);
            },
            SeekFrom::Current(offset) => {
                let pos = (self.pos.load(Ordering::SeqCst) as i64) + offset;
                if pos < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Attempted to seek to an invalid or negative offset"
                    ))
                }
                self.pos.store(pos as u64, Ordering::SeqCst);
            },
            SeekFrom::End(offset) => {
                if offset > 0 || (-offset as u64) > self.attr.size() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Attempted to seek to an invalid or negative offset"
                    ))
                }
                self.pos.store(self.attr.size() + (-offset as u64), Ordering::SeqCst);
            },
        };

        Ok(self.pos.load(Ordering::SeqCst))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        // This feels super wrong and will probably break something
        Ok(File {
            inner: self.inner.clone(),
            pos: AtomicU64::new(self.pos()),
            attr: self.attr.clone()
        })
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        Ok(())
    }

    pub fn diverge(&self) -> ! {
        panic!("file diverge")
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, _p: &Path) -> io::Result<()> {
        unsupported()
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Open File]")
    }
}

pub fn readdir(_p: &Path) -> io::Result<ReadDir> {
    unsupported()
}

pub fn unlink(_p: &Path) -> io::Result<()> {
    unsupported()
}

pub fn rename(_old: &Path, _new: &Path) -> io::Result<()> {
    unsupported()
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    Ok(())
}

pub fn rmdir(_p: &Path) -> io::Result<()> {
    unsupported()
}

pub fn remove_dir_all(_path: &Path) -> io::Result<()> {
    unsupported()
}

pub fn readlink(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

fn stat_internal(cstr: &CStr, size: u64) -> io::Result<FileAttr> {
    let mut entry_type: u32 = 0;

    let rc = unsafe {
        nnsdk::fs::GetEntryType(
            &mut entry_type,
            cstr.as_ptr() as _
        )
    };

    if rc != 0 {
        return Err(io::Error::from_raw_os_error(rc as _));
    }

    let file_type = match entry_type {
        0 => FileType::Dir,
        1 => FileType::File,
        _ => panic!("Invalid file type")
    };

    Ok(FileAttr {
        size: AtomicU64::new(size),
        file_type
    })
}

pub fn stat(path: &Path) -> io::Result<FileAttr> {
    File::open(path, &OpenOptions::new())?.file_attr()
}

pub fn lstat(_p: &Path) -> io::Result<FileAttr> {
    unsupported()
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn copy(_from: &Path, _to: &Path) -> io::Result<u64> {
    unsupported()
}
