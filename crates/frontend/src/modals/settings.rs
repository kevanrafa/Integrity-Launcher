use std::{path::Path, sync::Arc};

use bridge::{handle::BackendHandle, message::{BackendConfigWithPassword, MessageToBackend}};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState, NumberInput},
    select::{SearchableVec, Select, SelectEvent, SelectState},
    sheet::Sheet,
    spinner::Spinner,
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Disableable, Sizable, ThemeRegistry,
};
use schema::backend_config::{BackendConfig, DiscordRpcConfig, JavaRuntimeMode, ProxyConfig, ProxyProtocol};

use crate::{entity::DataEntities, icon::PandoraIcon, interface_config::InterfaceConfig, ts};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SettingsTab {
    #[default]
    Interface,
    Network,
}

struct Settings {
    selected_tab: SettingsTab,
    theme_folder: Arc<Path>,
    theme_select: Entity<SelectState<SearchableVec<SharedString>>>,
    backend_handle: BackendHandle,
    pending_request: bool,
    backend_config: Option<BackendConfig>,
    get_configuration_task: Option<Task<()>>,
    // Proxy settings state
    proxy_enabled: bool,
    proxy_protocol_select: Entity<SelectState<Vec<&'static str>>>,
    proxy_host_input: Entity<InputState>,
    proxy_port_input: Entity<InputState>,
    proxy_auth_enabled: bool,
    proxy_username_input: Entity<InputState>,
    proxy_password_input: Entity<InputState>,
    proxy_password_changed: bool,
    rpc_enabled: bool,
    rpc_show_advanced_details: bool,
    rpc_client_id_input: Entity<InputState>,
    rpc_idle_text_input: Entity<InputState>,
    rpc_selecting_text_input: Entity<InputState>,
    rpc_playing_text_input: Entity<InputState>,
    java_mode_select: Entity<SelectState<Vec<&'static str>>>,
    java_preferred_version_select: Entity<SelectState<Vec<&'static str>>>,
}

