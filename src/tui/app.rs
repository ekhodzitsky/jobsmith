use std::io;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::error::{JobsmithError, Result};
use crate::hh::client::HhClient;
use crate::hh::models::{Vacancy, VacancyDetail};

pub enum Action {
    Quit,
    Apply(String),
}

#[derive(PartialEq)]
pub(crate) enum Mode {
    List,
    Filter,
    Detail,
}

pub struct App {
    pub(crate) vacancies: Vec<Vacancy>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) filter: String,
    pub(crate) mode: Mode,
    pub(crate) detail: Option<VacancyDetail>,
    pub(crate) detail_loading: bool,
    client: HhClient,
}

impl App {
    pub fn new(vacancies: Vec<Vacancy>) -> Result<Self> {
        let filtered: Vec<usize> = (0..vacancies.len()).collect();
        Ok(Self {
            vacancies,
            filtered,
            selected: 0,
            filter: String::new(),
            mode: Mode::List,
            detail: None,
            detail_loading: false,
            client: HhClient::new()?,
        })
    }

    pub async fn run(&mut self) -> Result<Action> {
        let term = setup_terminal()?;
        let mut terminal = TerminalGuard::new(term);
        let mut reader = EventStream::new();
        self.event_loop(terminal.inner(), &mut reader).await
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        reader: &mut EventStream,
    ) -> Result<Action> {
        let (detail_tx, mut detail_rx) = mpsc::unbounded_channel::<Result<VacancyDetail>>();

        loop {
            terminal
                .draw(|f| super::ui::draw(f, self))
                .map_err(JobsmithError::Io)?;

            tokio::select! {
                biased;
                Some(event) = reader.next() => {
                    let event = event.map_err(JobsmithError::Io)?;
                    if let Some(action) = self.handle_event(event, &detail_tx)? {
                        return Ok(action);
                    }
                }
                Some(detail_result) = detail_rx.recv() => {
                    match detail_result {
                        Ok(detail) => self.detail = Some(detail),
                        Err(e) => {
                            tracing::warn!(error = %e, "detail load failed");
                            self.detail = None;
                        }
                    }
                    self.detail_loading = false;
                }
                else => {
                    return Ok(Action::Quit);
                }
            }
        }
    }

    fn handle_event(
        &mut self,
        event: Event,
        detail_tx: &mpsc::UnboundedSender<Result<VacancyDetail>>,
    ) -> Result<Option<Action>> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match self.mode {
                    Mode::List => Ok(self.handle_list_key(key.code, detail_tx)),
                    Mode::Filter => Ok(self.handle_filter_key(key.code)),
                    Mode::Detail => Ok(self.handle_detail_key(key.code)),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn handle_list_key(
        &mut self,
        code: KeyCode,
        detail_tx: &mpsc::UnboundedSender<Result<VacancyDetail>>,
    ) -> Option<Action> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered.is_empty() {
                    self.selected =
                        (self.selected + 1).min(self.filtered.len().saturating_sub(1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Filter;
                None
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    let id = self.vacancies[idx].id.clone();
                    let client = self.client.clone();
                    let tx = detail_tx.clone();
                    self.detail_loading = true;
                    tokio::spawn(async move {
                        let result = client.get_vacancy(&id).await;
                        if let Err(ref e) = result {
                            tracing::warn!(vacancy_id = %id, error = %e, "failed to load vacancy detail");
                        }
                        if let Err(e) = tx.send(result) {
                            tracing::warn!(error = %e, "detail channel closed");
                        }
                    });
                    self.mode = Mode::Detail;
                }
                None
            }
            KeyCode::Char('a') => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    Some(Action::Apply(self.vacancies[idx].id.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_filter_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                self.filter.clear();
                self.apply_filter();
                self.mode = Mode::List;
                None
            }
            KeyCode::Enter => {
                self.apply_filter();
                self.mode = Mode::List;
                None
            }
            KeyCode::Backspace => {
                self.filter.pop();
                None
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                None
            }
            _ => None,
        }
    }

    fn handle_detail_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::List;
                self.detail = None;
                self.detail_loading = false;
                None
            }
            KeyCode::Char('a') => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    Some(Action::Apply(self.vacancies[idx].id.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn apply_filter(&mut self) {
        let query = self.filter.to_lowercase();
        self.filtered = self
            .vacancies
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                let name = v.name.to_lowercase();
                let employer = v.employer_name().to_lowercase();
                name.contains(&query) || employer.contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new(terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Self {
        Self { terminal }
    }

    fn inner(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal(&mut self.terminal);
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().map_err(JobsmithError::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(JobsmithError::Io)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(JobsmithError::Io)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().map_err(JobsmithError::Io)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(JobsmithError::Io)?;
    terminal.show_cursor().map_err(JobsmithError::Io)
}
