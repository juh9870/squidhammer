use crate::error::{format_error, render_ansi};
use dbe2::diagnostic::prelude::{Diagnostic, DiagnosticLevel};
use egui::{CollapsingHeader, Margin, RichText, Ui};
use inline_tweak::tweak;

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

    frame.show(ui, |ui| {
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
                            render_ansi(ui, rest.trim());
                        });
                });
        } else {
            ui.colored_label(color, strip_ansi_escapes::strip_str(text));
        }
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
