use std::{path::Path, sync::Arc};

use bridge::{
    handle::BackendHandle,
    install::ContentInstall,
    instance::{InstanceContentID, InstanceID},
    message::{MessageToBackend, QuickPlayLaunch},
    modal_action::ModalAction,
};
use gpui::{prelude::*, *};
use gpui_component::{scroll::ScrollableElement, v_flex, Root, Theme, WindowExt};

use crate::{
    boot_sequence::BootSequence,
    entity::DataEntities,
    modals,
    ui::{LauncherUI, PageType},
    CloseWindow, OpenSettings, MAIN_FONT,
};

pub struct LauncherRootGlobal {
    pub root: Entity<LauncherRoot>,
}

impl Global for LauncherRootGlobal {}

pub struct LauncherRoot {
    pub ui: Entity<LauncherUI>,
    data: DataEntities,
    focus_handle: FocusHandle,
    boot_sequence: Entity<BootSequence>,
}

impl LauncherRoot {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let launcher_ui = cx.new(|cx| LauncherUI::new(data, window, cx));

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            ui: launcher_ui,
            data: data.clone(),
            focus_handle,
            boot_sequence: cx.new(|_| BootSequence::new()),
        }
    }
}

impl Render for LauncherRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(message) = &*self.data.panic_messages.deadlock_message.read() {
            let purple = Hsla {
                h: 0.8333333333,
                s: 1.,
                l: 0.25,
                a: 1.,
            };
            return v_flex()
                .size_full()
                .text_color(gpui::white())
                .bg(purple)
                .child(message.clone())
                .overflow_y_scrollbar()
                .into_any_element();
        }
        if let Some(message) = &*self.data.panic_messages.panic_message.read() {
            return v_flex()
                .size_full()
                .text_color(gpui::white())
                .bg(gpui::blue())
                .child(message.clone())
                .overflow_y_scrollbar()
                .into_any_element();
        }
        if self.data.backend_handle.is_closed() {
            return v_flex()
                .size_full()
                .text_color(gpui::white())
                .bg(gpui::red())
                .child(t::system::backend_shutdown())
                .into_any_element();
        }

        Theme::global_mut(cx).sheet.margin_top = Pixels::ZERO;

        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        let boot_overlay = self
            .boot_sequence
            .read(cx)
            .is_active()
            .then(|| self.boot_sequence.clone().into_any_element());

        v_flex()
            .size_full()
            .font_family(MAIN_FONT)
            .relative()
            .child(self.ui.clone())
            .when_some(boot_overlay, |this, overlay| this.child(overlay))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
            .track_focus(&self.focus_handle)
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action({
                let data = self.data.clone();
                move |_: &OpenSettings, window, cx| {
                    let build = crate::modals::settings::build_settings_sheet(&data, window, cx);
                    window.open_sheet_at(gpui_component::Placement::Left, cx, build);
                }
            })
            .on_mouse_down(MouseButton::Navigate(NavigationDirection::Back), {
                let ui = self.ui.clone();
                move |_, window, cx| {
                    ui.update(cx, |ui, cx| {
                        ui.nav_backwards(window, cx);
                    });
                }
            })
            .on_mouse_down(MouseButton::Navigate(NavigationDirection::Forward), {
                let ui = self.ui.clone();
                move |_, window, cx| {
                    ui.update(cx, |ui, cx| {
                        ui.nav_forwards(window, cx);
                    });
                }
            })
            .into_any_element()
    }
}

pub fn start_instance(
    id: InstanceID,
    name: SharedString,
    quick_play: Option<QuickPlayLaunch>,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let has_selected_account = cx
        .global::<LauncherRootGlobal>()
        .root
        .read(cx)
        .data
        .accounts
        .read(cx)
        .selected_account_uuid
        .is_some();

    if !has_selected_account {
        let data = cx.global::<LauncherRootGlobal>().root.read(cx).data.clone();
        crate::ui::open_account_manager(&data, window, cx);
        return;
    }

    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::StartInstance {
        id,
        quick_play,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = t::instance::start::title(&name).into();
    modals::generic::show_modal(window, cx, title, t::instance::start::error().into(), modal_action);
}

pub fn start_new_account_login(backend_handle: &BackendHandle, window: &mut Window, cx: &mut App) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::AddNewAccount {
        modal_action: modal_action.clone(),
    });

    let title = t::account::add::title();
    modals::generic::show_modal(window, cx, title.into(), t::account::add::error().into(), modal_action);
}

pub fn start_install(
    content_install: ContentInstall,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::InstallContent {
        content: content_install.clone(),
        modal_action: modal_action.clone(),
    });

    modals::generic::show_notification(window, cx, t::instance::content::install::error().into(), modal_action);
}

pub fn start_update_check(instance: InstanceID, backend_handle: &BackendHandle, window: &mut Window, cx: &mut App) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::UpdateCheck {
        instance,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = t::instance::content::update::check::title().into();
    modals::generic::show_modal(window, cx, title, t::instance::content::update::check::error().into(), modal_action);
}

pub fn update_single_mod(
    instance: InstanceID,
    mod_id: InstanceContentID,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::UpdateContent {
        instance,
        content_id: mod_id,
        modal_action: modal_action.clone(),
    });

    modals::generic::show_notification(
        window,
        cx,
        t::instance::content::update::download::error().into(),
        modal_action,
    );
}

pub fn upload_log_file(path: Arc<Path>, backend_handle: &BackendHandle, window: &mut Window, cx: &mut App) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::UploadLogFile {
        path,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = t::instance::logs::upload::title().into();
    modals::generic::show_modal(window, cx, title, t::instance::logs::upload::error().into(), modal_action);
}

pub fn switch_page(page: PageType, breadcrumbs: &[PageType], window: &mut Window, cx: &mut App) {
    cx.update_global::<LauncherRootGlobal, ()>(|global, cx| {
        global.root.update(cx, |launcher_root, cx| {
            launcher_root.ui.update(cx, |ui, cx| {
                ui.switch_page(page, breadcrumbs, window, cx);
            });
        });
    });
}
