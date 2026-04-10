use std::{collections::VecDeque, sync::Arc};

use bridge::{instance::InstanceID, message::{DiscordRpcUiState, MessageToBackend}};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Icon, InteractiveElementExt, WindowExt, button::{Button, ButtonVariants}, h_flex, notification::{Notification, NotificationType}, scroll::ScrollableElement, tooltip::Tooltip, v_flex
};
use rustc_hash::FxHashMap;
use schema::pandora_update::UpdatePrompt;
use serde::{Deserialize, Serialize};

use crate::{
    component::{menu::{MenuGroup, MenuGroupItem}, page_path::PagePath, resize_panel::{ResizePanel, ResizePanelState}, shrinking_text::ShrinkingText, title_bar::TitleBar}, entity::{
        DataEntities, account::AccountExt, instance::{InstanceAddedEvent, InstanceEntries, InstanceModifiedEvent, InstanceMovedToTopEvent, InstanceRemovedEvent}
    }, icon::PandoraIcon, interface_config::InterfaceConfig, modals, pages::{curseforge_page::CurseforgeSearchPage, import::ImportPage, instance::instance_page::InstancePage, instances_page::InstancesPage, integrity_modpack_page::IntegrityModpackPage, modrinth_page::ModrinthSearchPage, modrinth_project_page::ModrinthProjectPage, page::Page, skins_page::SkinsPage, syncing_page::SyncingPage}, png_render_cache, ts
};

pub struct LauncherUI {
    data: DataEntities,
    page: LauncherPage,
    pub update: Option<UpdatePrompt>,
    sidebar_state: ResizePanelState,
    recent_instances: heapless::Vec<(InstanceID, SharedString), 3>,
    page_history_backwards: VecDeque<(PageType, Arc<[PageType]>)>,
    page_history_forwards: Vec<(PageType, Arc<[PageType]>)>,
    previous_pages: FxHashMap<PageType, LauncherPage>,
    _instance_added_subscription: Subscription,
    _instance_modified_subscription: Subscription,
    _instance_removed_subscription: Subscription,
    _instance_moved_to_top_subscription: Subscription,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PageType {
    #[default]
    Instances,
    Skins,
    Modrinth {
        installing_for: Option<SharedString>,
    },
    Curseforge {
        installing_for: Option<SharedString>,
    },
    Import,
    IntegrityModpack,
    Syncing,
    ModrinthProject {
        project_id: SharedString,
        project_title: SharedString,
        install_for: Option<SharedString>,
    },
    InstancePage {
        name: SharedString,
    },
}

impl PageType {
    pub fn title(&self, data: &DataEntities, cx: &App) -> SharedString {
        match self {
            PageType::Instances => ts!("instance.title"),
            PageType::Skins => ts!("skins.title"),
            PageType::Modrinth { installing_for } => {
                if installing_for.is_some() {
                    ts!("instance.content.install.from_modrinth")
                } else {
                    ts!("modrinth.name")
                }
            },
            PageType::Curseforge { installing_for } => {
                if installing_for.is_some() {
                    ts!("instance.content.install.from_curseforge")
                } else {
                    ts!("curseforge.name")
                }
            },
            PageType::Import => "Import".into(),
            PageType::IntegrityModpack => ts!("modpack.integrity.nav"),
            PageType::Syncing => ts!("instance.sync.label"),
            PageType::ModrinthProject { project_title, .. } => project_title.clone(),
            PageType::InstancePage { name } => {
                InstanceEntries::find_title_by_name(&data.instances, name, cx)
                    .unwrap_or_else(|| name.clone())
            },
        }
    }
}

#[derive(Clone)]
pub enum LauncherPage {
    Instances(Entity<InstancesPage>),
    Skins(Entity<SkinsPage>),
    Modrinth(Entity<ModrinthSearchPage>),
    Curseforge(Entity<CurseforgeSearchPage>),
    Import(Entity<ImportPage>),
    IntegrityModpack(Entity<IntegrityModpackPage>),
    Syncing(Entity<SyncingPage>),
    ModrinthProject(Entity<ModrinthProjectPage>),
    InstancePage(Entity<InstancePage>),
}

impl LauncherPage {
    fn render(self, ui: &LauncherUI, window: &mut Window, cx: &mut App) -> impl IntoElement {
        fn process(entity: Entity<impl Page>, window: &mut Window, cx: &mut App) -> (bool, AnyElement, AnyElement) {
            entity.update(cx, |page, cx| {
                (page.scrollable(cx), page.controls(window, cx).into_any_element(), page.render(window, cx).into_any_element())
            })
        }

        let (scrollable, controls, page) = match self {
            LauncherPage::Instances(entity) => process(entity, window, cx),
            LauncherPage::Skins(entity) => process(entity, window, cx),
            LauncherPage::Modrinth(entity) => process(entity, window, cx),
            LauncherPage::Curseforge(entity) => process(entity, window, cx),
            LauncherPage::Import(entity) => process(entity, window, cx),
            LauncherPage::IntegrityModpack(entity) => process(entity, window, cx),
            LauncherPage::Syncing(entity) => process(entity, window, cx),
            LauncherPage::ModrinthProject(entity) => process(entity, window, cx),
            LauncherPage::InstancePage(entity) => process(entity, window, cx),
        };

        let config = InterfaceConfig::get(cx);
        let page_path = PagePath::new(ui.data.clone(), config.main_page.clone(), config.page_path.clone());
        let title_bar = TitleBar {
            page_path,
            controls,
            update: ui.update.clone(),
            send: ui.data.backend_handle.clone(),
        };

        if scrollable {
            v_flex()
                .size_full()
                .child(title_bar)
                .child(div().flex_1().overflow_hidden().child(
                    v_flex().size_full().overflow_y_scrollbar().child(page),
                ))
        } else {
            v_flex()
                .size_full()
                .child(title_bar)
                .child(page)
        }
    }
}

impl LauncherUI {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let recent_instances = data
            .instances
            .read(cx)
            .entries
            .iter()
            .take(3)
            .map(|(id, ent)| (*id, ent.read(cx).name.clone()))
            .collect();

