use bevy::{
    input::gamepad::{AxisSettings, GamepadButton, GamepadSettings},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_rapier2d::prelude::*;
use std::iter::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, startup)
        .add_systems(Update, controller_setup)
        .add_systems(Update, update)
        .add_systems(PostUpdate, handle_bullet_hit)
        .add_systems(PostUpdate, fade_out)
        .run();
}

enum ZOrder {
    Bullet,
    Player,
    HitEffect,
    Turret,
}
impl From<ZOrder> for f32 {
    fn from(value: ZOrder) -> Self {
        value as i32 as f32
    }
}

#[derive(Component)]
struct Bullet {
    source: Entity,
}

#[derive(Component)]
struct FadeOut {
    t: f32,
}

#[derive(Component)]
struct Tank {
    id: usize,
    health: i32,
    dash_cooldown: f32,
    turret_cooldown: f32,
    speed: f32,
    facing: Vec2,
}
impl Tank {
    const DEFAULT_SPEED: f32 = 300.0;
    const DASH_SPEED: f32 = 1500.0;
    const DEFAULT_TURRET_COOLDOWN: f32 = 0.8;
    const DEFAULT_DASH_COOLDOWN: f32 = 0.5;
    const SPEED_FALLOFF: f32 = (Tank::DASH_SPEED - Tank::DEFAULT_SPEED) / 0.2;

    fn new(id: usize) -> Self {
        Self {
            id,
            health: 3,
            dash_cooldown: 0.0,
            turret_cooldown: 0.0,
            speed: Tank::DEFAULT_SPEED,
            facing: Vec2::new(1.0, 0.0),
        }
    }
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
    mut rapier_config: ResMut<RapierConfiguration>,
    query_window: Query<&Window>,
) {
    commands.spawn(Camera2dBundle::default());
    rapier_config.gravity = Vec2::ZERO;

    let window_height = query_window.get_single().unwrap().height();
    let window_width = query_window.get_single().unwrap().width();

    for (id, pos_x) in zip(0..2, vec![-150.0, 150.0]) {
        commands
            .spawn((
                RigidBody::Dynamic,
                ActiveEvents::COLLISION_EVENTS,
                Velocity::linear(Vec2::ZERO),
                Collider::ball(50.0),
                MaterialMesh2dBundle {
                    mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
                    transform: Transform::from_translation(Vec3::new(
                        pos_x,
                        0.,
                        ZOrder::Player.into(),
                    )),
                    ..default()
                },
                Tank::new(id),
            ))
            .with_children(|builder| {
                builder.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(70.0, 20.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        35.,
                        0.,
                        ZOrder::Turret.into(),
                    )),
                    ..default()
                });
            });
    }
    for (pos, size) in std::iter::zip(
        vec![
            Vec2::new(0.0, -150.0),
            Vec2::new(0.0, 150.0),
            Vec2::new(-window_width / 2.0, 0.0),
            Vec2::new(0.0, -window_height / 2.0),
            Vec2::new(window_width / 2.0, 0.0),
            Vec2::new(0.0, window_height / 2.0),
        ],
        vec![
            Vec2::new(150.0, 150.0),
            Vec2::new(150.0, 150.0),
            Vec2::new(10.0, window_height),
            Vec2::new(window_width, 10.0),
            Vec2::new(10.0, window_height),
            Vec2::new(window_width, 10.0),
        ],
    ) {
        commands.spawn((
            RigidBody::Fixed,
            Collider::cuboid(size.x / 2.0, size.y / 2.0),
            SpriteBundle {
                sprite: Sprite {
                    color: Color::BLACK,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    pos.x,
                    pos.y,
                    ZOrder::Player.into(),
                )),
                ..default()
            },
        ));
    }
}

fn update(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut query: Query<(Entity, &mut Tank, &mut Transform, &mut Velocity)>,
) {
    for (entity, mut tank, mut transform, mut velocity) in &mut query {
        tank.speed = (tank.speed - Tank::SPEED_FALLOFF * time.delta_seconds())
            .clamp(Tank::DEFAULT_SPEED, Tank::DASH_SPEED);
        tank.turret_cooldown -= time.delta_seconds();
        tank.dash_cooldown -= time.delta_seconds();

        match gamepads.iter().find(|&x| x.id == tank.id) {
            Some(gamepad) => {
                velocity.linvel.x = tank.speed
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
                        .unwrap();
                velocity.linvel.y = tank.speed
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
                        .unwrap();
                if button_inputs
                    .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger))
                    && tank.dash_cooldown < 0.0
                {
                    tank.dash_cooldown = Tank::DEFAULT_DASH_COOLDOWN;
                    tank.speed = Tank::DASH_SPEED;
                }

                let right_stick_x = axes
                    .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickX))
                    .unwrap();
                let right_stick_y = axes
                    .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickY))
                    .unwrap();
                if f32::max(right_stick_x.abs(), right_stick_y.abs()) > 0.1 {
                    tank.facing.x = right_stick_x;
                    tank.facing.y = right_stick_y;
                }
                tank.facing = tank.facing.normalize();
                let right_stick_angle = f32::atan2(tank.facing.y, tank.facing.x);
                transform.rotation = Quat::from_rotation_z(right_stick_angle);

                if button_inputs.pressed(GamepadButton::new(
                    gamepad,
                    GamepadButtonType::RightTrigger2,
                )) && tank.turret_cooldown < 0.0
                {
                    tank.turret_cooldown = Tank::DEFAULT_TURRET_COOLDOWN;
                    commands.spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes.add(shape::RegularPolygon::new(10., 6).into()).into(),
                            material: materials.add(ColorMaterial::from(Color::RED)),
                            transform: Transform::from_xyz(
                                transform.translation.x,
                                transform.translation.y,
                                ZOrder::Bullet.into(),
                            ),
                            ..default()
                        },
                        RigidBody::Dynamic,
                        Sensor,
                        Collider::ball(10.0),
                        Restitution::coefficient(0.7),
                        Velocity::linear(1500.0 * tank.facing + velocity.linvel),
                        GravityScale(0.0),
                        Bullet { source: entity },
                    ));
                }
            }

            None => (), //warn!("No corresponding gamepad found for tank with id {:?}", tank.id),
        }
    }
}

fn handle_bullet_hit(
    rapier_context: Res<RapierContext>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query_tank: Query<(Entity, Option<&mut Tank>)>,
    query_bullet: Query<(Entity, &Bullet)>,
    mut commands: Commands,
) {
    for (entity_bullet, bullet) in &query_bullet {
        for (entity_other, tank_opt) in &mut query_tank {
            if bullet.source == entity_other {
                continue;
            }
            if rapier_context.intersection_pair(entity_other, entity_bullet) == Some(true) {
                commands.entity(entity_bullet).despawn();

                if let Some(mut tank) = tank_opt {
                    tank.health -= 1;
                    let child = commands
                        .spawn((
                            MaterialMesh2dBundle {
                                mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
                                material: materials.add(ColorMaterial::from(Color::RED)),
                                transform: Transform::from_translation(Vec3::new(
                                    0.,
                                    0.,
                                    ZOrder::HitEffect.into(),
                                )),
                                ..default()
                            },
                            FadeOut { t: 0.05 },
                        ))
                        .id();
                    commands.entity(entity_other).add_child(child);
                }
            }
        }
    }
}

fn fade_out(time: Res<Time>, mut commands: Commands, mut query: Query<(Entity, &mut FadeOut)>) {
    for (entity, mut fade_out) in &mut query {
        fade_out.t -= time.delta_seconds();
        if fade_out.t < 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
