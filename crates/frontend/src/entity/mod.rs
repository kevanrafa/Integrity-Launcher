use std::{path::Path, sync::Arc};

use bridge::{handle::BackendHandle, message::{IntegrityModpack, MessageToBackend, SkinLibrary}, serial::AtomicOptionSerial};
use gpui::{App, Entity, Global};
use parking_lot::RwLock;

use crate::entity::{
    account::AccountEntries, instance::InstanceEntries, metadata::FrontendMetadata
};

pub mod account;
pub mod instance;
pub mod metadata;

#[derive(Clone)]
pub struct DataEntities {
    pub instances: Entity<InstanceEntries>,
    pub metadata: Entity<FrontendMetadata>,
    pub accounts: Entity<AccountEntries>,
    pub backend_handle: BackendHandle,
    pub theme_folder: Arc<Path>,
    pub panic_messages: Arc<PanicMessages>,
}

pub struct PanicMessages {
    pub panic_message: Arc<RwLock<Option<String>>>,
    pub deadlock_message: Arc<RwLock<Option<String>>>,
}

struct SkinLibraryWrapper {
    skin_library: Option<SkinLibrary>,
    serial: AtomicOptionSerial,
}

impl Global for SkinLibraryWrapper {}

struct IntegrityModpacksWrapper {
    modpacks: Option<Arc<[IntegrityModpack]>>,
    serial: AtomicOptionSerial,
}

impl Global for IntegrityModpacksWrapper {}

impl DataEntities {
    pub fn use_skin_library<'a>(&self, cx: &'a mut App) -> Option<&'a SkinLibrary> {
        let wrapper = cx.global::<SkinLibraryWrapper>();

        let load = if let Some(skin_library) = &wrapper.skin_library {
            skin_library.state.set_observed();
            skin_library.state.should_load()
        } else {
            true
        };
        if load {
            self.backend_handle.send_with_serial(MessageToBackend::RequestSkinLibrary, &wrapper.serial);
        }

        wrapper.skin_library.as_ref()
    }

    pub fn set_skin_library(&self, skin_library: SkinLibrary, cx: &mut App) {
        let wrapper = cx.global_mut::<SkinLibraryWrapper>();
        wrapper.skin_library = Some(skin_library);
        cx.refresh_windows();
    }

    pub fn init_globals(cx: &mut App) {
        cx.set_global(SkinLibraryWrapper {
            skin_library: None,
            serial: AtomicOptionSerial::default(),
        });
        cx.set_global(IntegrityModpacksWrapper {
            modpacks: None,
            serial: AtomicOptionSerial::default(),
        });
    }

    pub fn use_integrity_modpacks<'a>(&self, cx: &'a mut App) -> Option<&'a Arc<[IntegrityModpack]>> {
        let wrapper = cx.global::<IntegrityModpacksWrapper>();
        if wrapper.modpacks.is_none() {
            self.backend_handle.send_with_serial(MessageToBackend::RequestIntegrityModpacks, &wrapper.serial);
        }
        wrapper.modpacks.as_ref()
    }

    pub fn set_integrity_modpacks(&self, modpacks: Arc<[IntegrityModpack]>, cx: &mut App) {
        let wrapper = cx.global_mut::<IntegrityModpacksWrapper>();
        wrapper.modpacks = Some(modpacks);
        cx.refresh_windows();
    }
}
