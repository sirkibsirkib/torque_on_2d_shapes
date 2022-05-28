use ggez::{
    event::{self, quit, EventHandler, MouseButton},
    graphics::{self, Color, DrawParam, Mesh, Transform},
    input::keyboard::{KeyCode, KeyMods},
    Context, ContextBuilder, GameResult,
};
use glam::Vec2 as VecXy;

#[derive(Default, Debug)]
struct FieldScalars {
    xy: VecXy,
    angle: f32,
}

#[derive(Debug, Copy, Clone)]
struct VecLa {
    length: f32,
    angle: f32,
}

struct Body {
    translation_scalar: f32, // inverse of mass
    rotation_scalar: f32,    // inverse of "angle mass"
    pos: FieldScalars,
    vel: FieldScalars,
    scale: VecXy,
    handle: VecLa,
}
struct MyGame {
    rect_mash: Mesh,
    body: Body,
}
struct VecLaTug {
    r: VecXy, // (point of force) - (center of mass)
    f: VecXy, // force vector
}

/////////////////////////////////

fn rotate_vec_xy(xy: VecXy, angle: f32) -> VecXy {
    let [sa, ca] = [angle.sin(), angle.cos()];
    let [x, y] = xy.to_array();
    VecXy::new(x * ca - y * sa, x * sa + y * ca)
}

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
        self.angle += other.angle;
    }
}
impl Body {
    fn tug(&mut self, VecLaTug { r, f }: VecLaTug) {
        let f_parr = f.project_onto(r);
        let f_perp = f - f_parr;
        // dbg!(f, r, f_parr, f_perp, r.angle_between(f_perp));
        self.vel.angle += self.rotation_scalar
            * f_perp.length()
            * if r.angle_between(f_perp) > 0. { 1. } else { -1. };

        self.vel.xy += self.translation_scalar * f_parr;
    }
    fn relative_handle_xy(&self) -> VecXy {
        self.handle.rotated(self.pos.angle).to_xy()
    }
    fn handle_xy(&self) -> VecXy {
        self.relative_handle_xy() + self.pos.xy
    }
}
impl VecLa {
    fn to_xy(self) -> VecXy {
        rotate_vec_xy(VecXy::new(self.length, 0.), self.angle)
    }
    fn from_xy(xy: VecXy) -> Self {
        Self { length: xy.length(), angle: xy.y.atan2(xy.x) }
    }
    fn rotated(mut self, angle_delta: f32) -> Self {
        self.angle += angle_delta;
        self
    }
}
impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        MyGame {
            body: Body {
                translation_scalar: 0.001,
                rotation_scalar: 0.00002,
                pos: FieldScalars { xy: VecXy::splat(300.), angle: 1. },
                vel: FieldScalars { xy: VecXy::splat(0.), angle: 0. },
                handle: VecLa { length: 24., angle: 3. },
                scale: VecXy::splat(50.),
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
    // fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
    //     let mouse_xy = VecXy::new(x, y);
    //     if let MouseButton::Left = button {
    //         let r = self.body.relative_handle_xy();
    //         let f = mouse_xy - self.body.pos.xy;
    //         self.body.tug(VecLaTug { r, f });
    //     }
    // }
    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        repeat: bool,
    ) {
        if repeat {
            return;
        }
        match keycode {
            KeyCode::Escape => quit(ctx),
            _ => {}
        }
    }
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let b = &mut self.body;

        // yanking
        let mouse_xy = VecXy::from(ggez::input::mouse::position(ctx));
        b.tug(VecLaTug { r: b.relative_handle_xy(), f: mouse_xy - b.pos.xy });

        // acceleration
        b.pos.add_from(&b.vel);

        //friction
        b.vel.xy *= 0.99;
        b.vel.angle *= 0.995;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);
        let mouse_xy = ggez::input::mouse::position(ctx);
        let b = &self.body;
        // draw body
        graphics::draw(
            ctx,
            &self.rect_mash,
            DrawParam {
                trans: Transform::Values {
                    dest: b.pos.xy.into(),
                    rotation: b.pos.angle,
                    scale: b.scale.into(),
                    offset: VecXy::ZERO.into(),
                },
                color: Color::WHITE,
                ..Default::default()
            },
        )?;

        // draw handle
        let handle_xy = b.handle_xy();
        let cord_la = VecLa::from_xy(handle_xy - VecXy::from(mouse_xy));
        graphics::draw(
            ctx,
            &self.rect_mash,
            DrawParam {
                trans: Transform::Values {
                    dest: handle_xy.into(),
                    rotation: cord_la.angle,
                    scale: VecXy::new(cord_la.length, 1.).into(),
                    offset: VecXy::new(0.5, 0.).into(),
                },
                color: Color::RED,
                ..Default::default()
            },
        )?;

        graphics::present(ctx)
    }
}
