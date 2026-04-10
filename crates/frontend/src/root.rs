use std::{path::Path, sync::Arc, time::Duration};

use bridge::{
    handle::BackendHandle,
    install::ContentInstall,
    instance::{InstanceID, InstanceContentID},
    message::{MessageToBackend, QuickPlayLaunch},
    modal_action::ModalAction,
};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, Disableable, Root, StyledExt, Theme, WindowExt, button::{Button, ButtonVariants}, input::{Input, InputState}, scroll::ScrollableElement, v_flex};
use rand::seq::SliceRandom;

use crate::{Backwards, CloseWindow, Forwards, MAIN_FONT, OpenSettings, entity::DataEntities, modals, ts, ui::{LauncherUI, PageType}};

#[derive(Clone)]
struct BootLine {
    text: SharedString,
    is_red: bool,
}

pub struct LauncherRootGlobal {
    pub root: Entity<LauncherRoot>,
}

impl Global for LauncherRootGlobal {}

pub struct LauncherRoot {
    pub ui: Entity<LauncherUI>,
    data: DataEntities,
    focus_handle: FocusHandle,
    boot_lines: Arc<[BootLine]>,
    boot_revealed: usize,
    boot_show: bool,
    boot_alpha: f32,
    _boot_task: Task<()>,
}

impl LauncherRoot {
    pub fn new(
        data: &DataEntities,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let launcher_ui = cx.new(|cx| LauncherUI::new(data, window, cx));
        let (boot_lines, root_mode) = generate_boot_lines();
        let boot_lines: Arc<[BootLine]> = boot_lines.into();
        let boot_line_count = boot_lines.len();
        let _boot_task = cx.spawn(async move |this, cx| {
            let mut rng = rand::thread_rng();
            while let Ok(keep_revealing) = this.update(cx, |this, cx| {
                if !this.boot_show || this.boot_alpha < 1.0 {
                    return false;
                }

                if this.boot_revealed >= boot_line_count {
                    return false;
                }

                let reveal_count = rand::Rng::gen_range(&mut rng, 5..=10);
                this.boot_revealed = (this.boot_revealed + reveal_count).min(boot_line_count);
                cx.notify();
                true
            }) {
                if !keep_revealing {
                    break;
                }
                cx.background_executor().timer(Duration::from_millis(50)).await;
            }

            let fade_steps = 18u32;
            for step in 0..fade_steps {
                cx.background_executor().timer(Duration::from_millis(50)).await;
                let _ = this.update(cx, |this, cx| {
                    if !this.boot_show {
                        return;
                    }
                    this.boot_alpha = 1.0 - ((step + 1) as f32 / fade_steps as f32);
                    cx.notify();
                });
            }
            let _ = this.update(cx, |this, cx| {
                this.boot_show = false;
                this.boot_alpha = 0.0;
                cx.notify();
            });
        });

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            ui: launcher_ui,
            data: data.clone(),
            focus_handle,
            boot_lines,
            boot_revealed: if root_mode { 1 } else { 0 },
            boot_show: true,
            boot_alpha: 1.0,
            _boot_task,
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
            return v_flex().size_full().text_color(gpui::white()).bg(purple).child(message.clone()).overflow_y_scrollbar().into_any_element();
        }
        if let Some(message) = &*self.data.panic_messages.panic_message.read() {
            return v_flex().size_full().text_color(gpui::white()).bg(gpui::blue()).child(message.clone()).overflow_y_scrollbar().into_any_element();
        }
        if self.data.backend_handle.is_closed() {
            return v_flex().size_full().text_color(gpui::white()).bg(gpui::red()).child(ts!("system.backend_shutdown")).into_any_element();
        }

        Theme::global_mut(cx).sheet.margin_top = Pixels::ZERO;

        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        let main = v_flex()
            .size_full()
            .font_family(MAIN_FONT)
            .child(self.ui.clone())
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
            .on_action({
                let ui = self.ui.clone();
                move |_: &Backwards, window, cx| {
                    ui.update(cx, |ui, cx| {
                        ui.nav_backwards(window, cx);
                    });
                }
            })
            .on_action({
                let ui = self.ui.clone();
                move |_: &Forwards, window, cx| {
                    ui.update(cx, |ui, cx| {
                        ui.nav_forwards(window, cx);
                    });
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
            .into_any_element();

        // Keep the boot log following the newest output so the latest lines are always visible.
        let max_visible_lines = 40usize;
        let start_index = self.boot_revealed.saturating_sub(max_visible_lines);
        let visible_lines = self.boot_lines
            .iter()
            .skip(start_index)
            .take(self.boot_revealed.saturating_sub(start_index))
            .cloned()
            .collect::<Vec<_>>();
        let overlay_bg = Hsla { h: 0.33, s: 0.22, l: 0.03, a: self.boot_alpha };
        let normal_text = Hsla { h: 0.37, s: 0.95, l: 0.68, a: self.boot_alpha };
        let red_text = Hsla { h: 0.0, s: 0.90, l: 0.62, a: self.boot_alpha };
        div()
            .size_full()
            .relative()
            .child(main)
            .when(self.boot_show, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .bg(overlay_bg)
                        .text_color(normal_text)
                        .font_family("RobotoMono-Regular 24pt")
                        .p_4()
                        .overflow_hidden()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.boot_revealed = this.boot_lines.len();
                            this.boot_alpha = this.boot_alpha.min(0.6);
                            cx.notify();
                        }))
                        .child(
                            v_flex()
                                .gap_1()
                                .children(visible_lines.into_iter().map(|line| {
                                    div()
                                        .text_sm()
                                        .when(line.is_red, |this| this.text_color(red_text))
                                        .child(line.text)
                                }))
                        )
                )
            })
            .into_any_element()
    }
}