pub fn build_settings_sheet(data: &DataEntities, window: &mut Window, cx: &mut App) -> impl Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static {
    let theme_folder = data.theme_folder.clone();
    let settings = cx.new(|cx| {
        let theme_select_delegate = SearchableVec::new(ThemeRegistry::global(cx).sorted_themes()
            .iter().map(|cfg| cfg.name.clone()).collect::<Vec<_>>());

        let theme_select = cx.new(|cx| {
            let mut state = SelectState::new(theme_select_delegate, Default::default(), window, cx).searchable(true);
            state.set_selected_value(&cx.theme().theme_name().clone(), window, cx);
            state
        });

        cx.subscribe_in(&theme_select, window, |_, entity, _: &SelectEvent<_>, _, cx| {
            let Some(theme_name) = entity.read(cx).selected_value().cloned() else {
                return;
            };

            InterfaceConfig::get_mut(cx).active_theme = theme_name.clone();

            let Some(theme) = gpui_component::ThemeRegistry::global(cx).themes().get(&SharedString::new(theme_name.trim_ascii())).cloned() else {
                return;
            };

            gpui_component::Theme::global_mut(cx).apply_config(&theme);
        }).detach();

        let proxy_protocol_select = cx.new(|cx| {
            let protocols = vec!["HTTP", "HTTPS", "SOCKS5"];
            let mut state = SelectState::new(protocols, None, window, cx);
            state.set_selected_value(&"HTTP", window, cx);
            state
        });

        let proxy_host_input = cx.new(|cx| InputState::new(window, cx).placeholder("proxy.example.com"));
        let proxy_port_input = cx.new(|cx| InputState::new(window, cx).default_value("8080".to_string()));
        let proxy_username_input = cx.new(|cx| InputState::new(window, cx).placeholder("username"));
        let proxy_password_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("password");
            state.set_masked(true, window, cx);
            state
        });

        let rpc_client_id_input = cx.new(|cx| InputState::new(window, cx).placeholder("Discord app client id"));
        let rpc_idle_text_input = cx.new(|cx| InputState::new(window, cx).placeholder("Idle in Launcher"));
        let rpc_selecting_text_input = cx.new(|cx| InputState::new(window, cx).placeholder("Selecting Instance"));
        let rpc_playing_text_input = cx.new(|cx| InputState::new(window, cx).placeholder("Playing Minecraft"));
        let java_mode_select = cx.new(|cx| {
            let modes = vec!["Auto", "System", "Bundled"];
            let mut state = SelectState::new(modes, None, window, cx);
            state.set_selected_value(&"Auto", window, cx);
            state
        });
        let java_preferred_version_select = cx.new(|cx| {
            let versions = vec!["Game Default", "Java 25", "Java 21", "Java 17", "Java 8"];
            let mut state = SelectState::new(versions, None, window, cx);
            state.set_selected_value(&"Game Default", window, cx);
            state
        });

        let mut settings = Settings {
            selected_tab: SettingsTab::Interface,
            theme_folder,
            theme_select,
            backend_handle: data.backend_handle.clone(),
            pending_request: false,
            backend_config: None,
            get_configuration_task: None,
            proxy_enabled: false,
            proxy_protocol_select,
            proxy_host_input,
            proxy_port_input,
            proxy_auth_enabled: false,
            proxy_username_input,
            proxy_password_input,
            proxy_password_changed: false,
            rpc_enabled: false,
            rpc_show_advanced_details: true,
            rpc_client_id_input,
            rpc_idle_text_input,
            rpc_selecting_text_input,
            rpc_playing_text_input,
            java_mode_select,
            java_preferred_version_select,
        };

        cx.subscribe(&settings.proxy_protocol_select, Settings::on_proxy_protocol_changed).detach();
        cx.subscribe(&settings.proxy_host_input, Settings::on_proxy_input_changed).detach();
        cx.subscribe(&settings.proxy_port_input, Settings::on_proxy_input_changed).detach();
        cx.subscribe(&settings.proxy_username_input, Settings::on_proxy_input_changed).detach();
        cx.subscribe(&settings.proxy_password_input, Settings::on_proxy_password_changed).detach();
        cx.subscribe(&settings.rpc_client_id_input, Settings::on_rpc_input_changed).detach();
        cx.subscribe(&settings.rpc_idle_text_input, Settings::on_rpc_input_changed).detach();
        cx.subscribe(&settings.rpc_selecting_text_input, Settings::on_rpc_input_changed).detach();
        cx.subscribe(&settings.rpc_playing_text_input, Settings::on_rpc_input_changed).detach();
        cx.subscribe(&settings.java_mode_select, Settings::on_java_mode_changed).detach();
        cx.subscribe(&settings.java_preferred_version_select, Settings::on_java_preferred_version_changed).detach();

        settings.update_backend_configuration(window, cx);

        settings
    });

    move |sheet, _, cx| {
        sheet
            .title(ts!("settings.title"))
            .size(px(420.))
            .p_0()
            .when(cfg!(target_os = "macos"), |this| this.pt_5())
            .child(v_flex()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(settings.clone())
            )
    }
}

