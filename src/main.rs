use bevy::{
    input::gamepad::{AxisSettings, GamepadButton, GamepadSettings},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_rapier2d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, startup)
        .add_systems(Update, controller_setup)
        .add_systems(Update, update)
        .run();
}

enum ZOrder {
    PLAYER,
    TURRET,
    BULLET,
}
impl From<ZOrder> for f32 {
    fn from(value: ZOrder) -> Self {
        value as i32 as f32
    }
}

#[derive(Component)]
struct Tank {
    id: usize,
    speed: f32,
    facing: Vec2,
}
impl Tank {
    const DEFAULT_SPEED: f32 = 500.0;
    const DASH_SPEED: f32 = 2000.0;
    const SPEED_FALLOFF: f32 = (Tank::DASH_SPEED - Tank::DEFAULT_SPEED) / 0.2;
}

fn controller_setup(gamepads: Res<Gamepads>, mut settings: ResMut<GamepadSettings>) {
    let mut stick_settings = AxisSettings::default();
    stick_settings.set_deadzone_lowerbound(-0.1);
    stick_settings.set_deadzone_upperbound(0.1);
    for gamepad in gamepads.iter() {
        settings.axis_settings.insert(
            GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickX,
            },
            stick_settings.clone(),
        );
        settings.axis_settings.insert(
            GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickY,
            },
            stick_settings.clone(),
        );
        settings.axis_settings.insert(
            GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickX,
            },
            stick_settings.clone(),
        );
        settings.axis_settings.insert(
            GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickY,
            },
            stick_settings.clone(),
        );
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
                material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
                transform: Transform::from_translation(Vec3::new(150., 0., ZOrder::PLAYER.into())),
                ..default()
            },
            Tank {
                id: 0,
                speed: 0.0,
                facing: Vec2::new(0.0, 1.0),
            },
        ))
        .with_children(|builder| {
            builder.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(70.0, 20.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(35., 0., ZOrder::TURRET.into())),
                ..default()
            });
        });
}

fn update(
    time: Res<Time>,
    mut commands: Commands,
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut query: Query<(&mut Tank, &mut Transform)>,
) {
    for (mut tank, mut transform) in &mut query {
        tank.speed = (tank.speed - Tank::SPEED_FALLOFF * time.delta_seconds())
            .clamp(Tank::DEFAULT_SPEED, Tank::DASH_SPEED);
        match gamepads.iter().find(|&x| x.id == tank.id) {
            Some(gamepad) => {
                transform.translation.x += tank.speed
                    * time.delta_seconds()
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
                        .unwrap();
                transform.translation.y += tank.speed
                    * time.delta_seconds()
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
                        .unwrap();
                if button_inputs.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::East))
                {
                    tank.speed = Tank::DASH_SPEED;
                }

                let right_stick_x = axes
                    .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickX))
                    .unwrap();
                let right_stick_y = axes
                    .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickY))
                    .unwrap();
                let right_stick_angle = f32::atan2(right_stick_y, right_stick_x);
                if f32::max(right_stick_x.abs(), right_stick_y.abs()) > 0.1 {
                    tank.facing = Vec2::new(right_stick_x, right_stick_y).normalize();
                    transform.rotation = Quat::from_rotation_z(right_stick_angle);
                }

                if button_inputs
                    .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::RightTrigger))
                {
                    commands.spawn((
                        RigidBody::Dynamic,
                        Collider::ball(10.0),
                        Restitution::coefficient(0.7),
                        Velocity::linear(1000.0 * tank.facing),
                        GravityScale(0.0),
                        TransformBundle::from(Transform::from_xyz(
                            transform.translation.x,
                            transform.translation.y,
                            ZOrder::BULLET.into(),
                        )),
                    ));
                }
            }

            None => warn!(
                "No corresponding gamepad found for tank with id {:?}",
                tank.id
            ),
        }
    }
}