        let _instance_added_subscription =
            cx.subscribe::<_, InstanceAddedEvent>(&data.instances, |this, _, event, cx| {
                if this.recent_instances.is_full() {
                    this.recent_instances.pop();
                }
                let _ = this.recent_instances.insert(0, (event.instance.id, event.instance.name.clone()));
                cx.notify();
            });
        let _instance_modified_subscription =
            cx.subscribe_in::<_, InstanceModifiedEvent>(&data.instances, window, |this, _, event, window, cx| {
                if let Some((_, name)) = this.recent_instances.iter_mut().find(|(id, _)| *id == event.instance.id) {
                    *name = event.instance.name.clone();
                    cx.notify();
                }
                if let LauncherPage::InstancePage(page) = &this.page
                    && page.read(cx).instance.read(cx).id == event.instance.id
                {
                    let page_path = InterfaceConfig::get_mut(cx).page_path.clone();
                    this.switch_page(PageType::InstancePage { name: event.instance.name.clone() }, &*page_path, window, cx);
                }
                cx.notify();
            });
        let _instance_removed_subscription =
            cx.subscribe_in::<_, InstanceRemovedEvent>(&data.instances, window, |this, _, event, window, cx| {
                this.recent_instances.retain(|entry| entry.0 != event.id);

                if let LauncherPage::InstancePage(page) = &this.page
                    && page.read(cx).instance.read(cx).id == event.id
                {
                    this.switch_page(PageType::Instances, &[], window, cx);
                }
                cx.notify();
            });
        let _instance_moved_to_top_subscription =
            cx.subscribe::<_, InstanceMovedToTopEvent>(&data.instances, |this, _, event, cx| {
                this.recent_instances.retain(|entry| entry.0 != event.instance.id);
                if this.recent_instances.is_full() {
                    this.recent_instances.pop();
                }
                let _ = this.recent_instances.insert(0, (event.instance.id, event.instance.name.clone()));
                cx.notify();
            });

        let config = InterfaceConfig::get(cx);

        let mut default_sidebar_width = config.sidebar_width;
        if default_sidebar_width <= 0.0 {
            default_sidebar_width = 150.0;
        }

        let sidebar_state = ResizePanelState::new(px(default_sidebar_width), px(150.0), px(225.0))
            .on_resize(|width, _, cx| {
                InterfaceConfig::get_mut(cx).sidebar_width = width.as_f32();
            });

        let main_page = config.main_page.clone();