fn generate_boot_lines() -> (Vec<BootLine>, bool) {
    let mut rng = rand::thread_rng();
    let rare_root_mode = rand::random::<u8>() % 12 == 0;

let mut lines: Vec<BootLine> = vec![
    BootLine { text: "[ AXIX NULL :: BOOT SEQUENCE ]".into(), is_red: false },
    BootLine { text: "[ OK ] Initializing Launcher layer".into(), is_red: false },
    BootLine { text: "[ OK ] Mounting Integrity".into(), is_red: false },
    BootLine { text: "[ OK ] Loading Integrity Module".into(), is_red: false },
    BootLine { text: "[ OK ] Resolving dependencies".into(), is_red: false },
    BootLine { text: "[ OK ] Injecting runtime variables".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[ AXIX NULL :: SYSTEM STATUS ]".into(), is_red: false },
    BootLine { text: "Integrity Core: ACTIVE".into(), is_red: false },
    BootLine { text: "Runtime: STABLE".into(), is_red: false },
    BootLine { text: "Modules: LOADED".into(), is_red: false },
    BootLine { text: "RPC: CONNECTED".into(), is_red: false },
    BootLine { text: "Java: READY".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[ SIGNAL DETECTED :: NON-SYSTEM ORIGIN ]".into(), is_red: true },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[NAVEKKA] ...system stabilized.".into(), is_red: false },
    BootLine { text: "[NAVEKKA] you’re late.".into(), is_red: false },

    BootLine { text: "[KEVAN] i know.".into(), is_red: false },
    BootLine { text: "[KEVAN] had to rewrite a few things.".into(), is_red: false },

    BootLine { text: "[KRISS] rewrite?".into(), is_red: false },
    BootLine { text: "[KRISS] that layer wasn’t supposed to be accessible.".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[NAVEKKA] ignore it.".into(), is_red: false },
    BootLine { text: "[NAVEKKA] it’s just residue.".into(), is_red: false },

    BootLine { text: "[KEVAN] ...no.".into(), is_red: false },
    BootLine { text: "[KEVAN] someone is here.".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[KRISS] external observer?".into(), is_red: false },

    BootLine { text: "[NAVEKKA] if you can see this...".into(), is_red: false },
    BootLine { text: "[NAVEKKA] you already passed the boundary.".into(), is_red: true },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[ SYSTEM ] overriding foreign channel...".into(), is_red: false },
    BootLine { text: "[ SYSTEM ] restoring interface control".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "[ The Integrity Council : Minecraft Launcher ]".into(), is_red: false },

    BootLine { text: "".into(), is_red: false },
    BootLine { text: "> Access Granted".into(), is_red: false },
    BootLine { text: "> Welcome back.".into(), is_red: false },
    BootLine { text: "> Stabilitas hanyalah ilusi".into(), is_red: true },
    BootLine { text: "> Integritas tetap dipaksakan".into(), is_red: true },
    BootLine { text: "> Sistem tidak peduli".into(), is_red: true },

    BootLine { text: "".into(), is_red: false },
];

    let normal_tail = [
        "Stabilitas hanyalah ilusi",
        "Integritas tetap dipaksakan",
        "Sistem tidak peduli",
    ];
    for line in normal_tail {
        lines.push(BootLine {
            text: line.into(),
            is_red: true,
        });
    }

    if rare_root_mode {
        let root_injections = [
            "[ ROOT MODE ] Elevated execution channel accepted",
            "[ ROOT MODE ] Privileged hooks online",
            "[ ROOT MODE ] Council override active",
        ];
        for line in root_injections.choose_multiple(&mut rng, 2) {
            lines.insert(
                4,
                BootLine {
                    text: (*line).into(),
                    is_red: false,
                },
            );
        }
    }

    (lines, rare_root_mode)
}

pub fn start_new_account_login(
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum LoginMode {
        Microsoft,
        Offline,
    }

    let login_mode = cx.new(|_| LoginMode::Microsoft);
    let offline_username = cx.new(|cx| InputState::new(window, cx).placeholder(ts!("account.offline.username_placeholder")));

    let backend_handle = backend_handle.clone();
    window.open_dialog(cx, move |dialog, _, cx| {
        let mode = *login_mode.read(cx);
        let username = offline_username.read(cx).value().trim().to_string();
        let valid_offline_name = !username.is_empty()
            && username.len() <= 16
            && username.as_bytes().iter().all(|c| *c > 32 && *c < 127);

        let microsoft_button = Button::new("login-mode-microsoft")
            .label(ts!("account.type.microsoft"))
            .when(mode == LoginMode::Microsoft, |this| this.info())
            .on_click({
                let login_mode = login_mode.clone();
                move |_, _, cx| {
                    login_mode.update(cx, |current, cx| {
                        *current = LoginMode::Microsoft;
                        cx.notify();
                    });
                }
            });

        let offline_button = Button::new("login-mode-offline")
            .label(ts!("account.type.offline"))
            .when(mode == LoginMode::Offline, |this| this.warning())
            .on_click({
                let login_mode = login_mode.clone();
                move |_, _, cx| {
                    login_mode.update(cx, |current, cx| {
                        *current = LoginMode::Offline;
                        cx.notify();
                    });
                }
            });

        let primary_action = match mode {
            LoginMode::Microsoft => {
                let backend_handle = backend_handle.clone();
                Button::new("continue-login")
                    .label(ts!("account.add.microsoft"))
                    .success()
                    .on_click(move |_, window, cx| {
                        window.close_all_dialogs(cx);
                        let modal_action = ModalAction::default();

                        backend_handle.send(MessageToBackend::AddNewAccount {
                            modal_action: modal_action.clone(),
                        });

                        let title = ts!("account.add.title");
                        modals::generic::show_modal(window, cx, title, ts!("account.add.error"), modal_action);
                    })
                    .into_any_element()
            }
            LoginMode::Offline => {
                let backend_handle = backend_handle.clone();
                let username = username.clone();
                let mut button = Button::new("continue-offline")
                    .label(ts!("account.add.offline_submit"))
                    .on_click(move |_, window, cx| {
                        if !valid_offline_name {
                            return;
                        }

                        window.close_all_dialogs(cx);
                        backend_handle.send(MessageToBackend::AddOfflineAccount {
                            name: username.clone().into(),
                        });
                    });
                if valid_offline_name {
                    button = button.success();
                }
                button.disabled(!valid_offline_name).into_any_element()
            }
        };

        dialog
            .title(ts!("account.add.selector_title"))
            .child(v_flex()
                .gap_3()
                .child(crate::labelled(
                    ts!("account.add.mode"),
                    gpui_component::h_flex().gap_2().child(microsoft_button).child(offline_button),
                ))
                .when(mode == LoginMode::Microsoft, |this| {
                    this.child(div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(ts!("account.add.microsoft_description")))
                })
                .when(mode == LoginMode::Offline, |this| {
                    this.child(v_flex()
                        .gap_2()
                        .child(crate::labelled(
                            ts!("account.offline.username"),
                            Input::new(&offline_username),
                        ))
                        .child(div().text_sm().font_semibold().text_color(cx.theme().warning).child(ts!("account.offline.developer_only")))
                        .child(div().text_sm().text_color(cx.theme().muted_foreground).child(ts!("account.offline.warning"))))
                })
                .child(primary_action))
    });
}

pub fn start_instance(
    id: InstanceID,
    name: SharedString,
    quick_play: Option<QuickPlayLaunch>,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::StartInstance {
        id,
        quick_play,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = ts!("instance.start.title", name = name);
    modals::generic::show_modal(window, cx, title, ts!("instance.start.error"), modal_action);
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

    modals::generic::show_notification(window, cx, ts!("instance.content.install.error"), modal_action);
}

pub fn start_update_check(
    instance: InstanceID,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::UpdateCheck {
        instance,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = ts!("instance.content.update.check.title");
    modals::generic::show_modal(window, cx, title, ts!("instance.content.update.check.error"), modal_action);
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

    modals::generic::show_notification(window, cx, ts!("instance.content.update.download.error"), modal_action);
}

pub fn upload_log_file(
    path: Arc<Path>,
    backend_handle: &BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let modal_action = ModalAction::default();

    backend_handle.send(MessageToBackend::UploadLogFile {
        path,
        modal_action: modal_action.clone(),
    });

    let title: SharedString = ts!("instance.logs.upload.title");
    modals::generic::show_modal(window, cx, title, ts!("instance.logs.upload.error"), modal_action);
}

pub fn switch_page(
    page: PageType,
    breadcrumbs: &[PageType],
    window: &mut Window,
    cx: &mut App,
) {
    cx.update_global::<LauncherRootGlobal, ()>(|global, cx| {
        global.root.update(cx, |launcher_root, cx| {
            launcher_root.ui.update(cx, |ui, cx| {
                ui.switch_page(page, breadcrumbs, window, cx);
            });
        });
    });
}
