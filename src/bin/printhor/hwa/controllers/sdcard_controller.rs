//! TODO: Pending to review after intense refactor

use core::pin::Pin;
use futures::task::{Context, Poll};
use embedded_sdmmc::{Directory, DirEntry, File, Mode, TimeSource, Timestamp, Volume, VolumeIdx, VolumeManager};
use futures::Stream;
use heapless;
use printhor_hwa_common::TrackedStaticCell;
use crate::hwa;
use crate::alloc::string::ToString;
use printhor_hwa_common::ControllerMutex;
use futures::Future;

const MAX_DIRS: usize = 3usize;
const MAX_FILES: usize = 1usize;

#[cfg(feature = "sdcard-uses-spi")]
pub type SDCardBlockDevice = hwa::adapters::SPIAdapter<hwa::device::SpiCardCSPin>;
#[cfg(not(feature = "sdcard-uses-spi"))]
pub type SDCardBlockDevice = hwa::device::SDCardBlockDevice;


#[allow(unused)]
#[derive(Debug)]
pub enum SDCardError {
    NoSuchVolume,
    InternalError,
    NoDirectorySpecified,
    #[allow(unused)]
    NotYetImplemented,
    MaxOpenDirs,
    InconsistencyError,
    TrailingEntries,
    NotFound,
}

pub type SDCardVolumeManager = VolumeManager<SDCardBlockDevice, DummyTimeSource, MAX_DIRS, MAX_FILES>;

/// Helper adaption for embedded_sdmmc::VolumeManager to manage open count and resolve paths
pub struct SDCard {
    mgr: SDCardVolumeManager,
    vol: Option<Volume>,
    pub(crate) opened_dir_slots: heapless::Vec<Option<Directory>, MAX_DIRS>,
    pub(crate) opened_dir_refcount: heapless::Vec<u8, MAX_DIRS>,
    /// Full paths as of now...
    pub(crate) opened_dir_names: heapless::Vec<Option<alloc::string::String>, MAX_DIRS>,
}

pub struct DirectoryRef {
    idx: u8,
}

#[allow(unused)]
impl SDCard {

    pub(crate) async fn retain(&mut self) {
        #[cfg(feature = "sdcard-uses-spi")]
        self.mgr.device().retain().await;
    }

    pub(crate) async fn release(&mut self) {
        #[cfg(feature = "sdcard-uses-spi")]
        self.mgr.device().release().await;
    }