        // If main_page failed to deserialize, also reset the path
        if main_page == PageType::Instances {
            let config = InterfaceConfig::get_mut(cx);
            config.page_path = [].into();
        }

        let page = match Self::create_page(&data, main_page.clone(), window, cx) {
            Ok(page) => page,
            Err(page_type) => {
                let config = InterfaceConfig::get_mut(cx);
                config.main_page = page_type.clone();
                config.page_path = [].into();
                Self::create_page(&data, page_type, window, cx).unwrap()
            },
        };

        Self {
            data: data.clone(),
            page,
            update: None,
            sidebar_state,
            recent_instances,
            page_history_backwards: VecDeque::with_capacity(32),
            page_history_forwards: Vec::new(),
            previous_pages: FxHashMap::default(),
            _instance_added_subscription,
            _instance_modified_subscription,
            _instance_removed_subscription,
            _instance_moved_to_top_subscription,
        }
    }

    fn create_page(data: &DataEntities, page: PageType, window: &mut Window, cx: &mut Context<Self>) -> Result<LauncherPage, PageType> {
        match page {
            PageType::Instances => {
                Ok(LauncherPage::Instances(cx.new(|cx| InstancesPage::new(data, window, cx))))
            },
            PageType::Skins => {
                Ok(LauncherPage::Skins(cx.new(|cx| SkinsPage::new(data, window, cx))))
            },
            PageType::Modrinth { installing_for } => {
                let installing_for = installing_for.as_ref().and_then(|name| InstanceEntries::find_id_by_name(&data.instances, name, cx));

                let page = cx.new(|cx| {
                    ModrinthSearchPage::new(installing_for, data, window, cx)
                });
                Ok(LauncherPage::Modrinth(page))
            },
            PageType::Curseforge { installing_for } => {
                let installing_for = installing_for.as_ref().and_then(|name| InstanceEntries::find_id_by_name(&data.instances, name, cx));

                let page = cx.new(|cx| {
                    CurseforgeSearchPage::new(installing_for, data, window, cx)
                });
                Ok(LauncherPage::Curseforge(page))
            },
            PageType::Import => {
                Ok(LauncherPage::Import(cx.new(|cx| ImportPage::new(data, window, cx))))
            },
            PageType::IntegrityModpack => {
                Ok(LauncherPage::IntegrityModpack(cx.new(|_| IntegrityModpackPage::new(data))))
            }
            PageType::Syncing => {
                Ok(LauncherPage::Syncing(cx.new(|cx| SyncingPage::new(data, window, cx))))
            },
            PageType::ModrinthProject { project_id, install_for, .. } => {
                let install_for_id = install_for.as_ref().and_then(|name| InstanceEntries::find_id_by_name(&data.instances, name, cx));

                let project_id = project_id.clone();
                let page = cx.new(|cx| {
                    ModrinthProjectPage::new(project_id, install_for_id, data, window, cx,)
                });
                Ok(LauncherPage::ModrinthProject(page))
            },
            PageType::InstancePage { ref name } => {
                let Some(id) = InstanceEntries::find_id_by_name(&data.instances, name, cx) else {
                    return Err(PageType::Instances);
                };

                Ok(LauncherPage::InstancePage(cx.new(|cx| {
                    InstancePage::new(id, data, window, cx)
                })))
            },
        }
    }

    pub fn switch_page(&mut self, page: PageType, page_path: &[PageType], window: &mut Window, cx: &mut Context<Self>) {
        let page_path: Arc<[PageType]> = page_path.into();

        let config = InterfaceConfig::get(cx);
        if config.main_page == page {
            return;
        }

        self.page_history_forwards.clear();
        if self.page_history_backwards.len() >= 32 {
            self.page_history_backwards.pop_back();
        }
        self.page_history_backwards.push_front((config.main_page.clone(), config.page_path.clone()));

        self.switch_page_without_history(page, page_path, window, cx);
    }

