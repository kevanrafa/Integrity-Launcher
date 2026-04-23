use std::time::{Duration, Instant};

use gpui::{prelude::*, *};
use gpui_component::{h_flex, v_flex};
use rand::{Rng, seq::SliceRandom};

const LINE_DELAY_MS: u64 = 95;
const MIN_DURATION_MS: u64 = 2200;

const COMMON_LINES: &[&str] = &[
    "[ AXIX NULL :: BOOT SEQUENCE ]",
    "Loading Integrity Core",
    "Verifying system integrity",
    "Scanning launch modules",
    "Mounting runtime partitions",
    "Synchronizing council directives",
    "Checking Java chains",
    "Priming modpack registry",
];

const RARE_LINES: &[&str] = &[
    "[ ROOT OVERRIDE :: ELEVATED CHANNEL ]",
    "Bypassing ceremonial locks",
    "Injecting root consensus",
    "Attuning forbidden telemetry",
];

pub struct BootSequence {
    started_at: Instant,
    lines: Vec<SharedString>,
    rare_root_mode: bool,
    skipped: bool,
}

impl BootSequence {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let rare_root_mode = rng.gen_ratio(1, 14);

        let mut lines = vec![
            COMMON_LINES[0].into(),
            COMMON_LINES[1].into(),
            COMMON_LINES[2].into(),
        ];

        let mut random_pool = COMMON_LINES[3..].to_vec();
        random_pool.shuffle(&mut rng);
        for line in random_pool.into_iter().take(4) {
            lines.push(line.into());
        }

        if rare_root_mode {
            let mut rare_pool = RARE_LINES.to_vec();
            rare_pool.shuffle(&mut rng);
            for line in rare_pool.into_iter().take(2) {
                lines.push(line.into());
            }
        }

        lines.push(if rare_root_mode {
            "Status: ROOT ACCESS GRANTED".into()
        } else {
            "Status: STABLE".into()
        });
        lines.push("[ The Integrity Council : Minecraft Launcher ]".into());
        lines.push(if rare_root_mode {
            "Welcome back, Architect.".into()
        } else {
            "Welcome. Systems are ready.".into()
        });

        Self {
            started_at: Instant::now(),
            lines,
            rare_root_mode,
            skipped: false,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.skipped && self.started_at.elapsed() < Duration::from_millis(MIN_DURATION_MS)
    }

    pub fn skip(&mut self) {
        self.skipped = true;
    }
}

impl Render for BootSequence {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.skipped && self.started_at.elapsed() < Duration::from_millis(MIN_DURATION_MS) {
            window.request_animation_frame();
        }

        let elapsed_ms = self.started_at.elapsed().as_millis() as u64;
        let visible_lines = ((elapsed_ms / LINE_DELAY_MS) as usize + 1).min(self.lines.len());

        let mut lines = Vec::with_capacity(visible_lines);
        for (index, line) in self.lines.iter().take(visible_lines).enumerate() {
            let highlight = index + 3 >= self.lines.len();
            let color = if highlight {
                if self.rare_root_mode {
                    hsla(28.0 / 360.0, 0.95, 0.63, 1.0)
                } else {
                    hsla(140.0 / 360.0, 0.82, 0.62, 1.0)
                }
            } else {
                hsla(145.0 / 360.0, 0.45, 0.78, 1.0)
            };

            lines.push(
                div()
                    .font_family("Roboto Mono")
                    .text_color(color)
                    .text_sm()
                    .line_height(rems(1.25))
                    .child(line.clone()),
            );
        }

        let accent = if self.rare_root_mode {
            hsla(28.0 / 360.0, 0.95, 0.55, 1.0)
        } else {
            hsla(140.0 / 360.0, 0.85, 0.5, 1.0)
        };

        div()
            .id("boot-sequence")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(hsla(215.0 / 360.0, 0.38, 0.06, 0.96))
            .text_color(gpui::white())
            .child(
                v_flex()
                    .size_full()
                    .justify_center()
                    .px_8()
                    .py_6()
                    .child(
                        v_flex()
                            .max_w(px(900.0))
                            .mx_auto()
                            .gap_2()
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_family("Roboto Mono")
                                            .text_color(accent)
                                            .text_xs()
                                            .child(if self.rare_root_mode { "mode=root" } else { "mode=normal" }),
                                    )
                                    .child(
                                        div()
                                            .font_family("Roboto Mono")
                                            .text_color(accent.opacity(0.75))
                                            .text_xs()
                                            .child("click anywhere to skip"),
                                    ),
                            )
                            .child(
                                div()
                                    .border_1()
                                    .border_color(accent.opacity(0.5))
                                    .rounded(px(10.0))
                                    .bg(hsla(215.0 / 360.0, 0.34, 0.09, 0.92))
                                    .p_5()
                                    .child(v_flex().gap_1p5().children(lines)),
                            ),
                    ),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.skip();
                cx.notify();
            }))
    }
}
