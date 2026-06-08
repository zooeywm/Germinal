use std::{cell::RefCell, collections::HashMap, rc::Rc};

use alacritty_terminal::{
    event::{Event, EventListener, WindowSize},
    grid::Dimensions,
    term::{Config as TermConfig, Term, cell::Flags, color::Colors, point_to_viewport},
    vte::ansi::{Color, NamedColor, Processor, Rgb},
};
use germinal_ports::{
    pty::{GShellId, PtyError, PtyResult, PtySize},
    terminal::{
        TerminalCell, TerminalColor, TerminalEnginePort, TerminalScreen, TerminalSize,
        TerminalUpdate,
    },
};

const DEFAULT_BACKGROUND: TerminalColor = TerminalColor {
    r: 16,
    g: 20,
    b: 28,
};
const DEFAULT_FOREGROUND: TerminalColor = TerminalColor {
    r: 220,
    g: 226,
    b: 235,
};

pub struct AlacrittyTerminalEngine {
    sessions: HashMap<GShellId, AlacrittySession>,
}

struct AlacrittySession {
    term: Term<SessionEventProxy>,
    parser: Processor,
    size: TerminalSize,
    events: Rc<RefCell<Vec<Event>>>,
    screen: TerminalScreen,
}

#[derive(Clone)]
struct SessionEventProxy {
    events: Rc<RefCell<Vec<Event>>>,
}

impl SessionEventProxy {
    fn new(events: Rc<RefCell<Vec<Event>>>) -> Self {
        Self { events }
    }
}

impl EventListener for SessionEventProxy {
    fn send_event(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }
}

struct TermDimensions {
    cols: usize,
    rows: usize,
}

impl TermDimensions {
    fn new(size: TerminalSize) -> Self {
        Self {
            cols: usize::from(size.cols.max(2)),
            rows: usize::from(size.rows.max(1)),
        }
    }
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

impl AlacrittyTerminalEngine {
    pub fn new(initial_id: GShellId, initial_size: TerminalSize) -> PtyResult<Self> {
        let mut engine = Self {
            sessions: HashMap::new(),
        };
        engine.create_terminal(initial_id, initial_size)?;
        Ok(engine)
    }

    fn session_mut(&mut self, id: GShellId) -> PtyResult<&mut AlacrittySession> {
        self.sessions.get_mut(&id).ok_or(PtyError::SessionNotFound)
    }

    fn session(&self, id: GShellId) -> PtyResult<&AlacrittySession> {
        self.sessions.get(&id).ok_or(PtyError::SessionNotFound)
    }
}

impl TerminalEnginePort for AlacrittyTerminalEngine {
    fn create_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<()> {
        if self.sessions.contains_key(&id) {
            return Err(PtyError::SessionAlreadyExists);
        }

        let events = Rc::new(RefCell::new(Vec::new()));
        let proxy = SessionEventProxy::new(events.clone());
        let dimensions = TermDimensions::new(size);
        let term = Term::new(TermConfig::default(), &dimensions, proxy);
        let screen = TerminalScreen::new(PtySize {
            cols: size.cols,
            rows: size.rows,
        });

        self.sessions.insert(
            id,
            AlacrittySession {
                term,
                parser: Processor::new(),
                size,
                events,
                screen,
            },
        );

        Ok(())
    }

    fn update_terminal_output(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<TerminalUpdate> {
        let session = self.session_mut(id)?;
        session.events.borrow_mut().clear();
        session.parser.advance(&mut session.term, bytes);

        let responses = collect_responses(session);
        session.screen = snapshot_screen(session);

        Ok(TerminalUpdate { responses })
    }

    fn resize_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<TerminalUpdate> {
        let session = self.session_mut(id)?;
        session.size = size;
        session.events.borrow_mut().clear();
        session.term.resize(TermDimensions::new(size));

        let responses = collect_responses(session);
        session.screen = snapshot_screen(session);

        Ok(TerminalUpdate { responses })
    }

    fn remove_terminal(&mut self, id: GShellId) -> PtyResult<()> {
        self.sessions
            .remove(&id)
            .map(|_| ())
            .ok_or(PtyError::SessionNotFound)
    }

    fn terminal_screen(&self, id: GShellId) -> PtyResult<&TerminalScreen> {
        Ok(&self.session(id)?.screen)
    }
}

fn collect_responses(session: &AlacrittySession) -> Vec<Vec<u8>> {
    let events = std::mem::take(&mut *session.events.borrow_mut());
    let mut responses = Vec::new();

    for event in events {
        match event {
            Event::PtyWrite(text) => responses.push(text.into_bytes()),
            Event::TextAreaSizeRequest(formatter) => responses.push(
                formatter(WindowSize {
                    num_lines: session.size.rows,
                    num_cols: session.size.cols,
                    cell_width: session.size.cell_width,
                    cell_height: session.size.cell_height,
                })
                .into_bytes(),
            ),
            Event::ColorRequest(index, formatter) => {
                let rgb = resolve_color_request(&session.term, index);
                responses.push(formatter(rgb).into_bytes());
            }
            _ => {}
        }
    }

    responses
}

fn snapshot_screen(session: &AlacrittySession) -> TerminalScreen {
    let cols = session.term.columns() as u16;
    let rows = session.term.screen_lines() as u16;
    let mut screen = TerminalScreen::new(PtySize { cols, rows });
    let renderable = session.term.renderable_content();
    let mut cells = vec![TerminalCell::new(' ', None, None); usize::from(cols) * usize::from(rows)];

    for indexed in renderable.display_iter {
        let Some(point) = point_to_viewport(renderable.display_offset, indexed.point) else {
            continue;
        };

        if point.line >= usize::from(rows) || point.column.0 >= usize::from(cols) {
            continue;
        }

        let cell = indexed.cell;
        let mut foreground = resolve_cell_color(cell.fg, renderable.colors);
        let mut background = resolve_cell_color(cell.bg, renderable.colors);

        if cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut foreground, &mut background);
        }

