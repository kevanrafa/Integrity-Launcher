use bridge::{message::MessageToBackend, modal_action::ModalAction};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, StyledExt, button::{Button, ButtonVariants}, h_flex, v_flex
};

use crate::{entity::DataEntities, pages::page::Page, ts};

pub struct IntegrityModpackPage {
    data: DataEntities,
}

impl IntegrityModpackPage {
    pub fn new(data: &DataEntities) -> Self {
        Self {
            data: data.clone(),
        }
    }
}

impl Page for IntegrityModpackPage {
    fn controls(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("refresh-integrity-modpacks")
            .info()
            .label(ts!("modpack.integrity.refresh"))
            .on_click({
                let backend_handle = self.data.backend_handle.clone();
                move |_, _, _| {
                    backend_handle.send(MessageToBackend::RequestIntegrityModpacks);
                }
            })
    }

    fn scrollable(&self, _: &App) -> bool {
        true
    }
}

impl Render for IntegrityModpackPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modpacks = self.data.use_integrity_modpacks(cx).cloned();

        let mut root = v_flex().gap_4().p_4();
        root = root.child(
            div()
                .text_xl()
                .font_bold()
                .child(ts!("modpack.integrity.title")),
        );

        if let Some(modpacks) = modpacks {
            if modpacks.is_empty() {
                root = root.child(div().text_color(cx.theme().muted_foreground).child(ts!("modpack.integrity.none")));
            } else {
                for (ix, modpack) in modpacks.iter().enumerate() {
                    let id = modpack.id.clone();
                    let name = modpack.name.clone();
                    let description = modpack.description.clone();
                    let version_text = format!("{} | {} {}", modpack.version, modpack.minecraft_version, modpack.loader);
                    root = root.child(
                        v_flex()
                            .gap_2()
                            .p_3()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(div().font_bold().child(name.to_string()))
                            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(version_text))
                            .when_some(description, |this, description| {
                                this.child(div().text_sm().child(description.to_string()))
                            })
                            .child(
                                h_flex().gap_2().child(
                                    Button::new(("install-integrity-modpack", ix))
                                        .success()
                                        .label(ts!("modpack.integrity.install"))
                                        .on_click({
                                            let backend_handle = self.data.backend_handle.clone();
                                            move |_, window, cx| {
                                                let modal_action = ModalAction::default();
                                                backend_handle.send(MessageToBackend::InstallIntegrityModpack {
                                                    id: id.clone(),
                                                    modal_action: modal_action.clone(),
                                                });
                                                crate::modals::generic::show_notification(
                                                    window,
                                                    cx,
                                                    ts!("modpack.integrity.install_error"),
                                                    modal_action,
                                                );
                                            }
                                        })
                                )
                            ),
                    );
                }
            }
        } else {
            root = root.child(div().text_color(cx.theme().muted_foreground).child(ts!("modpack.integrity.loading")));
        }

        root
    }
}
