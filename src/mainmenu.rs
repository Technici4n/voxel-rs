use anyhow::Result;
use log::info;

use crate::{
    fps::FpsCounter,
    input::InputState,
    settings::Settings,
    singleplayer::SinglePlayer,
    ui::{
        renderer::{self, UiRenderer},
        widgets,
    },
    window::{Gfx, State, StateTransition, WindowData, WindowFlags},
};

/// State of the main menu
pub struct MainMenu {
    fps_counter: FpsCounter,
    ui: self::Ui,
    ui_renderer: UiRenderer,
}

impl MainMenu {
    pub fn new(_settings: &mut Settings, gfx: &mut Gfx) -> Result<Box<dyn State>> {
        info!("Creating main menu");

        Ok(Box::new(Self {
            fps_counter: FpsCounter::new(),
            ui: self::Ui::new(),
            ui_renderer: UiRenderer::new(gfx)?,
        }))
    }
}

impl State for MainMenu {
    fn update(&mut self, _: &mut Settings, _: &InputState, _: &WindowData, flags: &mut WindowFlags, _: f64, _: &mut Gfx) -> Result<StateTransition> {
        flags.hide_and_center_cursor = false;

        if self.ui.should_exit {
            Ok(StateTransition::CloseWindow)
        } else if self.ui.should_start_single_player {
            Ok(StateTransition::ReplaceCurrent(Box::new(SinglePlayer::new)))
        } else {
            Ok(StateTransition::KeepCurrent)
        }
    }

    fn render(&mut self, _: &Settings, gfx: &mut Gfx, data: &WindowData) -> Result<StateTransition> {
        use gfx::traits::Device;

        self.fps_counter.add_frame();

        // Clear buffers
        gfx.encoder
            .clear(&gfx.color_buffer, crate::window::CLEAR_COLOR);
        gfx.encoder
            .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Rebuild ui
        self.ui.rebuild(self.fps_counter.fps(), data);
        self.ui_renderer.render(gfx, data, &self.ui.ui)?;
        // Flush and swap buffers
        gfx.encoder.flush(&mut gfx.device);
        gfx.context.swap_buffers()?;
        gfx.device.cleanup();

        Ok(StateTransition::KeepCurrent)
    }

    fn handle_mouse_motion(&mut self, _: &Settings, _: (f64, f64)) {}

    fn handle_cursor_movement(&mut self, logical_position: glutin::dpi::LogicalPosition) {
        self.ui.cursor_moved(logical_position);
    }

    fn handle_mouse_state_changes(&mut self, changes: Vec<(glutin::MouseButton, glutin::ElementState)>) {
        self.ui.handle_mouse_state_changes(changes);
    }

    fn handle_key_state_changes(&mut self, _: Vec<(u32, glutin::ElementState)>) {}
}

#[derive(Debug, Clone, Copy)]
enum UiMessage {
    StartSinglePlayer,
    ExitGame,
}

struct Ui {
    pub(self) ui: quint::Ui<renderer::PrimitiveBuffer, UiMessage>,
    messages: Vec<UiMessage>,
    pub(self) should_exit: bool,
    pub(self) should_start_single_player: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            ui: quint::Ui::new(),
            messages: Vec::new(),
            should_exit: false,
            should_start_single_player: false,
        }
    }

    pub fn cursor_moved(&mut self, p: glutin::dpi::LogicalPosition) {
        self.ui.set_cursor_position(quint::Position {
            x: p.x as f32,
            y: p.y as f32,
        });
    }

    pub fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(glutin::MouseButton, glutin::ElementState)>,
    ) {
        let changes = changes
            .into_iter()
            .map(|(button, state)| quint::Event::MouseInput {
                button: crate::ui::quint_mouse_button(button),
                state: crate::ui::quint_element_state(state),
            })
            .collect();
        self.messages.extend(self.ui.update(changes));
    }

    pub fn rebuild(&mut self, _fps: usize, data: &WindowData) {
        use quint::WidgetTree;

        self.update();

        let mut menu_button_count = 0;
        let mut menu_button = |text: &'static str, message| {

            menu_button_count += 1;
            quint::wt! {
                widgets::Button {
                    text: text.to_owned(),
                    font_size: gfx_glyph::Scale::uniform(40.0),
                    message,
                    style: quint::Style::default().absolute_size(400.0, 100.0),
                },
            }
        };

        let buttons = vec![
            menu_button("Start Singleplayer Game", UiMessage::StartSinglePlayer),
            menu_button("Exit Game", UiMessage::ExitGame),
        ];

        let menu_layer = WidgetTree::new(
            Box::new(widgets::WithStyle {
                style: quint::Style::default()
                    .percent_size(1.0, 1.0)
                    .center_cross()
                    .center_main()
                    .vertical(),
            }),
            buttons,
        );

        let (win_w, win_h) = (
            data.logical_window_size.width,
            data.logical_window_size.height,
        );
        self.ui.rebuild(
            vec![menu_layer],
            quint::Size {
                width: win_w as f32,
                height: win_h as f32,
            },
        );
    }

    pub fn update(&mut self) {
        for message in self.messages.drain(..) {
            match message {
                UiMessage::StartSinglePlayer => self.should_start_single_player = true,
                UiMessage::ExitGame => self.should_exit = true,
            }
        }
    }
}