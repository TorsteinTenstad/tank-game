//! Shows handling of gamepad input, connections, and disconnections.
use bevy::{
    input::gamepad::{AxisSettings, GamepadButton, GamepadSettings},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, startup)
        .add_systems(Update, controller_setup)
        .add_systems(Update, update)
        .run();
}

#[derive(Component)]
struct Tank {
    id: usize,
    movement_speed: f32,
}

impl Tank {
    const DEFAULT_MOVE_SPEED: f32 = 500.0;
    const DASH_MOVE_SPEED: f32 = 2000.0;
    const SPEED_FALLOFF: f32 = (Tank::DASH_MOVE_SPEED - Tank::DEFAULT_MOVE_SPEED) / 0.2;
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
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
            material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
            transform: Transform::from_translation(Vec3::new(150., 0., 0.)),
            ..default()
        },
        Tank {
            id: 0,
            movement_speed: 0.0,
        },
    ));
}

fn update(
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut query: Query<(&mut Tank, &mut Transform)>,
) {
    for (mut tank, mut transform) in &mut query {
        tank.movement_speed = (tank.movement_speed - Tank::SPEED_FALLOFF * time.delta_seconds())
            .clamp(Tank::DEFAULT_MOVE_SPEED, Tank::DASH_MOVE_SPEED);
        match gamepads.iter().find(|&x| x.id == tank.id) {
            Some(gamepad) => {
                transform.translation.x += tank.movement_speed
                    * time.delta_seconds()
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
                        .unwrap();
                transform.translation.y += tank.movement_speed
                    * time.delta_seconds()
                    * axes
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
                        .unwrap();
                if button_inputs.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::East))
                {
                    tank.movement_speed = Tank::DASH_MOVE_SPEED;
                }
            }

            None => warn!(
                "No corresponding gamepad found for tank with id {:?}",
                tank.id
            ),
        }
    }
}