        let terminal_cell = if cell
            .flags
            .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
        {
            TerminalCell::continuation(foreground, background)
        } else {
            let ch = if cell.flags.contains(Flags::HIDDEN) {
                ' '
            } else {
                cell.c
            };

            TerminalCell::new(ch, foreground, background)
        };

        let index = point.line * usize::from(cols) + point.column.0;
        cells[index] = terminal_cell;
    }

    screen.replace(PtySize { cols, rows }, cells);
    screen
}

fn resolve_color_request<T>(term: &Term<T>, index: usize) -> Rgb
where
    T: EventListener,
{
    let colors = term.renderable_content().colors;
    let fallback = fallback_named_color(index);

    colors[index].unwrap_or(Rgb {
        r: fallback.r,
        g: fallback.g,
        b: fallback.b,
    })
}

fn resolve_cell_color(color: Color, colors: &Colors) -> Option<TerminalColor> {
    Some(match color {
        Color::Spec(rgb) => TerminalColor {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        },
        Color::Indexed(index) => {
            let rgb = colors[usize::from(index)]
                .unwrap_or_else(|| terminal_color_to_rgb(fallback_named_color(usize::from(index))));
            TerminalColor {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
            }
        }
        Color::Named(named) => {
            let rgb = colors[named]
                .unwrap_or_else(|| terminal_color_to_rgb(fallback_named_color(named as usize)));
            TerminalColor {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
            }
        }
    })
}

fn fallback_named_color(index: usize) -> TerminalColor {
    match index {
        0 => TerminalColor { r: 0, g: 0, b: 0 },
        1 => TerminalColor {
            r: 205,
            g: 49,
            b: 49,
        },
        2 => TerminalColor {
            r: 13,
            g: 188,
            b: 121,
        },
        3 => TerminalColor {
            r: 229,
            g: 229,
            b: 16,
        },
        4 => TerminalColor {
            r: 36,
            g: 114,
            b: 200,
        },
        5 => TerminalColor {
            r: 188,
            g: 63,
            b: 188,
        },
        6 => TerminalColor {
            r: 17,
            g: 168,
            b: 205,
        },
        7 => TerminalColor {
            r: 229,
            g: 229,
            b: 229,
        },
        8 => TerminalColor {
            r: 102,
            g: 102,
            b: 102,
        },
        9 => TerminalColor {
            r: 241,
            g: 76,
            b: 76,
        },
        10 => TerminalColor {
            r: 35,
            g: 209,
            b: 139,
        },
        11 => TerminalColor {
            r: 245,
            g: 245,
            b: 67,
        },
        12 => TerminalColor {
            r: 59,
            g: 142,
            b: 234,
        },
        13 => TerminalColor {
            r: 214,
            g: 112,
            b: 214,
        },
        14 => TerminalColor {
            r: 41,
            g: 184,
            b: 219,
        },
        15 => TerminalColor {
            r: 255,
            g: 255,
            b: 255,
        },
        x if x == NamedColor::Foreground as usize => DEFAULT_FOREGROUND,
        x if x == NamedColor::Background as usize => DEFAULT_BACKGROUND,
        x if x == NamedColor::BrightForeground as usize => TerminalColor {
            r: 255,
            g: 255,
            b: 255,
        },
        x if x == NamedColor::DimForeground as usize => TerminalColor {
            r: 128,
            g: 132,
            b: 138,
        },
        _ => DEFAULT_FOREGROUND,
    }
}

fn terminal_color_to_rgb(color: TerminalColor) -> Rgb {
    Rgb {
        r: color.r,
        g: color.g,
        b: color.b,
    }
}
