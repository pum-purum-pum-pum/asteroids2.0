pub use super::*;

#[derive(Default)]
pub struct DeadScreen;

impl<'a> System<'a> for DeadScreen {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, MacroGame>,
        Write<'a, Progress>,
        Write<'a, UI>,
        Write<'a, AppState>,
        Write<'a, CurrentWave>,
        Write<'a, SpawnedUpgrades>,
        Read<'a, Mouse>,
        WriteExpect<'a, PreloadedImages>,
        ReadStorage<'a, ShipMarker>,
        ReadStorage<'a, AsteroidMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            viewport,
            mut macro_game,
            mut progress,
            mut ui,
            mut app_state,
            mut current_wave,
            mut spawned_upgrades,
            mouse,
            preloaded_images,
            ship_markers,
            asteroid_markers,
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        // dark background
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Picture(Picture {
                position: Point2::new(0f32, 0f32),
                width: w,
                height: h,
                image: Image(preloaded_images.transparent_sqr),
            }),
            with_projection: false,
        });
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w / 2.0, h / 2.0),
                text: format!("Your score: {}", progress.score),
            }),
            with_projection: false,
        });
        let to_menu_w = w / 10f32;
        let to_menu_h = h / 10f32 / 4.0;
        let to_menu = Button::new(
            Point2::new(w / 2.0 - to_menu_w, h - 3.0 * to_menu_h),
            to_menu_w,
            to_menu_h,
            None,
            false,
            Some(Image(preloaded_images.upg_bar)),
            "To menu".to_string(),
            Widgets::Upgrade as usize,
            None,
            None,
        );
        if to_menu.place_and_check(&mut ui, &*mouse) {
            for (entity, _ship_marker) in (&entities, &ship_markers).join() {
                entities.delete(entity).unwrap();
            }
            for (entity, _asteroid_marker) in (&entities, &asteroid_markers).join() {
                entities.delete(entity).unwrap();
            }
            *spawned_upgrades = vec![];
            *app_state = AppState::Menu;
            macro_game.score_table.push(progress.score);
            macro_game.score_table.sort_by(|a, b| b.cmp(a));
            *progress = Progress::default();
            *current_wave = CurrentWave::default();
        }
    }
}
