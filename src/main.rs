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
    speed: f32,
    facing: Vec2,
}
impl Tank {
    const DEFAULT_SPEED: f32 = 500.0;
    const DASH_SPEED: f32 = 2000.0;
    const SPEED_FALLOFF: f32 = (Tank::DASH_SPEED - Tank::DEFAULT_SPEED) / 0.2;

    fn new(id: usize) -> Self {
        Self {
            id,
            health: 3,
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
) {
    commands.spawn(Camera2dBundle::default());

    for id in 0..2 {
        commands
            .spawn((
                RigidBody::KinematicPositionBased,
                ActiveEvents::COLLISION_EVENTS,
                Sensor,
                Collider::ball(50.0),
                MaterialMesh2dBundle {
                    mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
                    transform: Transform::from_translation(Vec3::new(
                        150.,
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
}

fn update(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut query: Query<(Entity, &mut Tank, &mut Transform)>,
) {
    for (entity, mut tank, mut transform) in &mut query {
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
                if right_stick_x.abs() > 0.1 {
                    tank.facing.x = right_stick_x;
                }
                if right_stick_y.abs() > 0.1 {
                    tank.facing.y = right_stick_y;
                }
                tank.facing = tank.facing.normalize();
                let right_stick_angle = f32::atan2(tank.facing.y, tank.facing.x);
                transform.rotation = Quat::from_rotation_z(right_stick_angle);

                if button_inputs
                    .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::RightTrigger))
                {
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
                        Collider::ball(10.0),
                        Restitution::coefficient(0.7),
                        Velocity::linear(1500.0 * tank.facing),
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
    mut query_tank: Query<(Entity, &mut Tank)>,
    query_bullet: Query<(Entity, &Bullet)>,
    mut commands: Commands,
) {
    for (entity_tank, mut tank) in &mut query_tank {
        for (entity_bullet, bullet) in &query_bullet {
            if bullet.source == entity_tank {
                continue;
            }
            if rapier_context.intersection_pair(entity_tank, entity_bullet) == Some(true) {
                commands.entity(entity_bullet).despawn();
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
                commands.entity(entity_tank).add_child(child);
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
