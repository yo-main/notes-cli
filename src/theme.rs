use ratatui::style::{Color, Style};

const PURPLE: Color = Color::Rgb(0xbd, 0x93, 0xf9);
const PINK: Color = Color::Rgb(0xff, 0x79, 0xc6);
const CYAN: Color = Color::Rgb(0x8b, 0xe9, 0xfd);
const GREEN: Color = Color::Rgb(0x50, 0xfa, 0x7b);
const YELLOW: Color = Color::Rgb(0xf1, 0xfa, 0x8c);
const COMMENT: Color = Color::Rgb(0x62, 0x72, 0xa4);
pub const HIGHLIGHT_BG: Color = Color::Indexed(236);

#[derive(Clone)]
pub struct Dracula;

impl tui_markdown::StyleSheet for Dracula {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new().fg(PURPLE).bold().underlined(),
            _ => Style::new().fg(PURPLE).bold(),
        }
    }

    fn heading_meta(&self) -> Style {
        Style::new().fg(COMMENT)
    }

    fn code(&self) -> Style {
        Style::new().fg(GREEN)
    }

    fn link(&self) -> Style {
        Style::new().fg(CYAN).underlined()
    }

    fn blockquote(&self) -> Style {
        Style::new().fg(YELLOW).italic()
    }

    fn list_marker(&self) -> Style {
        Style::new().fg(PINK)
    }
}
