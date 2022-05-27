use ggez::{
    event::{self, EventHandler},
    graphics::{self, Color, DrawParam, Mesh, Transform},
    Context, ContextBuilder, GameResult,
};

#[derive(Default, Debug)]
struct FieldScalars {
    xy: glam::Vec2,
    rotation: f32,
}

struct Body {
    translation_scalar: f32, // inverse of mass
    rotation_intertia: f32,  // inverse of "rotation mass"
    pos: FieldScalars,
    vel: FieldScalars,
    scale: glam::Vec2,
}
struct MyGame {
    rect_mash: Mesh,
    body: Body,
}

/////////////////////////////////

fn main() {
    let (mut ctx, event_loop) = ContextBuilder::new("my_game", "Cool Game Author")
        .build()
        .expect("aieee, could not create ggez context!");
    let my_game = MyGame::new(&mut ctx);
    event::run(ctx, event_loop, my_game);
}

impl FieldScalars {
    fn add_from(&mut self, other: &Self) {
        self.xy += other.xy;
        self.rotation += other.rotation;
    }
}
impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        MyGame {
            body: Body {
                translation_scalar: 1.,
                rotation_intertia: 1.,
                pos: FieldScalars::default(),
                vel: FieldScalars { xy: glam::Vec2::splat(2.1), rotation: 0.01 },
                scale: glam::Vec2::splat(50.),
            },
            rect_mash: Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                ggez::graphics::Rect { x: -0.5, y: -0.5, w: 1., h: 1. },
                Color::WHITE,
            )
            .expect("new mesh fail"),
        }
    }
}

impl EventHandler for MyGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.body.pos.add_from(&self.body.vel);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);
        for b in [&self.body] {
            graphics::draw(
                ctx,
                &self.rect_mash,
                DrawParam {
                    trans: Transform::Values {
                        dest: b.pos.xy.into(),
                        rotation: b.pos.rotation,
                        scale: b.scale.into(),
                        offset: glam::Vec2::ZERO.into(),
                    },
                    color: Color::WHITE,
                    ..Default::default()
                },
            )?;
        }
        graphics::present(ctx)
    }
}