    fn switch_page_without_history(&mut self, page: PageType, page_path: Arc<[PageType]>, window: &mut Window, cx: &mut Context<Self>) {
        let config = InterfaceConfig::get_mut(cx);
        let previous_page_type = std::mem::replace(&mut config.main_page, page.clone());
        config.main_page = page.clone();
        config.page_path = page_path.clone();

        match &page {
            PageType::InstancePage { name } => {
                self.data.backend_handle.send(MessageToBackend::SetDiscordRpcUiState {
                    state: DiscordRpcUiState::SelectingInstance,
                    selected_instance: Some(name.to_string().into()),
                });
            }
            _ => {
                self.data.backend_handle.send(MessageToBackend::SetDiscordRpcUiState {
                    state: DiscordRpcUiState::IdleInLauncher,
                    selected_instance: None,
                });
            }
        }

        if let Some(previous_page) = self.previous_pages.remove(&page) {
            self.page = previous_page;
            self.previous_pages.retain(|k, _| page_path.contains(k));
            cx.notify();
            return;
        }

        match Self::create_page(&self.data, page, window, cx) {
            Ok(page) => {
                let previous_page = std::mem::replace(&mut self.page, page);
                if page_path.contains(&previous_page_type) {
                    self.previous_pages.insert(previous_page_type, previous_page);
                }
                self.previous_pages.retain(|k, _| page_path.contains(k));
            },
            Err(fallback) => {
                let config = InterfaceConfig::get_mut(cx);
                config.main_page = fallback.clone();
                config.page_path = [].into();
                self.previous_pages.clear();
                self.page = Self::create_page(&self.data, fallback, window, cx).unwrap();
            },
        }

        cx.notify();
    }

    pub fn nav_backwards(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((page, page_path)) = self.page_history_backwards.pop_front() else {
            return;
        };

        let config = InterfaceConfig::get(cx);
        self.page_history_forwards.push((config.main_page.clone(), config.page_path.clone()));

        self.switch_page_without_history(page, page_path, window, cx);
    }

    pub fn nav_forwards(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((page, page_path)) = self.page_history_forwards.pop() else {
            return;
        };

        let config = InterfaceConfig::get(cx);
        self.page_history_backwards.push_front((config.main_page.clone(), config.page_path.clone()));

        self.switch_page_without_history(page, page_path, window, cx);
    }
}

impl Render for LauncherUI {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page_type = InterfaceConfig::get(cx).main_page.clone();

