use ggez::{
    event::{self, quit, EventHandler, MouseButton},
    graphics::{self, Color, DrawParam, Mesh, Transform},
    input::keyboard::{KeyCode, KeyMods},
    Context, ContextBuilder, GameResult,
};
use glam::Vec2 as VecXy;

/// 2D vector in length-angle form
#[derive(Debug, Copy, Clone)]
struct VecLa {
    length: f32,
    angle: f32,
}

/// generalization of {position, velocity, ...} of rotating 2d body
#[derive(Default, Debug)]
struct FieldScalars {
    xy: VecXy,
    angle: f32,
}

/// Body statics for acceleration of some scalar
struct VelocityStatic {
    acc_scalar: f32,
    linear_friction_scalar: f32,
    constant_friction: f32,
}

/// VelocityStatic for {xy, angle}
struct VelocityStatics {
    xy: VelocityStatic,
    angle: VelocityStatic,
}

struct Tugger {
    relative_body_handle_xy: VecLa,
    world_dest: VecXy,
}

/// A 2d shape in the game world
struct Body {
    statics: VelocityStatics,
    pos: FieldScalars,
    vel: FieldScalars,
    scale: VecXy,
    tuggers: [Option<Tugger>; 3],
    max_tug_handle_distance: f32,
}

/// Game state
struct MyGame {
    rect_mash: Mesh,
    bodies: [Body; 2],
}

/// Utility functions for `f32` type. Workaround of orphan rule.
trait NegIf: Sized {
    fn neg_if(self, cond: bool) -> Self;
    fn toward_zero_saturating(self, by: f32) -> Self;
}

