use crate::ERROR_HAPPENED;
use cansi::v3::{categorise_text, CategorisedSlice};
use egui::text::LayoutJob;
use egui::{Color32, FontId, Stroke, TextFormat, Ui};
use std::borrow::Borrow;
use std::sync::atomic::Ordering;
use tracing::error;

pub fn report_error(err: impl Borrow<miette::Report>) {
    let err = err.borrow();
    let str = format_error(err, true);
    println!("{str}");
    error!("{str}");

    ERROR_HAPPENED.store(true, Ordering::Release);
}

pub fn format_error(err: &miette::Report, strip_ansi: bool) -> String {
    // let mut msg = String::new();
    //
    // for (pos, err) in err.chain().with_position() {
    //     if !matches!(pos, Position::First) {
    //         msg += "\n";
    //     }
    //
    //     match pos {
    //         Position::First | Position::Only => {
    //             msg += "X Error: ";
    //         }
    //         Position::Middle => {
    //             msg += "+ because: ";
    //         }
    //         Position::Last => {
    //             msg += "+ because: ";
    //         }
    //     }
    //     msg += &err.to_string();
    // }

    let str = format!("{err:?}");
    if strip_ansi {
        strip_ansi_escapes::strip_str(&str)
    } else {
        str
    }
}

pub fn render_ansi(ui: &mut Ui, ansi: &str, font: FontId) {
    let slices = categorise_text(ansi);
    let mut job = LayoutJob {
        break_on_newline: true,
        ..Default::default()
    };
    for CategorisedSlice {
        text,
        bg,
        fg,
        italic,
        underline,
        strikethrough,
        ..
    } in slices
    {
        let mut format = TextFormat::default();
        format.font_id = font.clone();

        if let Some(fg) = fg {
            format.color = ansi_to_color(fg);
        }
        if let Some(bg) = bg {
            format.background = ansi_to_color(bg);
        }
        if italic.is_some_and(|x| x) {
            format.italics = true;
        }
        if underline.is_some_and(|x| x) {
            format.underline = Stroke::new(1.0, format.color);
        }
        if strikethrough.is_some_and(|x| x) {
            format.strikethrough = Stroke::new(1.0, format.color);
        }

        job.append(text, 0.0, format);
    }

    let galley = ui.ctx().fonts(|f| f.layout_job(job));
    ui.label(galley);
}

fn ansi_to_color(ansi: cansi::Color) -> Color32 {
    // xterm colors, up to change at any point
    match ansi {
        cansi::Color::Black => Color32::from_rgb(0, 0, 0),
        cansi::Color::Red => Color32::from_rgb(205, 0, 0),
        cansi::Color::Green => Color32::from_rgb(0, 205, 0),
        cansi::Color::Yellow => Color32::from_rgb(205, 205, 0),
        cansi::Color::Blue => Color32::from_rgb(0, 0, 238),
        cansi::Color::Magenta => Color32::from_rgb(205, 0, 205),
        cansi::Color::Cyan => Color32::from_rgb(0, 205, 205),
        cansi::Color::White => Color32::from_rgb(229, 229, 229),
        cansi::Color::BrightBlack => Color32::from_rgb(127, 127, 127),
        cansi::Color::BrightRed => Color32::from_rgb(255, 0, 0),
        cansi::Color::BrightGreen => Color32::from_rgb(0, 255, 0),
        cansi::Color::BrightYellow => Color32::from_rgb(255, 255, 0),
        cansi::Color::BrightBlue => Color32::from_rgb(92, 92, 255),
        cansi::Color::BrightMagenta => Color32::from_rgb(255, 0, 255),
        cansi::Color::BrightCyan => Color32::from_rgb(0, 255, 255),
        cansi::Color::BrightWhite => Color32::from_rgb(255, 255, 255),
    }
}