impl Settings {
    pub fn update_backend_configuration(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.get_configuration_task.is_some() {
            self.pending_request = true;
            return;
        }

        let (send, recv) = tokio::sync::oneshot::channel();
        self.get_configuration_task = Some(cx.spawn_in(window, async move |page, cx| {
            let result: BackendConfigWithPassword = recv.await.unwrap_or_default();
            let _ = page.update_in(cx, move |settings, window, cx| {
                settings.proxy_enabled = result.config.proxy.enabled;
                settings.proxy_auth_enabled = result.config.proxy.auth_enabled;
                settings.rpc_enabled = result.config.discord_rpc.enabled;
                settings.rpc_show_advanced_details = result.config.discord_rpc.show_advanced_details;

                settings.proxy_host_input.update(cx, |input, cx| {
                    input.set_value(&result.config.proxy.host, window, cx);
                });
                settings.proxy_port_input.update(cx, |input, cx| {
                    input.set_value(result.config.proxy.port.to_string(), window, cx);
                });
                settings.proxy_username_input.update(cx, |input, cx| {
                    input.set_value(&result.config.proxy.username, window, cx);
                });
                settings.proxy_protocol_select.update(cx, |select, cx| {
                    select.set_selected_value(&result.config.proxy.protocol.name(), window, cx);
                });
                if let Some(ref password) = result.proxy_password {
                    settings.proxy_password_input.update(cx, |input, cx| {
                        input.set_value(password, window, cx);
                    });
                }
                settings.rpc_client_id_input.update(cx, |input, cx| {
                    input.set_value(&result.config.discord_rpc.client_id, window, cx);
                });
                settings.rpc_idle_text_input.update(cx, |input, cx| {
                    input.set_value(&result.config.discord_rpc.idle_text, window, cx);
                });
                settings.rpc_selecting_text_input.update(cx, |input, cx| {
                    input.set_value(&result.config.discord_rpc.selecting_text, window, cx);
                });
                settings.rpc_playing_text_input.update(cx, |input, cx| {
                    input.set_value(&result.config.discord_rpc.playing_text, window, cx);
                });
                settings.java_mode_select.update(cx, |select, cx| {
                    let selected = match result.config.java_runtime.mode {
                        JavaRuntimeMode::Auto => "Auto",
                        JavaRuntimeMode::System => "System",
                        JavaRuntimeMode::Bundled => "Bundled",
                    };
                    select.set_selected_value(&selected, window, cx);
                });
                settings.java_preferred_version_select.update(cx, |select, cx| {
                    let selected = match result.config.java_runtime.preferred_major_version {
                        Some(25) => "Java 25",
                        Some(21) => "Java 21",
                        Some(17) => "Java 17",
                        Some(8) => "Java 8",
                        _ => "Game Default",
                    };
                    select.set_selected_value(&selected, window, cx);
                });

                settings.backend_config = Some(result.config);
                settings.get_configuration_task = None;
                cx.notify();

                if settings.pending_request {
                    settings.pending_request = false;
                    settings.update_backend_configuration(window, cx);
                }
            });
        }));

        self.backend_handle.send(MessageToBackend::GetBackendConfiguration {
            channel: send,
        });
    }

    fn on_proxy_protocol_changed(
        &mut self,
        _state: Entity<SelectState<Vec<&'static str>>>,
        event: &SelectEvent<Vec<&'static str>>,
        _cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(_) = event;
        self.save_proxy_config(_cx);
    }

    fn on_proxy_input_changed(
        &mut self,
        _state: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Blur = event {
            self.save_proxy_config(cx);
        }
    }