/// Utility functions for `VecXy` type. Workaround of orphan rule.
trait VecXyExt: Sized {
    fn rotated(self, angle: f32) -> Self;
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
    fn rotated(self, angle: f32) -> Self {
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

impl VecLa {
    fn to_xy(self) -> VecXy {
        VecXy::new(self.length, 0.).rotated(self.angle)
    }
    fn from_xy(xy: VecXy) -> Self {
        Self { length: xy.length(), angle: xy.y.atan2(xy.x) }
    }
}

impl FieldScalars {
    fn add(mut self, other: Self) -> Self {
        self.add_from(&other);
        self
    }
    fn add_from(&mut self, other: &Self) {
        self.xy += other.xy;
        self.angle += other.angle;
    }
}

impl Body {
    fn xy_relative_handle(&self, mut body_handle: VecLa) -> VecXy {
        body_handle.angle += self.pos.angle;
        body_handle.to_xy()
    }
    fn absolute_handle(&self, body_handle: VecLa) -> VecXy {
        self.xy_relative_handle(body_handle) + self.pos.xy
    }

    /// Inspired by https://en.wikipedia.org/wiki/Angular_momentum
    /// contact: force application point relative to my center of mass
    fn tug_acc(&self, contact: VecXy, force: VecXy) -> FieldScalars {
        if force == VecXy::ZERO {
            // correct: zero force has no effect
            // necessary: otherwise projection returns NaN
            return FieldScalars::default();
        }

        // split force vector up into [force rotatable, force unrotatable]
        let [fr, fu]: [VecXy; 2] = {
            // 0. when contact is at center of mass
            // 1. when contact is at max tug handle distance
            let rotatable_proportion = contact.length() / self.max_tug_handle_distance;
            assert!(0. <= rotatable_proportion);
            assert!(rotatable_proportion <= 1.);
            let fr = force * rotatable_proportion;
            [fr, force - fr]
        };

        // split rotatable force up into [parallel, perpindicular] components wrt contact
        let [fr_parr, fr_perp] = fr.split_parr_perp(contact);

        FieldScalars {
            xy: self.statics.xy.acc_scalar * (fu + fr.with_length(fr_parr.length())),
            angle: self.statics.angle.acc_scalar
                * fr_perp.length()
                * if contact.angle_between(fr_perp) < 0. { -1. } else { 1. },
        }
    }
}

impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        MyGame {
            bodies: [
                Body {
                    statics: VelocityStatics {
                        xy: VelocityStatic {
                            acc_scalar: 0.003,
                            linear_friction_scalar: 0.99,
                            constant_friction: 0.001,
                        },
                        angle: VelocityStatic {
                            acc_scalar: 0.00009,
                            linear_friction_scalar: 0.99,
                            constant_friction: 0.0001,
                        },
                    },
                    pos: FieldScalars { xy: VecXy::splat(300.), angle: 1. },
                    vel: FieldScalars { xy: VecXy::splat(0.), angle: 0. },
                    scale: VecXy::new(50., 50.),
                    tuggers: [
                        None,
                        Some(Tugger {
                            world_dest: VecXy::new(300., 280.),
                            relative_body_handle_xy: VecLa { length: 7., angle: 2. },
                        }),
                        Some(Tugger {
                            world_dest: VecXy::new(400., 220.),
                            relative_body_handle_xy: VecLa { length: 9., angle: 2.4 },
                        }),
                    ],
                    max_tug_handle_distance: 35.,
                },
                Body {
                    statics: VelocityStatics {
                        xy: VelocityStatic {
                            acc_scalar: 0.002,
                            linear_friction_scalar: 0.99,
                            constant_friction: 0.001,
                        },
                        angle: VelocityStatic {
                            acc_scalar: 0.00007,
                            linear_friction_scalar: 0.99,
                            constant_friction: 0.0001,
                        },
                    },
                    pos: FieldScalars { xy: VecXy::splat(300.), angle: 1. },
                    vel: FieldScalars { xy: VecXy::splat(0.), angle: 0. },
                    scale: VecXy::new(80., 30.),
                    tuggers: [
                        None,
                        Some(Tugger {
                            world_dest: VecXy::new(450., 100.),
                            relative_body_handle_xy: VecLa { length: 35., angle: 0.3 },
                        }),
                        Some(Tugger {
                            world_dest: VecXy::new(510., 400.),
                            relative_body_handle_xy: VecLa { length: 30., angle: 3.1 },
                        }),
                    ],
                    max_tug_handle_distance: 80.,
                },
            ],
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
            for body in self.bodies.iter_mut() {
                let relative_body_handle_xy =
                    VecLa::from_xy((mouse_xy - body.pos.xy).rotated(-body.pos.angle));
                if relative_body_handle_xy.length <= body.max_tug_handle_distance {
                    body.tuggers[0] =
                        Some(Tugger { relative_body_handle_xy, world_dest: mouse_xy });
                }
            }
        }
    }
    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        if let MouseButton::Left = button {
            for body in self.bodies.iter_mut() {
                body.tuggers[0] = None;
            }
        }
    }
    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        for body in self.bodies.iter_mut() {
            if let Some(tugger) = &mut body.tuggers[0] {
                tugger.world_dest = VecXy::new(x, y);
            }
        }
    }
    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _: KeyMods, repeat: bool) {
        if repeat {
            return;
        }
        match keycode {
            KeyCode::Escape => quit(ctx),
            KeyCode::Space => {
                for body in self.bodies[1..].iter_mut() {
                    for tugger in body.tuggers.iter_mut().filter_map(Option::as_mut) {
                        let [x, y]: [f32; 2] = tugger.world_dest.into();
                        tugger.world_dest = VecXy::new(y, x);
                    }
                }
            }
            _ => {}
        }
    }
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        for body in self.bodies.iter_mut() {
            // update velocity wrt tug
            let mut acc = body
                .tuggers
                .iter()
                .filter_map(Option::as_ref)
                .map(|tugger| {
                    let xy_relative_handle =
                        body.xy_relative_handle(tugger.relative_body_handle_xy);
                    let force = tugger.world_dest - (xy_relative_handle + body.pos.xy);
                    body.tug_acc(xy_relative_handle, force)
                })
                .fold(FieldScalars::default(), FieldScalars::add);

            //gravity
            acc.xy.y += 0.15;

            body.vel.add_from(&acc);
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

        for body in self.bodies.iter_mut() {
            // draw body
            graphics::draw(
                ctx,
                &self.rect_mash,
                DrawParam {
                    trans: Transform::Values {
                        dest: body.pos.xy.into(),
                        rotation: body.pos.angle,
                        scale: body.scale.into(),
                        offset: VecXy::ZERO.into(),
                    },
                    color: Color::WHITE,
                    ..Default::default()
                },
            )?;

            // draw tug ropes
            for tugger in body.tuggers.iter().filter_map(Option::as_ref) {
                let body_handle_xy = body.absolute_handle(tugger.relative_body_handle_xy);
                let rope_la = VecLa::from_xy(body_handle_xy - tugger.world_dest);
                graphics::draw(
                    ctx,
                    &self.rect_mash,
                    DrawParam {
                        trans: Transform::Values {
                            dest: body_handle_xy.into(),
                            rotation: rope_la.angle,
                            scale: VecXy::new(rope_la.length, 1.).into(),
                            offset: VecXy::new(0.5, 0.).into(),
                        },
                        color: Color::RED,
                        ..Default::default()
                    },
                )?;
            }
        }
        graphics::present(ctx)
    }
}

fn main() {
    let (mut ctx, event_loop) =
        ContextBuilder::new("torque_on_2d_shapes", "Chris").build().expect("WAH!");
    let my_game = MyGame::new(&mut ctx);
    event::run(ctx, event_loop, my_game);
}
