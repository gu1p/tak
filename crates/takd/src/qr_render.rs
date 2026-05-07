use anyhow::Result;
use qrcode::QrCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tak_proto::{decode_tor_invite, encode_tor_invite_words};
use tui_qrcode::{Colors, QrCodeWidget};

use crate::tor_secret_warning;
use crate::word_table::numbered_words_text;

mod layout;

use layout::{buffer_to_plain_text, wrapped_text_height};

const MIN_VIEW_WIDTH: u16 = 84;
const TITLE_HEIGHT: u16 = 3;
const BLOCK_VERTICAL_CHROME: u16 = 4;
const BLOCK_HORIZONTAL_CHROME: u16 = 6;

pub(crate) fn render_onboarding_view(token: &str) -> Result<String> {
    let is_tor_invite = decode_tor_invite(token).is_ok();
    let word_phrase = encode_tor_invite_words(token).ok();
    let word_table = word_phrase
        .as_ref()
        .map(|phrase| numbered_words_text(phrase));
    let qr_code = QrCode::new(token.as_bytes())?;
    let command = format!("tak remote add '{token}'");
    let words_command = word_phrase
        .as_ref()
        .map(|phrase| format!("tak remote add --words {phrase}"));
    let warning = is_tor_invite.then(tor_secret_warning::text);
    let qr_title = if is_tor_invite {
        " Takd Invite "
    } else {
        " Takd Token "
    };
    let value_title = if is_tor_invite { " Invite " } else { " Token " };
    let qr_widget = QrCodeWidget::new(qr_code).colors(Colors::Normal);
    let qr_size = qr_widget.size(Rect::new(0, 0, 0, 0));
    let view_width = MIN_VIEW_WIDTH.max(qr_size.width + BLOCK_HORIZONTAL_CHROME);
    let text_width = view_width.saturating_sub(BLOCK_HORIZONTAL_CHROME).max(1);
    let command_height = wrapped_text_height(&command, text_width);
    let token_height = wrapped_text_height(token, text_width);
    let qr_block_height = qr_size.height + BLOCK_VERTICAL_CHROME;
    let command_block_height = command_height + BLOCK_VERTICAL_CHROME;
    let token_block_height = token_height + BLOCK_VERTICAL_CHROME;
    let words_block_height = word_table
        .as_ref()
        .map(|table| wrapped_text_height(table, text_width) + BLOCK_VERTICAL_CHROME)
        .unwrap_or(0);
    let warning_block_height = warning
        .as_ref()
        .map(|text| wrapped_text_height(text, text_width) + BLOCK_VERTICAL_CHROME)
        .unwrap_or(0);
    let view_height = TITLE_HEIGHT
        + warning_block_height
        + qr_block_height
        + command_block_height
        + token_block_height
        + words_block_height;

    let backend = TestBackend::new(view_width, view_height);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| {
        let mut constraints = vec![
            Constraint::Length(TITLE_HEIGHT),
            Constraint::Length(qr_block_height),
            Constraint::Length(command_block_height),
            Constraint::Length(token_block_height),
        ];
        if warning_block_height > 0 {
            constraints.insert(1, Constraint::Length(warning_block_height));
        }
        if words_block_height > 0 {
            constraints.push(Constraint::Length(words_block_height));
        }
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(frame.area());
        let title = Paragraph::new(Line::from("Scan this QR code"))
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(title, rows[0]);

        let mut row = 1;
        if let Some(warning) = &warning {
            let warning_block = Block::default().borders(Borders::ALL).title(" Secret ");
            let warning_area = warning_block.inner(rows[row]).inner(Margin {
                vertical: 1,
                horizontal: 2,
            });
            frame.render_widget(warning_block, rows[row]);
            frame.render_widget(
                Paragraph::new(Text::from(warning.clone())).wrap(Wrap { trim: false }),
                warning_area,
            );
            row += 1;
        }

        let qr_block = Block::default().borders(Borders::ALL).title(qr_title);
        let qr_area = qr_block.inner(rows[row]).inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        frame.render_widget(qr_block, rows[row]);
        frame.render_widget(qr_widget, qr_area);
        row += 1;

        let command_block = Block::default().borders(Borders::ALL).title(" Client ");
        let command_area = command_block.inner(rows[row]).inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        frame.render_widget(command_block, rows[row]);
        frame.render_widget(
            Paragraph::new(command.as_str()).wrap(Wrap { trim: false }),
            command_area,
        );
        row += 1;

        let token_block = Block::default().borders(Borders::ALL).title(value_title);
        let token_area = token_block.inner(rows[row]).inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        frame.render_widget(token_block, rows[row]);
        frame.render_widget(
            Paragraph::new(Text::from(token.to_string())).wrap(Wrap { trim: false }),
            token_area,
        );
        row += 1;

        if let Some(table) = &word_table {
            let words_block = Block::default().borders(Borders::ALL).title(" Words ");
            let words_area = words_block.inner(rows[row]).inner(Margin {
                vertical: 1,
                horizontal: 2,
            });
            frame.render_widget(words_block, rows[row]);
            frame.render_widget(
                Paragraph::new(Text::from(table.clone())).wrap(Wrap { trim: false }),
                words_area,
            );
        }
    })?;

    Ok(render_plain_text_view(
        buffer_to_plain_text(terminal.backend().buffer()),
        &command,
        token,
        words_command.as_deref(),
    ))
}

fn render_plain_text_view(
    view: String,
    command: &str,
    token: &str,
    words_command: Option<&str>,
) -> String {
    let mut output = format!("{view}\n\n{command}\n{token}\n");
    if let Some(words_command) = words_command {
        output.push_str(words_command);
        output.push('\n');
    }
    output
}