    pub(crate) fn open_root_dir(&mut self) -> Result<DirectoryRef, SDCardError> {
        match self.vol.as_ref() {
            Some(vol) => {
                match self.mgr.open_root_dir(vol) {
                    Ok(directory) => {
                        let mut idx = 0u8;
                        for refcount in &self.opened_dir_refcount {
                            if *refcount == 0 {
                                break;
                            }
                            idx += 1;
                        }
                        if idx < (self.opened_dir_refcount.len() as u8 ){
                            self.opened_dir_refcount[idx as usize] += 1;
                            self.opened_dir_slots[idx as usize] = Some(directory);
                            self.opened_dir_names[idx as usize] = None;
                            Ok(DirectoryRef{idx})
                        }
                        else {
                            self.mgr.close_dir(vol, directory);
                            Err(SDCardError::MaxOpenDirs)
                        }
                    }
                    Err(reason) => {
                        match reason {
                            embedded_sdmmc::Error::DirAlreadyOpen => {
                                let mut idx = 0u8;
                                for dirname in &self.opened_dir_names {
                                    if dirname.is_none() {
                                        break;
                                    }
                                    idx += 1;
                                }
                                if idx < (self.opened_dir_refcount.len() as u8 ){
                                    self.opened_dir_refcount[idx as usize] += 1;
                                    Ok(DirectoryRef{idx})
                                }
                                else {
                                    Err(SDCardError::InconsistencyError)
                                }
                            }
                            _ => {
                                Err(SDCardError::InternalError)
                            }
                        }
                    }
                }
            }
            None => {
                Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) fn open_dir(&mut self, parent_dir_ref: &DirectoryRef, name: &str) -> Result<DirectoryRef, SDCardError> {
        match self.vol.as_ref() {
            Some(vol) => {
                let parent_idx = parent_dir_ref.idx as usize;
                match &self.opened_dir_slots[parent_idx] {
                    Some(parent_dir) => {
                        hwa::debug!("Found parent at idx {}", parent_idx);
                        match self.mgr.open_dir(vol, parent_dir, name) {

                            Ok(directory) => {
                                hwa::debug!("Looking for a place...");
                                let mut idx = 0u8;
                                for refcount in &self.opened_dir_refcount {
                                    if *refcount == 0 {
                                        break;
                                    }
                                    idx += 1;
                                }
                                hwa::debug!("Will get idx {}... Len is {}", idx, MAX_DIRS);
                                if idx < (self.opened_dir_refcount.len() as u8 ){
                                    self.opened_dir_refcount[idx as usize] += 1;
                                    self.opened_dir_slots[idx as usize] = Some(directory);
                                    self.opened_dir_names[idx as usize] = None;
                                    Ok(DirectoryRef{idx})
                                }
                                else {
                                    self.mgr.close_dir(vol, directory);
                                    Err(SDCardError::MaxOpenDirs)
                                }
                            }
                            Err(reason) => {
                                match reason {
                                    embedded_sdmmc::Error::DirAlreadyOpen => {
                                        let mut idx = 0u8;
                                        for dirname in &self.opened_dir_names {
                                            if dirname.is_none() {
                                                break;
                                            }
                                            idx += 1;
                                        }
                                        if idx < (MAX_DIRS as u8 ){
                                            self.opened_dir_refcount[idx as usize] += 1;
                                            Ok(DirectoryRef{idx})
                                        }
                                        else {
                                            Err(SDCardError::InconsistencyError)
                                        }
                                    }
                                    _ => {
                                        Err(SDCardError::InternalError)
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        Err(SDCardError::InconsistencyError)
                    }
                }
            }
            None => {
                Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) fn close_dir(&mut self, dir_ref: DirectoryRef) {
        match self.vol.as_ref() {
            Some(vol) => {
                let idx = dir_ref.idx as usize;
                if self.opened_dir_refcount[idx] > 0 {
                    self.opened_dir_refcount[idx] -= 1;
                    if self.opened_dir_refcount[idx] == 0 {
                        hwa::debug!("Refcount of {} went to 0", idx);
                        if let Some(dir) = self.opened_dir_slots[idx].take() {
                            self.mgr.close_dir(vol, dir);
                            let _ = self.opened_dir_names[idx].take();
                        }
                    }
                }
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) fn is_dir(&mut self, parent_dir_ref: &DirectoryRef, entry_name: &str) -> Result<bool, SDCardError>{
        match self.vol.as_ref() {
            Some(vol) => {
                let idx = parent_dir_ref.idx as usize;
                if self.opened_dir_refcount[idx] > 0 {
                    match &self.opened_dir_slots[idx] {
                        Some(parent_dir) => {
                            match self.mgr.find_directory_entry(vol, parent_dir, entry_name) {
                                Ok(dir_entry) => {
                                    Ok(dir_entry.attributes.is_directory())
                                }
                                Err(_e) => {
                                    match _e {
                                        embedded_sdmmc::Error::NoSuchVolume => {
                                            Err(SDCardError::NoSuchVolume)
                                        }
                                        embedded_sdmmc::Error::FileNotFound => {
                                            Err(SDCardError::NotFound)
                                        }
                                        _ => {
                                            Err(SDCardError::InternalError)
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            todo!("hodor")
                        }
                    }
                }
                else {
                    Err(SDCardError::InconsistencyError)
                }
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }

    /***
    This is quite slow but safe as we are holding refcounts, so it's not possible to get inconsistencies
     */
    pub(crate) fn list_dir<F>(&mut self, dir: &DirectoryRef, func: F) -> Result<(), SDCardError>
    where F: FnMut(&DirEntry)
    {
        match self.vol.as_ref() {
            Some(vol) => {
                if self.opened_dir_refcount[dir.idx as usize] == 0 {
                    Err(SDCardError::InternalError)
                }
                else {
                    match &self.opened_dir_slots[dir.idx as usize] {
                        Some(dir) => {
                            self.mgr.iterate_dir(vol, dir, func).map_err( |e|
                                match e {
                                    _ => SDCardError::InternalError
                                }
                            )
                        }
                        None => {
                            Err(SDCardError::InternalError)
                        }
                    }
                }
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) async fn open_file(&mut self, parent_dir_ref: &DirectoryRef, file_name: &str) -> Result<File, SDCardError> {
        match self.vol.as_mut() {
            Some(vol) => {
                let idx = parent_dir_ref.idx as usize;
                if self.opened_dir_refcount[idx] > 0 {
                    match &self.opened_dir_slots[idx] {
                        Some(parent_dir) => {
                            match self.mgr.open_file_in_dir(vol, parent_dir, file_name, Mode::ReadOnly) {
                                Ok(file) => {
                                    Ok(file)
                                }
                                Err(_e) => {
                                    hwa::error!("Error opening file in directory. CLUE: File releasing is still incompleted :)");
                                    todo!("hodor")
                                }
                            }
                        }
                        None => {
                            hwa::error!("TODO Logic error. CLUE: File releasing is still incompleted :)");
                            todo!("hodor")
                        }
                    }
                }
                else {
                    Err(SDCardError::InconsistencyError)
                }
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) async fn close_file(&mut self, file: File) -> Result<(), SDCardError> {
        match self.vol.as_ref() {
            Some(vol) => {
                match self.mgr.close_file(vol, file) {
                    Ok(()) => Ok(()),
                    Err(_e) => {
                        panic!("hodor")
                    }
                }
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }

    pub(crate) async fn read(&mut self, file: &mut File, buffer: &mut [u8]) -> Result<usize, SDCardError> {
        match self.vol.as_ref() {
            Some(vol) => {
                Ok(self.mgr.read(vol, file, buffer).map_err(|e| match e {
                    _ => {
                        SDCardError::InternalError
                    }
                })?)
            }
            None => {
                todo!("No volume")
                //Err(SDCardError::NoSuchVolume)
            }
        }
    }
}

pub struct DummyTimeSource {

}
impl TimeSource for DummyTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub struct CardController {
    instance: &'static ControllerMutex<SDCard>,
}

#[allow(unused)]
impl CardController {
    pub(crate) async fn new(device: SDCardBlockDevice) -> Self {
        static CARD_CTRL_SHARED_STATE: TrackedStaticCell<ControllerMutex<SDCard>> = TrackedStaticCell::new();
        let mut card = SDCardVolumeManager::new_with_limits(device, DummyTimeSource{});
        #[cfg(feature = "sdcard-uses-spi")]
        card.device().retain().await;
        let vol = card.get_volume(VolumeIdx(hwa::SDCARD_PARTITION)).unwrap();
        #[cfg(feature = "sdcard-uses-spi")]
        card.device().release().await;
        let mut opened_dir_slots = heapless::Vec::new();
        let mut opened_dir_refcount = heapless::Vec::new();
        let mut opened_dir_names = heapless::Vec::new();
        for _ in 0 .. MAX_DIRS {
            opened_dir_slots.push(None).unwrap();
            opened_dir_refcount.push(0).unwrap();
            opened_dir_names.push(None).unwrap();
        }

        Self{
            instance: CARD_CTRL_SHARED_STATE.init(
                "card_shared_state",
                ControllerMutex::new(SDCard {
                    mgr: card,
                    vol: Some(vol),
                    opened_dir_slots,
                    opened_dir_refcount,
                    opened_dir_names,
                })
            )
        }
    }

    pub (crate) async fn list_dir(&self, full_path: &str) -> Result<CardAsyncDirIterator, SDCardError> {
        hwa::debug!("list_dir() called");
        let mut path: heapless::Vec<DirectoryRef, MAX_DIRS> = heapless::Vec::new();
        hwa::debug!("Locking card");
        let mut card = self.instance.lock().await;
        hwa::debug!("Locking card_dev");
        card.retain().await;
        hwa::debug!("opening root dir");
        let dir = card.open_root_dir()?; // TODO: Release on errors
        path.push(dir).map_err(|dr| {
            card.close_dir(dr);
            SDCardError::MaxOpenDirs
        })?;
        hwa::debug!("Opened root dir");
        for subdir in full_path.trim_start_matches('/').split('/') {
            if !subdir.is_empty() {
                if subdir == "." {
                    continue;
                }
                else if subdir == ".." {
                    if let Some(last_dir) = path.pop() {
                        card.close_dir(last_dir);
                        continue;
                    }
                    else {
                        return Err(SDCardError::InconsistencyError);
                    }

                }
                hwa::debug!("---- Opening {}", subdir);
                if let Some(last_dir) = path.last() {
                    path.push(card.open_dir(last_dir, subdir)?).map_err(|d| {
                        hwa::error!("Error opening subdir: push failed");
                        card.close_dir(d);
                        SDCardError::MaxOpenDirs
                    })?;
                }
            }
        }
        card.release().await;
        Ok(CardAsyncDirIterator::new(self.instance, path))
    }

    pub (crate) async fn new_stream(&self, file_path: &str) -> Result<SDCardStream, SDCardError> {

        let mut path: heapless::Vec<DirectoryRef, MAX_DIRS> = heapless::Vec::new();
        let mut card = self.instance.lock().await;
        card.retain().await;
        let dir = card.open_root_dir()?;
        path.push(dir).map_err(|dr| {
            card.close_dir(dr);
            SDCardError::MaxOpenDirs
        })?;
        hwa::debug!("Opened root dir");
        let mut file: Option<File> = None;
        for next_entry in file_path.trim_start_matches('/').split('/') {
            match file.take() { // If already got a file but willing to deep into tree.. consume the file and fail
                Some(file) => {
                    card.close_file(file).await.map_err(|_d| {
                        hwa::error!("Unexpected error closing file");
                        SDCardError::MaxOpenDirs
                    })?;
                    return Err(SDCardError::TrailingEntries);
                }
                None => {}
            }
            if !next_entry.is_empty() {
                if next_entry == "." {
                    continue;
                }
                else if next_entry == ".." {
                    if let Some(last_dir) = path.pop() {
                        card.close_dir(last_dir);
                        continue;
                    }
                    else {
                        return Err(SDCardError::InconsistencyError);
                    }

                }
                hwa::debug!("---- Opening {}", next_entry);
                if let Some(last_dir) = path.last() {

                    if card.is_dir(last_dir, next_entry)? {
                        path.push(card.open_dir(last_dir, next_entry)?).map_err(|d| {
                            hwa::error!("Error opening subdir: push failed");
                            card.close_dir(d);
                            SDCardError::MaxOpenDirs
                        })?;
                    }
                    else {
                        // File found -> Open it
                        file.replace(
                            card.open_file(last_dir, next_entry).await
                                .map_err(|_e| { SDCardError::InternalError })?
                        );
                    }
                }
            }
        }
        card.release().await;
        match file {
            Some(f) => {
                Ok(SDCardStream::new(self.clone(), path, f))
            }
            None => {
                Err(SDCardError::NotFound)
            }
        }
    }

    #[inline]
    pub(crate) async fn read(&mut self, file: &mut File, buffer: &mut [u8]) -> Result<usize, SDCardError> {
        let mut card = self.instance.lock().await;
        card.retain().await;
        let result = card.read(file, buffer).await;
        card.release().await;
        result
    }

    #[inline]
    pub(crate) async fn close_file(&mut self, file: File) -> Result<(), SDCardError> {
        let mut card = self.instance.lock().await;
        card.retain().await;
        let result = card.close_file(file).await;
        card.release().await;
        result
    }

    #[inline]
    pub(crate) async fn close_dir(&mut self, dir: DirectoryRef) -> () {
        let mut card = self.instance.lock().await;
        card.retain().await;
        card.close_dir(dir);
        card.release().await;
    }
}

impl Clone for CardController {
    fn clone(&self) -> Self {
        Self{ instance: self.instance }
    }
}

pub enum SDEntryType {
    FILE,
    DIRECTORY,
}

pub struct SDDirEntry {
    pub name: alloc::string::String,
    pub entry_type: SDEntryType,
    pub size: u32,
}

pub struct CardAsyncDirIterator {
    instance: &'static ControllerMutex<SDCard>,
    path: heapless::Vec<DirectoryRef, MAX_DIRS>,
    current_index: usize,
}

impl CardAsyncDirIterator {
    pub fn new(instance: &'static ControllerMutex<SDCard>, path: heapless::Vec<DirectoryRef, MAX_DIRS>) -> Self {

        Self {
            instance,
            path,
            current_index: 0,
        }
    }
    pub async fn next(&mut self) -> Result<Option<SDDirEntry>, SDCardError> {

        match self.path.last() {
            Some(d) => {
                let mut card = self.instance.lock().await;
                card.retain().await;
                let mut idx = 0;
                let mut entry = None;
                match card.list_dir(d, |de| {
                    if self.current_index == idx {
                        let name: alloc::string::String = match de.name.extension().is_empty() {
                            true => {
                                alloc::string::String::from_utf8_lossy(de.name.base_name()).to_string()
                            }
                            false => {
                                alloc::format!("{}.{}",
                                    alloc::string::String::from_utf8_lossy(de.name.base_name()).to_string().as_str(),
                                    alloc::string::String::from_utf8_lossy(de.name.extension()).to_string().as_str()
                                )
                            }
                        };
                        entry = Some(SDDirEntry {
                            name,
                            entry_type: match de.attributes.is_directory() {
                                true => SDEntryType::DIRECTORY,
                                false => SDEntryType::FILE,
                            },
                            size: de.size,
                        });
                    }
                    idx += 1;
                }) {
                    Ok(_) => {
                        card.release().await;
                        self.current_index += 1;
                        Ok(entry)
                    }
                    Err(_) => {
                        card.release().await;
                        drop(card);
                        self.cleanup().await;
                        Ok(None)
                    }
                }
            }
            None => {
                self.cleanup().await;
                Err(SDCardError::NoDirectorySpecified)
            }
        }
    }

    pub async fn close(&mut self) {
        let mut instance = self.instance.lock().await;
        while let Some(d) = self.path.pop() {
            instance.close_dir(d);
        }
    }

    async fn cleanup(&mut self) {
        let mut card = self.instance.lock().await;
        while let Some(d) = self.path.pop() {
            card.close_dir(d);
        }
    }
}

const BSIZE: usize = 32;

#[allow(unused)]
pub struct SDCardStream
{
    card_controller: CardController,
    file: Option<File>,
    path: heapless::Vec<DirectoryRef, MAX_DIRS>,
    buffer: [u8; BSIZE],
    bytes_read: u8,
    current_byte_index: u8,
}

impl SDCardStream
{
    pub(self) fn new(card_controller: CardController, path: heapless::Vec<DirectoryRef, MAX_DIRS>, file: File) -> Self {
        Self {
            card_controller,
            file: Some(file),
            path,
            buffer: [0; BSIZE],
            bytes_read: 0,
            current_byte_index: 0,
        }
    }
}

impl Stream for SDCardStream {
    type Item = Result<u8, async_gcode::Error>;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.current_byte_index < this.bytes_read {
            let byte = this.buffer[this.current_byte_index as usize];
            this.current_byte_index += 1;
            Poll::Ready(Some(Ok(byte)))
        }
        else {
            this.current_byte_index = 0;
            this.bytes_read = 0;
            let result = match this.file.as_mut() {
                None => {
                    Poll::Ready(Err(SDCardError::NotFound))
                }
                Some(f) => {
                    core::pin::pin!(
                        this.card_controller.read(f, &mut this.buffer)
                    ).poll(ctx)
                }
            };
            match result {
                Poll::Ready(rst) => {
                    match rst {
                        Ok(bytes_read) => {
                            this.bytes_read = bytes_read as u8;
                            if bytes_read > 0 {
                                let byte = this.buffer[this.current_byte_index as usize];
                                this.current_byte_index = 1;
                                Poll::Ready(Some(Ok(byte)))
                            }
                            else {
                                this.bytes_read = 0;
                                this.current_byte_index = 0;
                                Poll::Ready(None)
                            }
                        }
                        Err(_) => {
                            // FIXME: Propper error type and logic
                            Poll::Ready(Some(Err(async_gcode::Error::NumberOverflow)))
                        }
                    }
                }
                Poll::Pending => {
                    Poll::Pending
                }
            }
        }
    }
}

/*
impl async_gcode::AsyncRead for SDCardStream
{
    #[inline]
    async fn read_byte(&mut self) -> Option<Result<u8, async_gcode::Error>> {

    }

    #[inline]
    fn push_back(&mut self, _b: u8)  {
        //crate::debug!("async stream push back");
        if self.current_byte_index > 0 {
            self.current_byte_index -= 1;
        }
    }

    #[inline]
    async fn close(&mut self) {
        match self.file.take() {
            Some(file) => {
                let _ = self.card_controller.close_file(file).await;
            }
            None => {}
        }
        while let Some(d) = self.path.pop() {
            self.card_controller.close_dir(d).await;
        }
    }
}

*/