        let library_group = MenuGroup::new("Minecraft")
            .child(MenuGroupItem::new(ts!("instance.title"))
                .active(page_type == PageType::Instances)
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Instances, &[], window, cx);
                })))
            .child(MenuGroupItem::new(ts!("skins.title"))
                .active(page_type == PageType::Skins)
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Skins, &[], window, cx);
                })));

        let content_group = MenuGroup::new(ts!("instance.content.title"))
            .child(MenuGroupItem::new(ts!("modrinth.name"))
                .active(matches!(page_type, PageType::Modrinth { installing_for: None } | PageType::ModrinthProject { install_for: None, .. }))
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Modrinth { installing_for: None }, &[], window, cx);
                })))
            .child(MenuGroupItem::new(ts!("curseforge.name"))
                .active(matches!(page_type, PageType::Curseforge { installing_for: None }))
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Curseforge { installing_for: None }, &[], window, cx);
                })));

        let files_group = MenuGroup::new("Files")
            .child(MenuGroupItem::new("Import")
                .active(page_type == PageType::Import)
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Import, &[], window, cx);
                })))
            .child(MenuGroupItem::new(ts!("modpack.integrity.nav"))
                .active(page_type == PageType::IntegrityModpack)
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::IntegrityModpack, &[], window, cx);
                })))
            .child(MenuGroupItem::new(ts!("instance.sync.label"))
                .active(page_type == PageType::Syncing)
                .on_click(cx.listener(|launcher, _, window, cx| {
                    launcher.switch_page(PageType::Syncing, &[], window, cx);
                })));

        let mut groups: heapless::Vec<MenuGroup, 4> = heapless::Vec::new();

        let _ = groups.push(library_group);
        let _ = groups.push(content_group);
        let _ = groups.push(files_group);

        if !self.recent_instances.is_empty() {
            let mut recent_instances_group = MenuGroup::new(ts!("instance.recent"));

            for (_, name) in &self.recent_instances {
                let name = name.clone();
                let active = page_type == PageType::InstancePage { name: name.clone() };
                let item = MenuGroupItem::new(name.clone())
                    .active(active)
                    .on_click(cx.listener(move |launcher, _, window, cx| {
                        launcher.switch_page(PageType::InstancePage { name: name.clone() }, &[PageType::Instances], window, cx);
                    }));
                recent_instances_group = recent_instances_group.child(item);
            }

            let _ = groups.push(recent_instances_group);
        }

        let selected_account = self.data.accounts.read(cx).selected_account.clone();
        let has_accounts = !self.data.accounts.read(cx).accounts.is_empty();
        let (account_head, account_name, account_type) = if let Some(account) = selected_account {
            let account_name = account.username(InterfaceConfig::get(cx).hide_usernames);
            let head = if let Some(head) = &account.head {
                let resize = png_render_cache::ImageTransformation::Resize { width: 32, height: 32 };
                png_render_cache::render_with_transform(head.clone(), resize, cx)
            } else {
                gpui::img(ImageSource::Resource(Resource::Embedded("images/default_head.png".into())))
            };
            let account_type = if account.offline {
                ts!("account.type.offline")
            } else {
                ts!("account.type.microsoft")
            };
            (head, account_name, account_type)
        } else {
            (
                gpui::img(ImageSource::Resource(Resource::Embedded("images/default_head.png".into()))),
                ts!("account.none"),
                SharedString::new_static(""),
            )
        };

        let account_button = h_flex()
            .id("account-button")
            .flex_1()
            .flex_shrink_0()
            .p_2()
            .max_w_full()
            .gap_2()
            .justify_center()
            .text_size(rems(0.9375))
            .line_height(rems(1.0))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .child(account_head.size_8().min_w_8().min_h_8())
            .child(v_flex()
                .min_w_0()
                .gap_0p5()
                .child(ShrinkingText::new(account_name))
                .when(!account_type.is_empty(), |this| {
                    this.child(div().text_xs().text_color(cx.theme().muted_foreground).child(account_type))
                }))
            .child(PandoraIcon::ChevronRight)
            .tooltip(move |window, cx| {
                Tooltip::new(ts!("account.manage.open")).build(window, cx)
            })
            .on_click({
                let accounts = self.data.accounts.clone();
                let backend_handle = self.data.backend_handle.clone();
                move |_, window, cx| {
                    if accounts.read(cx).accounts.is_empty() {
                        crate::root::start_new_account_login(&backend_handle, window, cx);
                        return;
                    }

                    let accounts = accounts.clone();
                    let backend_handle = backend_handle.clone();
                    window.open_sheet_at(gpui_component::Placement::Left, cx, move |sheet, _, cx| {
                        let (accounts, selected_account) = {
                            let accounts = accounts.read(cx);
                            (accounts.accounts.clone(), accounts.selected_account_uuid)
                        };

                        let items = accounts.iter().map(|account| {
                            let head = if let Some(head) = &account.head {
                                let resize = png_render_cache::ImageTransformation::Resize { width: 32, height: 32 };
                                png_render_cache::render_with_transform(head.clone(), resize, cx)
                            } else {
                                gpui::img(ImageSource::Resource(Resource::Embedded("images/default_head.png".into())))
                            };
                            let account_name = account.username(InterfaceConfig::get(cx).hide_usernames);

                            let selected = Some(account.uuid) == selected_account;
                            let account_type = if account.offline {
                                ts!("account.type.offline")
                            } else {
                                ts!("account.type.microsoft")
                            };

                            h_flex()
                                .gap_2()
                                .w_full()
                                .child(Button::new(account_name.clone())
                                    .min_w_0()
                                    .flex_1()
                                    .when(selected, |this| {
                                        this.info()
                                    })
                                    .h_10()
                                    .child(head.size_8().min_w_8().min_h_8())
                                    .child(v_flex()
                                        .pt_0p5()
                                        .gap_0p5()
                                        .child(div().line_clamp(2).line_height(rems(1.0)).child(account_name.clone()))
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(account_type)))
                                    .when(!selected, |this| {
                                        this.on_click({
                                            let backend_handle = backend_handle.clone();
                                            let uuid = account.uuid;
                                            move |_, _, _| {
                                                backend_handle.send(MessageToBackend::SelectAccount { uuid });
                                            }
                                        })
                                    }))
                                .child(Button::new((account_name.clone(), 1))
                                    .icon(PandoraIcon::Trash2)
                                    .h_10()
                                    .w_10()
                                    .danger()
                                    .on_click({
                                        let backend_handle = backend_handle.clone();
                                        let uuid = account.uuid;
                                        move |_, _, _| {
                                            backend_handle.send(MessageToBackend::DeleteAccount { uuid });
                                        }
                                    }))

                        });

                        sheet
                            .when(cfg!(target_os = "macos"), |this| this.pt_5())
                            .title(ts!("account.title"))
                            .child(v_flex()
                                .gap_2()
                                .child(Button::new("add-account").h_10().success().icon(PandoraIcon::Plus).label(ts!("account.add.label")).on_click({
                                    let backend_handle = backend_handle.clone();
                                    move |_, window, cx| {
                                        crate::root::start_new_account_login(&backend_handle, window, cx);
                                    }
                                }))
                                .children(items)
                            )

                    });
                }
            });

        let account_manager_button = Button::new("account-manager")
            .h_8()
            .w_full()
            .flex_shrink_0()
            .icon(PandoraIcon::Plus)
            .label(if has_accounts { ts!("account.manage.open") } else { ts!("account.manage.add_first") })
            .when(has_accounts, |this| this.info())
            .when(!has_accounts, |this| this.success())
            .on_click({
                let accounts = self.data.accounts.clone();
                let backend_handle = self.data.backend_handle.clone();
                move |_, window, cx| {
                    if accounts.read(cx).accounts.is_empty() {
                        crate::root::start_new_account_login(&backend_handle, window, cx);
                        return;
                    }

                    let accounts = accounts.clone();
                    let backend_handle = backend_handle.clone();
                    window.open_sheet_at(gpui_component::Placement::Left, cx, move |sheet, _, cx| {
                        let (accounts, selected_account) = {
                            let accounts = accounts.read(cx);
                            (accounts.accounts.clone(), accounts.selected_account_uuid)
                        };

                        let items = accounts.iter().map(|account| {
                            let head = if let Some(head) = &account.head {
                                let resize = png_render_cache::ImageTransformation::Resize { width: 32, height: 32 };
                                png_render_cache::render_with_transform(head.clone(), resize, cx)
                            } else {
                                gpui::img(ImageSource::Resource(Resource::Embedded("images/default_head.png".into())))
                            };
                            let account_name = account.username(InterfaceConfig::get(cx).hide_usernames);

                            let selected = Some(account.uuid) == selected_account;
                            let account_type = if account.offline {
                                ts!("account.type.offline")
                            } else {
                                ts!("account.type.microsoft")
                            };

                            h_flex()
                                .gap_2()
                                .w_full()
                                .child(Button::new(account_name.clone())
                                    .min_w_0()
                                    .flex_1()
                                    .when(selected, |this| {
                                        this.info()
                                    })
                                    .h_10()
                                    .child(head.size_8().min_w_8().min_h_8())
                                    .child(v_flex()
                                        .pt_0p5()
                                        .gap_0p5()
                                        .child(div().line_clamp(2).line_height(rems(1.0)).child(account_name.clone()))
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(account_type)))
                                    .when(!selected, |this| {
                                        this.on_click({
                                            let backend_handle = backend_handle.clone();
                                            let uuid = account.uuid;
                                            move |_, _, _| {
                                                backend_handle.send(MessageToBackend::SelectAccount { uuid });
                                            }
                                        })
                                    }))
                                .child(Button::new((account_name.clone(), 1))
                                    .icon(PandoraIcon::Trash2)
                                    .h_10()
                                    .w_10()
                                    .danger()
                                    .on_click({
                                        let backend_handle = backend_handle.clone();
                                        let uuid = account.uuid;
                                        move |_, _, _| {
                                            backend_handle.send(MessageToBackend::DeleteAccount { uuid });
                                        }
                                    }))

                        });

                        sheet
                            .when(cfg!(target_os = "macos"), |this| this.pt_5())
                            .title(ts!("account.title"))
                            .child(v_flex()
                                .gap_2()
                                .child(Button::new("add-account").h_10().success().icon(PandoraIcon::Plus).label(ts!("account.add.label")).on_click({
                                    let backend_handle = backend_handle.clone();
                                    move |_, window, cx| {
                                        crate::root::start_new_account_login(&backend_handle, window, cx);
                                    }
                                }))
                                .children(items)
                            )

                    });
                }
            });

        let settings_button = div()
            .id("settings-button")
            .p_2()
            .rounded(cx.theme().radius)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .child(PandoraIcon::Settings)
            .on_click({
                let data = self.data.clone();
                move |_, window, cx| {
                    let build = modals::settings::build_settings_sheet(&data, window, cx);
                    window.open_sheet_at(gpui_component::Placement::Left, cx, build);
                }
            });
        let bug_report_button = div()
            .id("bug-report-button")
            .p_2()
            .rounded(cx.theme().radius)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .child(PandoraIcon::Bug)
            .tooltip(move |window, cx| {
                Tooltip::new("Report a bug").build(window, cx)
            })
            .on_click({
                move |_, window, cx| {
                    open_bug_report_url(window, cx);
                }
            });

        let header = h_flex()
            .when_else(cfg!(target_os = "macos"), |this| this.pt(px(9.0)), |this| this.pt(px(14.0)))
            .px_5()
            .pb_2()
            .gap_2()
            .w_full()
            .justify_center()
            .text_size(rems(0.9375))
            .child(Icon::new(PandoraIcon::Pandora).size_8().min_w_8().min_h_8())
            .child(ts!("common.app_name"));
        let footer_buttons = h_flex().child(settings_button).child(bug_report_button);
        let footer = v_flex()
            .pb_2()
            .px_2()
            .items_center()
            .min_w_full()
            .max_w_full()
            .w_full()
            .flex_shrink_0()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .gap_2()
            .child(footer_buttons)
            .child(account_manager_button)
            .child(account_button);
        let sidebar = v_flex()
            .size_full()
            .min_size_full()
            .max_size_full()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .when(cfg!(target_os = "macos"), |this| {
                this.child(h_flex()
                    .id("sidebar-double-clicker")
                    .w_full()
                    .h(px(32.0))
                    .on_double_click(|_, window, _| window.titlebar_double_click())
                )
            })
            .child(header)
            .child(v_flex()
                .flex_1()
                .min_h_0()
                .px_3()
                .gap_y_3()
                .children(groups)
                .overflow_y_scrollbar())
            .child(footer);

        ResizePanel::new(&self.sidebar_state, sidebar, self.page.clone().render(&self, window, cx))
    }
}

