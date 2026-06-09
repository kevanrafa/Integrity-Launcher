use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Icon, WindowExt, button::{Button, ButtonVariants}, h_flex, skeleton::Skeleton, v_flex
};

use crate::{
    component::{error_alert::ErrorAlert, responsive_grid::ResponsiveGrid},
    icon::PandoraIcon,
    integrity_api::{self, IntegrityModpack, IntegrityModpackCatalog},
    pages::page::Page,
};

pub struct IntegrityModpacksPage {
    catalog: Option<IntegrityModpackCatalog>,
    error: Option<SharedString>,
    _load_task: Task<()>,
}

impl IntegrityModpacksPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            catalog: None,
            error: None,
            _load_task: Task::ready(()),
        };
        page.reload(cx);
        page
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.catalog = None;

        let client = cx.http_client();
        self._load_task = cx.spawn(async move |page, cx| {
            let result = integrity_api::load_modpack_catalog(client).await;
            let _ = page.update(cx, |page, cx| {
                match result {
                    Ok(catalog) => page.catalog = Some(catalog),
                    Err(error) => page.error = Some(SharedString::new(error.to_string())),
                }
                cx.notify();
            });
        });
    }

    fn render_section(
        &self,
        title: &'static str,
        items: &[IntegrityModpack],
        featured: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let card_size = Size::new(
            gpui::AvailableSpace::Definite(px(if featured { 340.0 } else { 300.0 })),
            gpui::AvailableSpace::MinContent,
        );

        v_flex()
            .gap_3()
            .child(div().text_xl().font_semibold().child(title))
            .when_else(items.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child("No modpacks available yet."),
                )
            }, |this| {
                this.child(
                    ResponsiveGrid::new(card_size)
                        .size_full()
                        .gap_4()
                        .children(items.iter().enumerate().map(|(index, modpack)| {
                            self.render_modpack_card(modpack, featured, index, window, cx)
                        })),
                )
            })
    }

    fn render_modpack_card(
        &self,
        modpack: &IntegrityModpack,
        featured: bool,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = cx.theme();
        let name = SharedString::new(modpack.display_name());
        let version = SharedString::new(modpack.display_version());
        let loader_line = SharedString::new(modpack.loader_line());
        let description = modpack
            .description
            .as_ref()
            .map(|value| SharedString::new(value.clone()))
            .unwrap_or_else(|| "No description provided.".into());

        let image = if let Some(icon_url) = &modpack.icon_url
            && !icon_url.is_empty()
        {
            gpui::img(SharedUri::from(icon_url.as_ref()))
                .with_fallback(|| Skeleton::new().rounded_lg().size_14().into_any_element())
                .into_any_element()
        } else {
            gpui::img(ImageSource::Resource(Resource::Embedded("images/default_mod.png".into())))
                .into_any_element()
        };

        let install_button = Button::new(("integrity-install", featured, index))
            .success()
            .icon(PandoraIcon::Download)
            .label(t::instance::content::install::label())
            .on_click({
                let download_url = modpack.download_url.clone();
                let name = name.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    if let Some(download_url) = &download_url {
                        crate::open_external_url(download_url, window, cx);
                    } else {
                        let notification = gpui_component::notification::Notification::new()
                            .autohide(false)
                            .with_type(gpui_component::notification::NotificationType::Error)
                            .title(format!("No download URL for {name}"));
                        window.push_notification(notification, cx);
                    }
                }
            });

        let changelog_button = modpack.changelog.as_ref().map(|changelog| {
            Button::new(("integrity-changelog", featured, index))
                .outline()
                .icon(PandoraIcon::BookOpen)
                .label("Changelog")
                .on_click({
                    let changelog = changelog.clone();
                    move |_, window, cx| {
                        cx.stop_propagation();
                        crate::open_external_url(&changelog, window, cx);
                    }
                })
        });

        v_flex()
            .gap_3()
            .p_4()
            .min_h(px(if featured { 230.0 } else { 210.0 }))
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                h_flex()
                    .gap_3()
                    .child(div().size_14().min_w_14().min_h_14().child(image))
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_0p5()
                            .child(div().text_lg().font_semibold().line_clamp(1).child(name))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(loader_line))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(version)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .line_height(px(20.0))
                    .line_clamp(if featured { 4 } else { 3 })
                    .child(description),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(install_button)
                    .when_some(changelog_button, |this, button| this.child(button)),
            )
    }
}

impl Page for IntegrityModpacksPage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("refresh-integrity-modpacks")
            .outline()
            .icon(PandoraIcon::RefreshCcw)
            .label("Refresh")
            .on_click(cx.listener(|page, _, _, cx| page.reload(cx)))
    }

    fn scrollable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for IntegrityModpacksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(error) = self.error.clone() {
            return div()
                .p_4()
                .child(ErrorAlert::new("Unable to load Integrity Modpack API".into(), error))
                .into_any_element();
        }

        let Some(catalog) = &self.catalog else {
            return v_flex()
                .p_4()
                .gap_4()
                .child(Skeleton::new().w_full().h_32().rounded_lg())
                .child(Skeleton::new().w_full().h_32().rounded_lg())
                .into_any_element();
        };

        v_flex()
            .p_4()
            .gap_6()
            .child(self.render_section("Featured Modpacks", &catalog.featured, true, window, cx))
            .child(self.render_section("All Modpacks", &catalog.modpacks, false, window, cx))
            .into_any_element()
    }
}