    fn on_proxy_password_changed(
        &mut self,
        _state: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                self.proxy_password_changed = true;
            }
            InputEvent::Blur => {
                if self.proxy_password_changed {
                    self.save_proxy_config(cx);
                }
            }
            _ => {}
        }
    }

    fn on_rpc_input_changed(
        &mut self,
        _state: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Blur = event {
            self.save_rpc_config(cx);
        }
    }

    fn on_java_mode_changed(
        &mut self,
        _state: Entity<SelectState<Vec<&'static str>>>,
        event: &SelectEvent<Vec<&'static str>>,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(_) = event;
        let mode = self.get_java_runtime_mode(cx);
        if let Some(config) = &mut self.backend_config {
            if config.java_runtime.mode == mode {
                return;
            }
            config.java_runtime.mode = mode;
        }
        self.backend_handle.send(MessageToBackend::SetJavaRuntimeMode { mode });
    }

    fn on_java_preferred_version_changed(
        &mut self,
        _state: Entity<SelectState<Vec<&'static str>>>,
        event: &SelectEvent<Vec<&'static str>>,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(_) = event;
        let major = self.get_java_runtime_preferred_version(cx);
        if let Some(config) = &mut self.backend_config {
            if config.java_runtime.preferred_major_version == major {
                return;
            }
            config.java_runtime.preferred_major_version = major;
        }
        self.backend_handle.send(MessageToBackend::SetJavaRuntimePreferredVersion { major });
    }

    fn get_proxy_config(&self, cx: &App) -> ProxyConfig {
        let protocol_name = self.proxy_protocol_select.read(cx).selected_value()
            .map(|s| *s)
            .unwrap_or("HTTP");

        ProxyConfig {
            enabled: self.proxy_enabled,
            protocol: ProxyProtocol::from_name(protocol_name),
            host: self.proxy_host_input.read(cx).value().to_string(),
            port: self.proxy_port_input.read(cx).value().parse().unwrap_or(8080),
            auth_enabled: self.proxy_auth_enabled,
            username: self.proxy_username_input.read(cx).value().to_string(),
        }
    }

    fn save_proxy_config(&mut self, cx: &mut Context<Self>) {
        let config = self.get_proxy_config(cx);

        if let Some(backend_config) = &mut self.backend_config {
            if !self.proxy_password_changed && backend_config.proxy == config {
                return;
            }
            backend_config.proxy = config.clone();
        }

        let password = if self.proxy_password_changed {
            Some(self.proxy_password_input.read(cx).value().to_string())
        } else {
            None
        };

        self.backend_handle.send(MessageToBackend::SetProxyConfiguration {
            config,
            password,
        });

        self.proxy_password_changed = false;
    }

    fn get_rpc_config(&self, cx: &App) -> DiscordRpcConfig {
        DiscordRpcConfig {
            enabled: self.rpc_enabled,
            show_advanced_details: self.rpc_show_advanced_details,
            client_id: self.rpc_client_id_input.read(cx).value().trim().to_string(),
            idle_text: self.rpc_idle_text_input.read(cx).value().trim().to_string(),
            selecting_text: self.rpc_selecting_text_input.read(cx).value().trim().to_string(),
            playing_text: self.rpc_playing_text_input.read(cx).value().trim().to_string(),
        }
    }

    fn save_rpc_config(&mut self, cx: &mut Context<Self>) {
        let config = self.get_rpc_config(cx);
        if let Some(backend_config) = &mut self.backend_config {
            if backend_config.discord_rpc == config {
                return;
            }
            backend_config.discord_rpc = config.clone();
        }

        self.backend_handle.send(MessageToBackend::SetDiscordRpcConfiguration { config });
    }

    fn get_java_runtime_mode(&self, cx: &App) -> JavaRuntimeMode {
        match self.java_mode_select.read(cx).selected_value().copied().unwrap_or("Auto") {
            "System" => JavaRuntimeMode::System,
            "Bundled" => JavaRuntimeMode::Bundled,
            _ => JavaRuntimeMode::Auto,
        }
    }

    fn get_java_runtime_preferred_version(&self, cx: &App) -> Option<u8> {
        match self.java_preferred_version_select.read(cx).selected_value().copied().unwrap_or("Game Default") {
            "Java 25" => Some(25),
            "Java 21" => Some(21),
            "Java 17" => Some(17),
            "Java 8" => Some(8),
            _ => None,
        }
    }

    fn render_interface_tab(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let interface_config = InterfaceConfig::get(cx);

        let mut div = v_flex()
            .px_4()
            .py_3()
            .gap_3()
            .child(crate::labelled(
                ts!("settings.theme.title"),
                Select::new(&self.theme_select)
            ))
            .child(Button::new("open-theme-folder").info().icon(PandoraIcon::FolderOpen).label(ts!("settings.theme.open_folder")).on_click({
                let theme_folder = self.theme_folder.clone();
                move |_, window, cx| {
                    crate::open_folder(&theme_folder, window, cx);
                }
            }))
            .child(Button::new("open-theme-repo").info().icon(PandoraIcon::Globe).label(ts!("settings.theme.open_repo")).on_click({
                move |_, _, cx| {
                    cx.open_url("https://github.com/longbridge/gpui-component/tree/main/themes");
                }
            }))
            .child(crate::labelled(ts!("settings.delete.title"),
                v_flex().gap_2()
                    .child(Checkbox::new("confirm-delete-mods")
                        .label(ts!("settings.delete.skip_mod_delete_confirmation"))
                        .checked(interface_config.quick_delete_mods)
                        .on_click(|value, _, cx| {
                            InterfaceConfig::get_mut(cx).quick_delete_mods = *value;
                        }))
                    .child(Checkbox::new("confirm-delete-instance")
                        .label(ts!("settings.delete.skip_instance_delete_confirmation"))
                        .checked(interface_config.quick_delete_instance).on_click(|value, _, cx| {
                            InterfaceConfig::get_mut(cx).quick_delete_instance = *value;
                        }))
                    )
            );

        if let Some(backend_config) = &self.backend_config {
            div = div
                .child(crate::labelled(
                    ts!("settings.windows.title"),
                    v_flex().gap_2()
                        .child(Checkbox::new("hide-on-launch")
                            .label(ts!("settings.windows.hide_main_window"))
                            .checked(interface_config.hide_main_window_on_launch)
                            .on_click(|value, _, cx| {
                                InterfaceConfig::get_mut(cx).hide_main_window_on_launch = *value;
                            }))
                        .child(Checkbox::new("open-game-output")
                            .label(ts!("settings.windows.open_game_output"))
                            .checked(!backend_config.dont_open_game_output_when_launching)
                            .on_click(cx.listener({
                                let backend_handle = self.backend_handle.clone();
                                move |settings, value, window, cx| {
                                    backend_handle.send(MessageToBackend::SetOpenGameOutputAfterLaunching {
                                        value: *value
                                    });
                                    settings.update_backend_configuration(window, cx);
                                }
                            })))
                        .child(Checkbox::new("quit-on-main-close")
                            .label(ts!("settings.windows.close_all_when_main_closed"))
                            .checked(interface_config.quit_on_main_closed)
                            .on_click(|value, _, cx| {
                                InterfaceConfig::get_mut(cx).quit_on_main_closed = *value;
                            }))
                        .child(Checkbox::new("developer-mode")
                            .label(ts!("settings.developer_mode"))
                            .checked(backend_config.developer_mode)
                            .on_click(cx.listener({
                                let backend_handle = self.backend_handle.clone();
                                move |settings, value, window, cx| {
                                    backend_handle.send(MessageToBackend::SetDeveloperMode {
                                        value: *value
                                    });
                                    if let Some(config) = &mut settings.backend_config {
                                        config.developer_mode = *value;
                                    }
                                    settings.update_backend_configuration(window, cx);
                                }
                            })))
                ));

            div = div
                .child(crate::labelled(
                    ts!("settings.discord.title"),
                    v_flex().gap_2()
                        .child(Checkbox::new("discord-rpc-enabled")
                            .label(ts!("settings.discord.enabled"))
                            .checked(self.rpc_enabled)
                            .on_click(cx.listener(|settings, value, _, cx| {
                                settings.rpc_enabled = *value;
                                settings.save_rpc_config(cx);
                                cx.notify();
                            })))
                        .child(Checkbox::new("discord-rpc-advanced")
                            .label(ts!("settings.discord.show_advanced"))
                            .checked(self.rpc_show_advanced_details)
                            .disabled(!self.rpc_enabled)
                            .on_click(cx.listener(|settings, value, _, cx| {
                                settings.rpc_show_advanced_details = *value;
                                settings.save_rpc_config(cx);
                                cx.notify();
                            })))
                        .child(crate::labelled(
                            ts!("settings.discord.client_id"),
                            Input::new(&self.rpc_client_id_input).disabled(!self.rpc_enabled),
                        ))
                        .child(crate::labelled(
                            ts!("settings.discord.idle_text"),
                            Input::new(&self.rpc_idle_text_input).disabled(!self.rpc_enabled),
                        ))
                        .child(crate::labelled(
                            ts!("settings.discord.selecting_text"),
                            Input::new(&self.rpc_selecting_text_input).disabled(!self.rpc_enabled),
                        ))
                        .child(crate::labelled(
                            ts!("settings.discord.playing_text"),
                            Input::new(&self.rpc_playing_text_input).disabled(!self.rpc_enabled),
                        ))
                ));

            div = div
                .child(crate::labelled(
                    ts!("settings.java_runtime.title"),
                    v_flex().gap_2()
                        .child(crate::labelled(
                            ts!("settings.java_runtime.mode"),
                            Select::new(&self.java_mode_select),
                        ))
                        .child(crate::labelled(
                            ts!("settings.java_runtime.preferred_version"),
                            Select::new(&self.java_preferred_version_select),
                        ))
                        .child(gpui::div().text_sm().text_color(cx.theme().muted_foreground).child(ts!("settings.java_runtime.note")))
                ));
        } else {
            div = div.child(Spinner::new().large());
        }

        div = div.child(crate::labelled(ts!("settings.privacy.title"),
            v_flex().gap_2()
                .child(Checkbox::new("hide-usernames")
                    .label(ts!("settings.privacy.hide_usernames"))
                    .checked(interface_config.hide_usernames)
                    .on_click(|value, _, cx| {
                        InterfaceConfig::get_mut(cx).hide_usernames = *value;
                    }))
                .child(Checkbox::new("hide-server-addresses")
                    .label(ts!("settings.privacy.hide_server_addresses"))
                    .checked(interface_config.hide_server_addresses)
                    .on_click(|value, _, cx| {
                        InterfaceConfig::get_mut(cx).hide_server_addresses = *value;
                    }))
        ));

        div
    }

    fn render_network_tab(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let proxy_enabled = self.proxy_enabled;
        let proxy_auth_enabled = self.proxy_auth_enabled;

        v_flex()
            .px_4()
            .py_3()
            .gap_3()
            .child(crate::labelled(
                ts!("settings.proxy.title"),
                v_flex().gap_2()
                    .child(Checkbox::new("proxy-enabled")
                        .label(ts!("settings.proxy.enabled"))
                        .checked(proxy_enabled)
                        .on_click(cx.listener(|settings, value, _, cx| {
                            settings.proxy_enabled = *value;
                            settings.save_proxy_config(cx);
                            cx.notify();
                        })))
                    .child(h_flex().gap_2()
                        .child(v_flex().gap_1().w_32()
                            .child(ts!("settings.proxy.protocol"))
                            .child(Select::new(&self.proxy_protocol_select)
                                .disabled(!proxy_enabled)
                                .w_full()))
                        .child(v_flex().gap_1().flex_1()
                            .child(ts!("settings.proxy.host"))
                            .child(Input::new(&self.proxy_host_input)
                                .disabled(!proxy_enabled)))
                        .child(v_flex().gap_1().w_32()
                            .child(ts!("settings.proxy.port"))
                            .child(NumberInput::new(&self.proxy_port_input)
                                .disabled(!proxy_enabled))))
            ))
            .child(crate::labelled(
                ts!("settings.proxy.auth"),
                v_flex().gap_2()
                    .child(Checkbox::new("proxy-auth-enabled")
                        .label(ts!("settings.proxy.use_auth"))
                        .checked(proxy_auth_enabled)
                        .disabled(!proxy_enabled)
                        .on_click(cx.listener(|settings, value, _, cx| {
                            settings.proxy_auth_enabled = *value;
                            settings.save_proxy_config(cx);
                            cx.notify();
                        })))
                    .child(h_flex().gap_2()
                        .child(v_flex().gap_1().flex_1()
                            .child(ts!("settings.proxy.username"))
                            .child(Input::new(&self.proxy_username_input)
                                .disabled(!proxy_enabled || !proxy_auth_enabled)))
                        .child(v_flex().gap_1().flex_1()
                            .child(ts!("settings.proxy.password"))
                            .child(Input::new(&self.proxy_password_input)
                                .disabled(!proxy_enabled || !proxy_auth_enabled))))
            ))
            .child(div()
                .pt_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(ts!("settings.proxy.launcher_only_note")))
    }
}
impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_tab = self.selected_tab;

        let tab_bar = TabBar::new("settings-tabs")
            .prefix(div().w_4())
            .selected_index(match selected_tab {
                SettingsTab::Interface => 0,
                SettingsTab::Network => 1,
            })
            .underline()
            .child(Tab::new().label(ts!("settings.interface")))
            .child(Tab::new().label(ts!("settings.network")))
            .on_click(cx.listener(|settings, index, _window, cx| {
                settings.selected_tab = match index {
                    0 => SettingsTab::Interface,
                    1 => SettingsTab::Network,
                    _ => SettingsTab::Interface,
                };
                cx.notify();
            }));

        let content = match selected_tab {
            SettingsTab::Interface => self.render_interface_tab(window, cx).into_any_element(),
            SettingsTab::Network => self.render_network_tab(window, cx).into_any_element(),
        };

        v_flex()
            .child(tab_bar)
            .child(content)
    }
}