fn open_bug_report_url(window: &mut Window, cx: &mut App) {
    let mut body = String::from(r#"## Description of bug
(Write here)

## Steps to reproduce
(Write here)

## This issue is unique
- [ ] I've searched the other issues and didn't see an issue describing the same bug

## Environment
"#);

    use std::fmt::Write;
    _ = writeln!(&mut body, "Version: {:?}", option_env!("PANDORA_RELEASE_VERSION"));
    _ = writeln!(&mut body, "Distributor: {:?}", option_env!("PANDORA_DISTRIBUTION"));
    _ = writeln!(&mut body, "OS: {} ({})", std::env::consts::OS, std::env::consts::ARCH);

    if cfg!(target_os = "linux") {
        if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
            for line in os_release.lines() {
                let line = line.trim_ascii();
                if let Some(name) = line.strip_prefix("NAME=") {
                    _ = writeln!(&mut body, "OS Name: {}", name);
                } else if let Some(version) = line.strip_prefix("VERSION=") {
                    _ = writeln!(&mut body, "OS Version: {}", version);
                }
            }
        }

        _ = writeln!(&mut body, "Desktop: {:?}", std::env::var_os("XDG_CURRENT_DESKTOP"));

        if let Some(snap_name) = std::env::var_os("SNAP_NAME") {
            _ = writeln!(&mut body, "Snap: {:?}", snap_name);
        }
        if let Some(snap_name) = std::env::var_os("FLATPAK_ID") {
            _ = writeln!(&mut body, "Flatpak ID: {:?}", snap_name);
        }
        if std::env::var_os("APPIMAGE").is_some() {
            body.push_str("AppImage: true\n");
        }
    }

    let Some(github) = option_env!("GITHUB_REPOSITORY_URL") else {
        let mut notification: Notification = (NotificationType::Error, SharedString::from("Unable to report bug, GITHUB_REPOSITORY_URL was not set at compile time")).into();
        notification = notification.autohide(false);
        window.push_notification(notification, cx);
        return;
    };

    cx.open_url(&format!("{}/issues/new?body={}", github, urlencoding::encode(&body)));
}
