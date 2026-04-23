use std::time::{Duration, Instant};

use gpui::{prelude::*, *};
use gpui_component::{h_flex, v_flex};
use rand::{seq::SliceRandom, Rng};

const MIN_LINE_DELAY_MS: u64 = 1;
const MAX_LINE_DELAY_MS: u64 = 1000;
const MIN_DURATION_MS: u64 = 2200;
const FADE_OUT_DURATION_MS: u64 = 1000;

const COMMON_LINES: &[&str] = &[
    "[ The Integrity Council :: Minecraft Launcher ]",
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
    "ERROR : You are not the Sudoer of this mainframe",
    "Cancel Injecting root consensue",
    "Cancel Attuning forbidden telemetry",
];

const DIALOGUE_LINES: &[&str] = &[
    "Navekka> Someone just launched Integrity again.",
    "Kriss> Good. Let them see what stable looks like.",
    "Navekka> Think they know how deep this stack goes?",
    "Kriss> Not yet. Keep the council quiet and the launcher fast.",
    "Navekka> They are watching the boot log.",
    "Kriss> Then give them a clean start worth trusting.",
];

pub struct BootSequence {
    started_at: Instant,
    lines: Vec<SharedString>,
    line_delays: Vec<u64>,
    rare_root_mode: bool,
    skipped: bool,
    fading_out: bool,
    fade_start: Option<Instant>,
}

impl BootSequence {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let rare_root_mode = rng.gen_ratio(1, 14);

        let mut lines = vec![COMMON_LINES[0].into(), COMMON_LINES[1].into(), COMMON_LINES[2].into()];

        let mut line_delays = vec![
            0,
            rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS),
            rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS),
        ];

        let mut random_pool = COMMON_LINES[3..].to_vec();
        random_pool.shuffle(&mut rng);
        for line in random_pool.into_iter().take(4) {
            lines.push(line.into());
            line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));
        }

        let mut dialogue_pool = DIALOGUE_LINES.to_vec();
        dialogue_pool.shuffle(&mut rng);
        for line in dialogue_pool.into_iter().take(3) {
            lines.push(line.into());
            line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));
        }

        if rare_root_mode {
            let mut rare_pool = RARE_LINES.to_vec();
            rare_pool.shuffle(&mut rng);
            for line in rare_pool.into_iter().take(2) {
                lines.push(line.into());
                line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));
            }
        }

        lines.push(if rare_root_mode {
            "Status: ROOT ACCESS GRANTED".into()
        } else {
            "Status: STABLE".into()
        });
        line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));

        lines.push("[ The Integrity Council : Minecraft Launcher ]".into());
        line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));

        lines.push(if rare_root_mode {
            "Welcome back, Architect.".into()
        } else {
            "Welcome. Systems are ready.".into()
        });
        line_delays.push(rng.gen_range(MIN_LINE_DELAY_MS..=MAX_LINE_DELAY_MS));

        Self {
            started_at: Instant::now(),
            lines,
            line_delays,
            rare_root_mode,
            skipped: false,
            fading_out: false,
            fade_start: None,
        }
    }

    pub fn is_active(&self) -> bool {
        if self.fading_out {
            return self
                .fade_start
                .map_or(false, |start| start.elapsed() < Duration::from_millis(FADE_OUT_DURATION_MS));
        }
        true
    }

    fn begin_fade_out(&mut self) {
        if !self.fading_out {
            self.fading_out = true;
            self.fade_start = Some(Instant::now());
        }
    }

    pub fn skip(&mut self) {
        self.skipped = true;
        self.begin_fade_out();
    }
}

impl Render for BootSequence {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_delay: u64 = self.line_delays.iter().sum();
        let elapsed_ms = self.started_at.elapsed().as_millis() as u64;
        let all_lines_visible_at = total_delay;
        let fade_start_at = MIN_DURATION_MS.max(all_lines_visible_at);

        if !self.fading_out && elapsed_ms >= fade_start_at && !self.skipped {
            self.begin_fade_out();
        }

        if self.fading_out {
            window.request_animation_frame();
        } else {
            window.request_animation_frame();
        }

        let mut cumulative_delay = 0;
        let mut visible_lines = 0;
        for delay in &self.line_delays {
            cumulative_delay += *delay;
            if elapsed_ms >= cumulative_delay {
                visible_lines += 1;
            } else {
                break;
            }
        }
        let visible_lines = visible_lines.clamp(1, self.lines.len());

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

        let opacity = if self.fading_out {
            let fade_elapsed = self.fade_start.unwrap().elapsed().as_millis() as f32;
            (1.0 - fade_elapsed / FADE_OUT_DURATION_MS as f32).max(0.0)
        } else {
            1.0
        };

        let mode_text = if self.rare_root_mode {
            "mode=root"
        } else {
            "mode=normal"
        };

        div()
            .id("boot-sequence")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(hsla(215.0 / 360.0, 0.38, 0.06, opacity))
            .text_color(gpui::white())
            .opacity(opacity)
            .child(
                v_flex().size_full().justify_center().px_8().py_6().child(
                    v_flex()
                        .max_w(px(900.0))
                        .mx_auto()
                        .gap_2()
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .child(div().font_family("Roboto Mono").text_color(accent).text_xs().child(mode_text))
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
                                .h(px(360.0))
                                .p_5()
                                .overflow_hidden()
                                .child(
                                    v_flex()
                                        .size_full()
                                        .justify_end()
                                        .child(v_flex().gap_1p5().children(lines)),
                                ),
                        ),
                ),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.skip();
                cx.notify();
            }))
    }
}
