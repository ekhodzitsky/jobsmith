use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use crate::hh::models::strip_html;
use crate::tui::app::{App, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let main_area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(main_area);

    draw_table(f, app, chunks[0]);

    match app.mode {
        Mode::Filter => draw_filter_bar(f, app, chunks[1]),
        Mode::List | Mode::Detail => draw_status_bar(f, app, chunks[1]),
    }

    if app.mode == Mode::Detail {
        draw_detail_popup(f, app);
    }
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Title"),
        Cell::from("Company"),
        Cell::from("Salary"),
        Cell::from("Location"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(display_idx, &vacancy_idx)| {
            let vacancy = &app.vacancies[vacancy_idx];
            let is_selected = display_idx == app.selected;

            let salary = vacancy
                .salary
                .as_ref()
                .map(|s| {
                    let from = s
                        .from
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "?".to_string());
                    let to =
                        s.to.map(|v| format!("{v}"))
                            .unwrap_or_else(|| "?".to_string());
                    let currency = s.currency.as_deref().unwrap_or("RUR");
                    format!("{}-{} {}", from, to, currency)
                })
                .unwrap_or_else(|| "—".to_string());

            let location = vacancy
                .area
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "?".to_string());

            let cells = vec![
                Cell::from(vacancy.id.clone()),
                Cell::from(vacancy.name.clone()),
                Cell::from(vacancy.employer_name().to_string()),
                Cell::from(salary),
                Cell::from(location),
            ];

            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Length(18),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(Block::default().title("Vacancies").borders(Borders::ALL));

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help = format!(
        "{} vacancies | j/k or ↑/↓: navigate | Enter: details | a: apply | /: filter | q: quit",
        app.filtered.len()
    );
    let paragraph =
        Paragraph::new(help).block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(paragraph, area);
}

fn draw_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = format!("/{}", app.filter);
    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Filter"));
    f.render_widget(paragraph, area);
}

fn draw_detail_popup(f: &mut Frame, app: &App) {
    let area = f.area().inner(Margin {
        horizontal: 5,
        vertical: 2,
    });

    let block = Block::default()
        .title("Vacancy Details")
        .borders(Borders::ALL);

    let content = if app.detail_loading {
        Text::from("Loading...")
    } else if let Some(ref detail) = app.detail {
        let base = &detail.base;
        let salary = base
            .salary
            .as_ref()
            .map(|s| {
                let from = s
                    .from
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "?".to_string());
                let to =
                    s.to.map(|v| format!("{v}"))
                        .unwrap_or_else(|| "?".to_string());
                let currency = s.currency.as_deref().unwrap_or("RUR");
                format!("{}-{} {}", from, to, currency)
            })
            .unwrap_or_else(|| "—".to_string());

        let location = base
            .area
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "?".to_string());
        let description = base
            .description
            .as_deref()
            .map(strip_html)
            .unwrap_or_default();

        let text = format!(
            "ID: {}\nTitle: {}\nCompany: {}\nSalary: {}\nLocation: {}\n\n{}",
            base.id,
            base.name,
            base.employer_name(),
            salary,
            location,
            description
        );
        Text::from(text)
    } else {
        Text::from("No details available")
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
