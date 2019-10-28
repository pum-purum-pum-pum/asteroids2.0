use super::*;
use gfx_h::{TextData, WorldTextData};

pub struct ScoreTableRendering {
    reader: ReaderId<Primitive>,
}

impl ScoreTableRendering {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        ScoreTableRendering { reader: reader }
    }
}

impl<'a> System<'a> for ScoreTableRendering {
    type SystemData = (
        ReadStorage<'a, ThreadPin<ImageData>>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        Read<'a, Mouse>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        WriteExpect<'a, ThreadPin<WorldTextData<'static>>>,
        Write<'a, AppState>,
        ReadExpect<'a, MacroGame>,
    );
    fn run(&mut self, data: Self::SystemData) {
        let (
            image_datas,
            gl,
            mut canvas,
            viewport,
            mut primitives_channel,
            mut ui,
            mouse,
            mut text_data,
            mut world_text_data,
            mut app_state,
            macro_game,
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let (button_w, button_h) = (w / 4f32, h / 4f32);

        let mut current_h = h / 20.0;
        let text_gap_h = h / 20.0; // TODO somehow measure it
        for score in macro_game.score_table.iter() {
            current_h += text_gap_h;
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w / 20.0, current_h),
                    color: (1.0, 1.0, 1.0, 1.0),
                    text: format!("{}", score).to_string(),
                }),
                with_projection: false,
            });
        }

        let back_to_menu = Button::new(
            Point2::new(w / 2.0, 1.5 * button_h),
            button_w,
            button_h,
            Some(Point3::new(0f32, 0f32, 0f32)),
            false,
            None,
            "Back to Menu".to_string(),
            Widgets::BackMenu as usize,
            None,
            None,
        );
        if back_to_menu.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::Menu;
        }

        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
            &mut world_text_data,
        );
    }
}
