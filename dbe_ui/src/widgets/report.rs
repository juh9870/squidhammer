use crate::error::{format_error, render_ansi, report_error};
use dbe_backend::diagnostic::prelude::{Diagnostic, DiagnosticLevel};
use egui::{CollapsingHeader, Margin, RichText, TextStyle, Ui};
use egui_hooks::UseHookExt;
use inline_tweak::tweak;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct DiagnosticState {
    msg: String,
    err: Arc<Diagnostic>,
    lifetime: usize,
}

pub fn reporting_error_widget(ui: &mut Ui, error: Option<miette::Report>) {
    reporting_diagnostic_widget(
        ui,
        error.map(|info| Diagnostic {
            info,
            level: DiagnosticLevel::Error,
        }),
    );
}
pub fn reporting_diagnostic_widget(ui: &mut Ui, diagnostic: Option<Diagnostic>) {
    ui.push_id("diagnostic", |ui| {
        let mut var_last_state = ui.use_state(|| None::<DiagnosticState>, ()).into_var();

        if let Some(last_state) = &mut *var_last_state {
            if let Some(diagnostic) = diagnostic {
                let new_msg = format_error(&diagnostic.info, false);
                if new_msg != last_state.msg {
                    last_state.msg = new_msg;
                    last_state.err = Arc::new(diagnostic);
                    last_state.lifetime = 0;
                } else {
                    last_state.lifetime += 1;
                }
                if last_state.lifetime >= 1 {
                    diagnostic_widget(ui, &last_state.err);
                }
            } else {
                if last_state.lifetime < 60 && last_state.err.level >= DiagnosticLevel::Error {
                    // Message was short-lived, report it
                    report_error(&last_state.err.info);
                }
                *var_last_state = None;
            }
        } else if let Some(diagnostic) = diagnostic {
            let new_msg = format_error(&diagnostic.info, false);
            *var_last_state = Some(DiagnosticState {
                msg: new_msg,
                err: Arc::new(diagnostic),
                lifetime: 0,
            });
        }
    });
}

pub fn diagnostic_widget(ui: &mut Ui, diagnostic: &Diagnostic) {
    let style = ui.style();

    let mut fill = style.visuals.widgets.noninteractive.bg_fill;
    let mut stroke = style.visuals.widgets.noninteractive.bg_stroke;
    let mut color = style.visuals.widgets.noninteractive.fg_stroke.color;

    match diagnostic.level {
        DiagnosticLevel::Trace => {
            fill = style.visuals.widgets.inactive.bg_fill.gamma_multiply(0.5);
            stroke = style.visuals.widgets.inactive.bg_stroke;
            stroke.color = stroke.color.gamma_multiply(0.5);
            color = style
                .visuals
                .widgets
                .inactive
                .fg_stroke
                .color
                .gamma_multiply(0.5);
        }
        DiagnosticLevel::Debug => {
            fill = style.visuals.widgets.inactive.bg_fill;
            stroke = style.visuals.widgets.inactive.bg_stroke;
            color = style.visuals.widgets.inactive.fg_stroke.color;
        }
        DiagnosticLevel::Info => {}
        DiagnosticLevel::Warning => {
            fill = style.visuals.warn_fg_color.gamma_multiply(0.25);
            stroke.color = style.visuals.warn_fg_color;
            color = style.visuals.warn_fg_color;
        }
        DiagnosticLevel::Error => {
            fill = style.visuals.error_fg_color.gamma_multiply(0.25);
            stroke.color = style.visuals.error_fg_color;
            color = style.visuals.error_fg_color;
        }
    }

    let frame = egui::Frame {
        inner_margin: Margin::same(2.0),
        fill,
        stroke,
        ..Default::default()
    };

    let res = frame.show(ui, |ui| {
        let text = format_error(&diagnostic.info, false);
        let text = text.trim();
        let idx = text.find('\n');
        if let Some(idx) = idx {
            let (title, rest) = text.split_at(idx);
            CollapsingHeader::new(RichText::new(strip_ansi_escapes::strip_str(title)).color(color))
                .show_unindented(ui, |ui| {
                    egui::Frame::none()
                        .outer_margin(Margin {
                            top: tweak!(0.0),
                            bottom: tweak!(2.0),
                            left: tweak!(0.0),
                            right: tweak!(8.0),
                        })
                        .show(ui, |ui| {
                            render_ansi(ui, rest.trim(), TextStyle::Body.resolve(ui.style()));
                        });
                });
        } else {
            ui.colored_label(color, strip_ansi_escapes::strip_str(text));
        }
    });

    res.response.on_hover_ui(|ui| {
        ui.label(format_error(&diagnostic.info, true).trim());
    });
}

pub fn diagnostics_column<'a>(ui: &mut Ui, diagnostics: impl IntoIterator<Item = &'a Diagnostic>) {
    let mut d = diagnostics.into_iter().peekable();

    if d.peek().is_none() {
        return;
    }

    ui.vertical(|ui| {
        for diagnostic in d {
            diagnostic_widget(ui, diagnostic);
        }
    });
}
