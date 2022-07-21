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

struct VelocityStatic {
    vel_scalar: f32,
    linear_friction_scalar: f32,
    constant_friction: f32,
}
struct VelocityStatics {
    xy: VelocityStatic,
    angle: VelocityStatic,
}

struct Body {
    statics: VelocityStatics,
    pos: FieldScalars,
    vel: FieldScalars,
    scale: VecXy,
    tug_handle: Option<VecLa>, //
    max_tug_handle_distance: f32,
}
struct MyGame {
    rect_mash: Mesh,
    body: Body,
}

///
struct Tug {
    // e.g. https://en.wikipedia.org/wiki/Angular_momentum
    contact: VecXy, // called "r". force application point relative to body's center of mass
    force: VecXy,   // called "f"
}

trait NegIf: Sized {
    fn neg_if(self, cond: bool) -> Self;
    fn toward_zero_saturating(self, by: f32) -> Self;
}
trait VecXyExt: Sized {
    fn rotate(self, angle: f32) -> Self;
    fn split_parr_perp(self, other: Self) -> [Self; 2];
    fn with_length(self, length: f32) -> Self;
    fn reduce_length_saturating(self, by: f32) -> Self;
}
/////////////////////////////////
impl NegIf for f32 {
    fn neg_if(self, cond: bool) -> Self {
        if cond {
            -self
        } else {
            self
        }
    }
    fn toward_zero_saturating(self, by: f32) -> Self {
        if self >= 0. {
            (self - by).max(0.)
        } else {
            (self + by).min(0.)
        }
    }
}
impl VecXyExt for VecXy {
    fn rotate(self, angle: f32) -> Self {
        let [sa, ca] = [angle.sin(), angle.cos()];
        let [x, y] = self.to_array();
        Self::new(x * ca - y * sa, x * sa + y * ca)
    }
    fn split_parr_perp(self, other: Self) -> [Self; 2] {
        let parr = self.project_onto(other);
        let perp = self - parr;
        [parr, perp]
    }
    fn with_length(self, length: f32) -> Self {
        self.normalize_or_zero() * length
    }
    fn reduce_length_saturating(self, by: f32) -> Self {
        self.with_length(self.length().toward_zero_saturating(by))
    }
}
fn main() {
    let (mut ctx, event_loop) =
        ContextBuilder::new("torque_on_2d_shapes", "Chris").build().expect("WAH!");
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
    fn tug(&mut self, Tug { contact, force: f }: Tug) {
        if f == VecXy::ZERO {
            // correct: zero force has no effect
            // necessary: otherwise projection returns NaN
            return;
        }

        // split force vector up into [force rotatable, force unrotatable]
        let [fr, fu]: [VecXy; 2] = {
            // 0. when contact is at center of mass
            // 1. when contact is at max tug handle distance
            let rotatable_proportion = contact.length() / self.max_tug_handle_distance;
            assert!(0. <= rotatable_proportion);
            assert!(rotatable_proportion <= 1.);
            let fr = f * rotatable_proportion;
            [fr, f - fr]
        };

        // split rotatable force up into [parallel, perpindicular] components wrt contact
        let [fr_parr, fr_perp] = fr.split_parr_perp(contact);

        // accelerate xy with unrotatable and parallel component of rotatable
        self.vel.xy += self.statics.xy.vel_scalar * (fu + fr.with_length(fr_parr.length()));

        // accelerate angle with perpindicular component of rotatable
        self.vel.angle += self.statics.angle.vel_scalar
            * fr_perp.length()
            * if contact.angle_between(fr_perp) < 0. { -1. } else { 1. };
    }
    fn relative_tug_handle_xy(&self) -> Option<VecXy> {
        Some(self.tug_handle?.rotated(self.pos.angle).to_xy())
    }
    fn absolutify_relative_xy(&self, relative_xy: VecXy) -> VecXy {
        self.pos.xy + relative_xy
    }
    fn relativize_absolute_xy(&self, absolute_xy: VecXy) -> VecXy {
        absolute_xy - self.pos.xy
    }
    fn absolute_tug_handle_xy(&self) -> Option<VecXy> {
        Some(self.absolutify_relative_xy(self.relative_tug_handle_xy()?))
    }
}
impl VecLa {
    fn to_xy(self) -> VecXy {
        VecXy::new(self.length, 0.).rotate(self.angle)
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
                statics: VelocityStatics {
                    xy: VelocityStatic {
                        vel_scalar: 0.003,
                        linear_friction_scalar: 0.99,
                        constant_friction: 0.001,
                    },
                    angle: VelocityStatic {
                        vel_scalar: 0.00009,
                        linear_friction_scalar: 0.99,
                        constant_friction: 0.0001,
                    },
                },
                pos: FieldScalars { xy: VecXy::splat(300.), angle: 1. },
                vel: FieldScalars { xy: VecXy::splat(0.), angle: 0. },
                scale: VecXy::splat(50.),
                tug_handle: None,
                max_tug_handle_distance: 35.,
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
    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let MouseButton::Left = button {
            let mouse_xy = VecXy::new(x, y);
            let vec_la = VecLa::from_xy(self.body.relativize_absolute_xy(mouse_xy))
                .rotated(-self.body.pos.angle);
            if vec_la.length <= self.body.max_tug_handle_distance {
                self.body.tug_handle = Some(vec_la);
            }
        }
    }
    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _: f32, _: f32) {
        if let MouseButton::Left = button {
            self.body.tug_handle = None;
        }
    }
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
        let mouse_xy = VecXy::from(ggez::input::mouse::position(ctx));
        for body in [&mut self.body] {
            // update velocity wrt tug
            if let Some(contact) = body.relative_tug_handle_xy() {
                let force = mouse_xy - body.absolutify_relative_xy(contact);
                body.tug(Tug { contact, force });
            }
            // accelerate
            body.pos.add_from(&body.vel);

            // linear friction
            body.vel.xy *= body.statics.xy.linear_friction_scalar;
            body.vel.angle *= body.statics.angle.linear_friction_scalar;

            // constant friction
            body.vel.xy = body.vel.xy.reduce_length_saturating(body.statics.xy.constant_friction);
            body.vel.angle =
                body.vel.angle.toward_zero_saturating(body.statics.angle.constant_friction);
        }
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

        // draw tug_handle
        if let Some(tug_handle_xy) = b.absolute_tug_handle_xy() {
            let cord_la = VecLa::from_xy(tug_handle_xy - VecXy::from(mouse_xy));
            graphics::draw(
                ctx,
                &self.rect_mash,
                DrawParam {
                    trans: Transform::Values {
                        dest: tug_handle_xy.into(),
                        rotation: cord_la.angle,
                        scale: VecXy::new(cord_la.length, 1.).into(),
                        offset: VecXy::new(0.5, 0.).into(),
                    },
                    color: Color::RED,
                    ..Default::default()
                },
            )?;
        }
        graphics::present(ctx)
    }
}
