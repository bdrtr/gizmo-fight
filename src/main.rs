use gizmo::egui;
use gizmo::prelude::*;

pub struct DamageEvent {
    pub target: u32,
    pub attacker: u32,
    pub damage: f32,
    pub hit_stun_time: f32,
    pub knockback_y: f32,
    pub pushback_x: f32,
    pub is_blocked: bool,
    pub hit_type: u32,
}

#[derive(Clone, Debug)]
pub struct FighterInput {
    pub dx: f32,
    pub punch: bool,
    pub kick: bool,
    pub jump: bool,
    pub crouch: bool,
}

impl Default for FighterInput {
    fn default() -> Self {
        Self {
            dx: 0.0,
            punch: false,
            kick: false,
            jump: false,
            crouch: false,
        }
    }
}

gizmo::core::impl_component!(FighterInput);

#[derive(Clone, PartialEq, Eq, Debug)]
enum FighterState {
    Idle,
    Walking,
    Crouching,
    Punching,
    CrouchPunching,
    LowKicking,
    JumpKicking,
    StandingKick,
    HitStun,
    Knockdown,
}

#[derive(Clone)]
struct Fighter {
    pub player_id: u8,
    pub health: f32,
    pub state: FighterState,
    pub state_timer: f32,
    pub facing_right: bool,
    pub velocity_y: f32,
    pub velocity_x: f32,
    pub combo_count: u32,
    pub combo_timer: f32,
    pub is_blocking: bool,
    pub has_hit: bool,
}
gizmo::core::impl_component!(Fighter);

#[derive(Clone)]
struct AiController {
    pub timer: f32,
    pub action: u32,
}
gizmo::core::impl_component!(AiController);

impl Default for AiController {
    fn default() -> Self {
        Self {
            timer: 0.0,
            action: 0,
        }
    }
}

#[derive(Clone)]
struct Particle {
    pub velocity: Vec3,
    pub timer: f32,
}
gizmo::core::impl_component!(Particle);

// (dead code warning'i önler)

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum GamePhase {
    Menu,
    Playing,
    Paused,
    RoundOver,
    GameOver,
}

struct GameAssets {
    spark_mesh: gizmo::core::asset::Handle<Mesh>,
    spark_mat: gizmo::core::asset::Handle<Material>,
}

struct RoundState {
    p1_wins: u8,
    p2_wins: u8,
    round: u8,
    round_timer: f32,
    round_over_timer: f32,
    p1_display_health: f32,
    p2_display_health: f32,
    needs_reset: bool,
}

struct CombatFeedback {
    camera_shake: f32,
    hit_stop: f32,
}

struct RngState {
    seed: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(usize)]
enum AnimIndex {
    Idle = 0,
    WalkBack = 1,
    RunToEnemy = 2,
    DirectPunch = 3,
    ElbowPunch = 4,
    CrouchPunch = 5,
    Jump = 6,
    KickUp = 7,
    EvadeKick = 8,
    BodyBlock = 9,
    Damaged = 10,
    Dead = 11,
    StandUp = 12,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
enum AiAction {
    Idle = 0,
    MoveForward = 1,
    MoveBack = 2,
    Punch = 3,
    Kick = 4,
    CrouchKick = 5,
    JumpInPlace = 6,
    RunJump = 8,
}

impl AiAction {
    fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::MoveForward,
            2 => Self::MoveBack,
            3 => Self::Punch,
            4 => Self::Kick,
            5 => Self::CrouchKick,
            6 => Self::JumpInPlace,
            8 => Self::RunJump,
            _ => Self::Idle,
        }
    }
}

// spawn_gltf_node — değişmedi, doğru görünüyor
fn spawn_gltf_node(
    world: &mut World,
    parent_entity: Option<gizmo::core::Entity>,
    node_data: &gizmo::renderer::asset::loaders::GltfNodeData,
    fallback_mat: &gizmo::core::asset::Handle<Material>,
    skeleton_opt: Option<&gizmo::renderer::components::Skeleton>,
) -> gizmo::core::Entity {
    let ent = world.spawn();

    if let Some(parent) = parent_entity {
        world.add_component(ent, gizmo::core::component::Parent(parent.id()));
    }

    let is_armature = node_data.name.as_deref() == Some("Armature");

    let mut trans = if is_armature {
        // Armature transform (scale=0.01, rot=90°X) is already baked into
        // the skeleton's root_transform. Applying it here too would double it.
        // Use identity so only the skeleton pipeline applies it.
        Transform::default()
    } else {
        let mut t = Transform::new(node_data.translation.into());
        t.rotation = gizmo::math::Quat::from_array(node_data.rotation);
        t.scale = Vec3::new(node_data.scale[0], node_data.scale[1], node_data.scale[2]);
        t
    };
    trans.update_local_matrix();

    world.add_component(ent, trans);
    world.add_component(ent, gizmo::physics::GlobalTransform::default());
    if let Some(name) = &node_data.name {
        world.add_component(ent, gizmo::core::EntityName(name.clone()));
    }

    let mut children_ids = Vec::new();

    if !node_data.primitives.is_empty() {
        let (mesh, opt_mat) = &node_data.primitives[0];

        let mesh_h = world
            .get_resource_mut::<gizmo::core::asset::Assets<Mesh>>()
            .unwrap()
            .add(mesh.clone());
        let mat_h = if let Some(mat) = opt_mat {
            let mut modified_mat = mat.clone();
            modified_mat.metallic = 0.0; // Kötü yansımaları engelle
            modified_mat.roughness = modified_mat.roughness.max(0.7); // Parlamayı azalt
            world
                .get_resource_mut::<gizmo::core::asset::Assets<Material>>()
                .unwrap()
                .add(modified_mat)
        } else {
            fallback_mat.clone()
        };
        world.add_component(ent, mesh_h);
        world.add_component(ent, mat_h);
        world.add_component(ent, gizmo::renderer::components::MeshRenderer::new());
        if let Some(skel) = skeleton_opt {
            world.add_component(ent, skel.clone());
        }

        for (i, (extra_mesh, extra_opt_mat)) in node_data.primitives.iter().enumerate().skip(1) {
            let extra_ent = world.spawn();
            world.add_component(extra_ent, Transform::default());
            world.add_component(extra_ent, gizmo::physics::GlobalTransform::default());
            world.add_component(extra_ent, gizmo::core::component::Parent(ent.id()));
            if let Some(name) = &node_data.name {
                world.add_component(
                    extra_ent,
                    gizmo::core::EntityName(format!("{}_prim{}", name, i)),
                );
            }
            let extra_mesh_h = world
                .get_resource_mut::<gizmo::core::asset::Assets<Mesh>>()
                .unwrap()
                .add(extra_mesh.clone());
            let extra_mat_h = if let Some(mat) = extra_opt_mat {
                let mut modified_mat = mat.clone();
                modified_mat.metallic = 0.0;
                modified_mat.roughness = modified_mat.roughness.max(0.7);
                world
                    .get_resource_mut::<gizmo::core::asset::Assets<Material>>()
                    .unwrap()
                    .add(modified_mat)
            } else {
                fallback_mat.clone()
            };
            world.add_component(extra_ent, extra_mesh_h);
            world.add_component(extra_ent, extra_mat_h);
            world.add_component(extra_ent, gizmo::renderer::components::MeshRenderer::new());
            if let Some(skel) = skeleton_opt {
                world.add_component(extra_ent, skel.clone());
            }
            children_ids.push(extra_ent.id());
        }
    }

    for child in &node_data.children {
        let child_ent = spawn_gltf_node(world, Some(ent), child, fallback_mat, skeleton_opt);
        children_ids.push(child_ent.id());
    }

    if !children_ids.is_empty() {
        world.add_component(ent, gizmo::core::component::Children(children_ids));
    }

    ent
}

fn spawn_city_environment(
    world: &mut World,
    mat_h: gizmo::core::asset::Handle<Material>,
    sunset_mat_h: gizmo::core::asset::Handle<Material>,
    ground_mesh: gizmo::core::asset::Handle<Mesh>,
) {
    // ── Tekken-style Arena Platform ──
    // Ana zemin: geniş, düz, koyu renkli platform
    let ground = world.spawn_bundle(
        gizmo::prelude::MeshBundle::new(ground_mesh.clone(), mat_h.clone())
            .at(Vec3::new(0.0, -0.05, 0.0))
            .with_scale(Vec3::new(20.0, 0.1, 12.0)),
    );
    world.add_component(ground, gizmo::core::EntityName("ArenaFloor".to_string()));

    // Arka plan duvarı — dramatik gün batımı backdrop
    let sky = world.spawn_bundle(
        gizmo::prelude::MeshBundle::new(ground_mesh.clone(), sunset_mat_h.clone())
            .at(Vec3::new(0.0, 15.0, -25.0))
            .with_scale(Vec3::new(80.0, 40.0, 1.0)),
    );
    world.add_component(sky, gizmo::core::EntityName("SunsetSky".to_string()));

    // Gün batımı (Sunset) Ana Işığı - Ufuktan gelen güçlü turuncu/kızıl ışık
    let sun_light = world.spawn_bundle(gizmo::prelude::PointLightBundle {
        position: Vec3::new(-15.0, 3.0, -10.0), // Arkadan ve aşağıdan vurur
        color: Vec3::new(1.0, 0.35, 0.05),      // Ateşli turuncu
        intensity: 2500.0,
        radius: 150.0,
        ..Default::default()
    });
    world.add_component(sun_light, gizmo::core::EntityName("SunsetMain".to_string()));

    // Ortam (Fill) Işığı - Gölgeleri yumuşatan morumsu akşam ışığı
    let rim_light = world.spawn_bundle(gizmo::prelude::PointLightBundle {
        position: Vec3::new(10.0, 12.0, 10.0), // Karşı çaprazdan
        color: Vec3::new(0.3, 0.15, 0.6),      // Koyu mor/mavi
        intensity: 1200.0,
        radius: 100.0,
        ..Default::default()
    });
    world.add_component(rim_light, gizmo::core::EntityName("CoolRim".to_string()));
}

fn spawn_fighter(
    world: &mut World,
    renderer: &gizmo::renderer::Renderer,
    pos: Vec3,
    is_p1: bool,
    mesh_h: gizmo::core::asset::Handle<Mesh>,
    mat_h: gizmo::core::asset::Handle<Material>,
    gltf_scene: Option<&gizmo::renderer::asset::loaders::GltfSceneAsset>,
    animations: std::sync::Arc<[gizmo::renderer::animation::AnimationClip]>,
) -> gizmo::core::Entity {
    let root = world.spawn();
    let mut trans = Transform::new(pos);
    trans.scale = Vec3::new(1.0, 1.0, 1.0);
    trans.update_local_matrix();
    world.add_component(root, trans);
    world.add_component(root, gizmo::physics::GlobalTransform::default());
    world.add_component(
        root,
        gizmo::core::EntityName(if is_p1 {
            "Player1".to_string()
        } else {
            "Player2".to_string()
        }),
    );

    if let Some(scene) = gltf_scene {
        let model_root = world.spawn();
        let mut m_trans = Transform::new(Vec3::ZERO);
        m_trans.scale = Vec3::new(1.0, 1.0, 1.0);
        m_trans.update_local_matrix();
        world.add_component(model_root, m_trans);
        world.add_component(model_root, gizmo::physics::GlobalTransform::default());
        world.add_component(model_root, gizmo::core::component::Parent(root.id()));

        let mut skeleton_opt = None;
        if !scene.skeletons.is_empty() {
            let skeleton =
                renderer.create_skeleton(std::sync::Arc::new(scene.skeletons[0].clone()));
            world.add_component(model_root, skeleton.clone());
            skeleton_opt = Some(skeleton);

            if !animations.is_empty() {
                let mut player = gizmo::renderer::components::AnimationPlayer::default();
                player.animations = animations.clone();
                player.active_animation = AnimIndex::Idle as usize;
                player.loop_anim = true;
                world.add_component(model_root, player);
            }
        }

        let mut child_ids = Vec::new();
        for root_node in &scene.roots {
            let child_ent = spawn_gltf_node(
                world,
                Some(model_root),
                root_node,
                &mat_h,
                skeleton_opt.as_ref(),
            );
            child_ids.push(child_ent.id());
        }
        if !child_ids.is_empty() {
            world.add_component(model_root, gizmo::core::component::Children(child_ids));
        }
        world.add_component(
            root,
            gizmo::core::component::Children(vec![model_root.id()]),
        );
    } else {
        let mesh_ent = world.spawn_bundle(
            gizmo::prelude::MeshBundle::new(mesh_h, mat_h)
                .at(Vec3::new(0.0, 2.0, 0.0))
                .with_scale(Vec3::new(1.0, 4.0, 1.0)),
        );
        world.add_component(mesh_ent, gizmo::core::component::Parent(root.id()));
        world.add_component(root, gizmo::core::component::Children(vec![mesh_ent.id()]));
    }

    world.add_component(
        root,
        Fighter {
            player_id: if is_p1 { 1 } else { 2 },
            health: 100.0,
            state: FighterState::Idle,
            state_timer: 0.0,
            combo_count: 0,
            combo_timer: 0.0,
            velocity_y: 0.0,
            velocity_x: 0.0,
            is_blocking: false,
            has_hit: false,
            facing_right: is_p1,
        },
    );
    world.add_component(root, FighterInput::default());

    root
}

fn setup(world: &mut World, renderer: &Renderer) {
    // ── Directional Sun ──
    world.spawn_bundle(DirectionalLightBundle {
        rotation: gizmo::math::Quat::from_rotation_y(-0.4)
            * gizmo::math::Quat::from_rotation_x(-0.7),
        color: Vec3::new(1.0, 0.95, 0.88),
        intensity: 0.7,
        role: gizmo::renderer::components::LightRole::Sun,
    });

    // ── Tekken-style Camera ──
    // Yandan bakış, hafif yukarıdan, dövüşçüleri çerçeveleyen açı
    world.spawn_bundle(CameraBundle {
        position: Vec3::new(0.0, 1.8, 5.5),
        yaw: -std::f32::consts::FRAC_PI_2,
        pitch: -0.1, // hafif aşağı bakış
        ..Default::default()
    });

    let quad_mesh = renderer.create_plane(1.0);
    let cube_mesh = renderer.create_cube();

    let (_quad_handle, cube_handle) = {
        let mut meshes = world
            .get_resource_mut::<gizmo::core::asset::Assets<Mesh>>()
            .unwrap();
        (meshes.add(quad_mesh), meshes.add(cube_mesh))
    };

    let white_tex = renderer.create_white_texture();
    let checkerboard_tex = renderer.create_checkerboard_texture();

    let p1_mat = Material::new(white_tex.clone()).with_pbr(Vec4::new(0.2, 0.5, 1.0, 1.0), 0.8, 0.0);
    let p2_mat = Material::new(white_tex.clone()).with_pbr(Vec4::new(1.0, 0.2, 0.2, 1.0), 0.8, 0.0);
    // Volkanik taş zemin tekstürü
    let floor_tex = renderer
        .load_texture("assets/arena_floor.jpg")
        .unwrap_or_else(|e| {
            println!("Failed to load arena_floor.jpg: {}", e);
            checkerboard_tex.clone()
        });
    let ground_mat = Material::new(floor_tex).with_pbr(Vec4::new(0.8, 0.8, 0.8, 1.0), 0.95, 0.05);
    let city_mat =
        Material::new(white_tex.clone()).with_pbr(Vec4::new(0.5, 0.5, 0.55, 1.0), 0.8, 0.1);
    let spark_mat =
        Material::new(white_tex.clone()).with_pbr(Vec4::new(1.0, 0.8, 0.2, 1.0), 0.2, 0.8);

    // Tekken-style arena backdrop
    let backdrop_tex = renderer
        .load_texture("assets/arena_backdrop.jpg")
        .unwrap_or_else(|e| {
            println!("Failed to load arena_backdrop.jpg: {}", e);
            white_tex.clone()
        });
    let sunset_mat = Material::new(backdrop_tex)
        .with_unlit(Vec4::new(1.0, 1.0, 1.0, 1.0))
        .with_double_sided(true);

    let (p1_mat_h, p2_mat_h, ground_mat_h, _city_mat_h, spark_mat_h, sunset_mat_h) = {
        let mut materials = world
            .get_resource_mut::<gizmo::core::asset::Assets<Material>>()
            .unwrap();
        (
            materials.add(p1_mat),
            materials.add(p2_mat),
            materials.add(ground_mat),
            materials.add(city_mat),
            materials.add(spark_mat),
            materials.add(sunset_mat),
        )
    };

    let gltf_scene = match renderer.load_gltf("assets/main_char.glb") {
        Ok(scene) => Some(scene),
        Err(e) => {
            eprintln!("GLB FAILED: {e}");
            None
        }
    };
    spawn_city_environment(
        world,
        ground_mat_h.clone(), // Use ground material for the ground
        sunset_mat_h.clone(),
        cube_handle.clone(),
    );

    // Harici Animasyonları Yükleme
    let mut all_anims: Vec<gizmo::renderer::animation::AnimationClip> = Vec::new();

    let apply_fixup = |mut anim: gizmo::renderer::animation::AnimationClip| -> gizmo::renderer::animation::AnimationClip {
        let fixup = gizmo::math::Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        for track in &mut anim.rotations {
            if track.target_node_name.as_deref().is_some_and(|n| n.contains("Hips")) {
                for key in &mut track.keyframes {
                    key.value = (fixup * key.value).normalize();
                }
            }
        }
        let bind_translation = gizmo::math::Vec3::new(0.0, 3.985525, -96.3794);
        for track in &mut anim.translations {
            if track.target_node_name.as_deref().is_some_and(|n| n.contains("Hips")) {
                if let Some(first_key) = track.keyframes.first().map(|k| k.value) {
                    for key in &mut track.keyframes {
                        let delta = key.value - first_key;
                        let fixed_delta = fixup * (delta * 100.0);
                        key.value = bind_translation + fixed_delta;
                    }
                }
            }
        }
        anim
    };

    // 0: Idle / Base Anim (Kullanıcı isteği: Guard animasyonu base olsun)
    let mut base_anim = None;
    if let Ok(scene) = renderer.load_gltf("assets/Center Block.glb") {
        let best_anim = scene
            .animations
            .iter()
            .max_by_key(|a| a.rotations.len() + a.translations.len() + a.scales.len());
        if let Some(anim) = best_anim {
            if !anim.rotations.is_empty() || !anim.translations.is_empty() {
                base_anim = Some(apply_fixup(anim.clone()));
            }
        }
    }

    if let Some(anim) = base_anim {
        all_anims.push(anim);
    } else if let Some(ref scene) = gltf_scene {
        // Center Block yüklenemezse main_char.glb'ye geri dön (T-Pose)
        all_anims.push(scene.animations[0].clone());
    } else {
        panic!("Neither Center Block nor main_char.glb loaded!");
    }

    let mut load_external_anim = |path: &str| {
        if let Ok(scene) = renderer.load_gltf(path) {
            let best_anim = scene
                .animations
                .iter()
                .max_by_key(|a| a.rotations.len() + a.translations.len() + a.scales.len());
            if let Some(anim) = best_anim {
                if !anim.rotations.is_empty() || !anim.translations.is_empty() {
                    all_anims.push(apply_fixup(anim.clone()));
                    return;
                }
            }
        }
        all_anims.push(all_anims[0].clone()); // Yüklenemezse veya boşsa Idle yap (fallback)
    };

    load_external_anim("assets/Short Left Side Step.glb"); // 1: WalkBack
    load_external_anim("assets/Jogging.glb"); // 2: RunToEnemy
    load_external_anim("assets/Mutant Punch.glb"); // 3: DirectPunch
    load_external_anim("assets/Illegal Elbow Punch.glb"); // 4: ElbowPunch
    load_external_anim("assets/Kidney Hit.glb"); // 5: CrouchPunch
    load_external_anim("assets/Jump.glb"); // 6: Jump
    load_external_anim("assets/Capoeira.glb"); // 7: KickUp
    load_external_anim("assets/Capoeira.glb"); // 8: EvadeKick
    load_external_anim("assets/Body Block.glb"); // 9: BodyBlock
    load_external_anim("assets/Reaction.glb"); // 10: Damaged
    load_external_anim("assets/Dying.glb"); // 11: Dead
    load_external_anim("assets/Standing Up.glb"); // 12: StandUp

    let anims: std::sync::Arc<[gizmo::renderer::animation::AnimationClip]> =
        std::sync::Arc::from(all_anims.into_boxed_slice());

    let _p1 = spawn_fighter(
        world,
        renderer,
        Vec3::new(-2.0, 0.0, 0.0),
        true,
        cube_handle.clone(),
        p1_mat_h.clone(),
        gltf_scene.as_ref(),
        anims.clone(),
    );

    let p2_ent = spawn_fighter(
        world,
        renderer,
        Vec3::new(2.0, 0.0, 0.0),
        false,
        cube_handle.clone(),
        p2_mat_h.clone(),
        gltf_scene.as_ref(),
        anims.clone(),
    );
    world.add_component(p2_ent, AiController::default());

    world.insert_resource(GameAssets {
        spark_mesh: cube_handle.clone(),
        spark_mat: spark_mat_h.clone(),
    });

    // Debug küpler — artık gerek yok, karakterler görünüyor ✓
    // let debug_cube = world.spawn_bundle(
    //     gizmo::prelude::MeshBundle::new(cube_handle.clone(), p1_mat_h.clone())
    //         .at(Vec3::new(0.0, 0.0, 0.0)).with_scale(Vec3::splat(0.5)),
    // );

    world.insert_resource(State::new(GamePhase::Menu));
    world.insert_resource(RoundState {
        p1_wins: 0,
        p2_wins: 0,
        round: 1,
        round_timer: 99.0,
        round_over_timer: 0.0,
        p1_display_health: 100.0,
        p2_display_health: 100.0,
        needs_reset: false,
    });
    world.insert_resource(CombatFeedback {
        camera_shake: 0.0,
        hit_stop: 0.0,
    });
    world.insert_resource(RngState { seed: 1337 });
}

// AI Sistemi
struct AiInput {
    dx: f32,
    punch: bool,
    kick: bool,
    jump: bool,
    crouch: bool,
}

fn compute_ai_input(
    ai: &mut AiController,
    fighter: &Fighter,
    distance_3d: f32,
    is_grounded: bool,
    rng_seed: &mut u32,
    dt: f32,
) -> AiInput {
    ai.timer -= dt;

    *rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let rand_val = (*rng_seed as f32) / (u32::MAX as f32);

    if ai.timer <= 0.0 {
        ai.timer = 0.1 + (rand_val * 0.2); // Daha hızlı refleks

        if distance_3d > 2.5 {
            ai.action = if rand_val < 0.80 {
                AiAction::MoveForward as u32
            } else if rand_val < 0.95 {
                AiAction::RunJump as u32
            } else {
                AiAction::Idle as u32
            };
        } else if distance_3d > 1.6 {
            ai.action = if rand_val < 0.60 {
                AiAction::MoveForward as u32
            } else if rand_val < 0.75 {
                AiAction::JumpInPlace as u32
            } else if rand_val < 0.90 {
                AiAction::Kick as u32
            } else {
                AiAction::MoveBack as u32
            };
        } else {
            ai.action = if rand_val < 0.15 {
                AiAction::MoveForward as u32
            } else if rand_val < 0.45 {
                AiAction::Punch as u32
            } else if rand_val < 0.70 {
                AiAction::Kick as u32
            } else if rand_val < 0.85 {
                AiAction::CrouchKick as u32
            } else if rand_val < 0.95 {
                AiAction::MoveBack as u32
            } else {
                AiAction::Idle as u32
            };
        }
    }

    let mut input = AiInput {
        dx: 0.0,
        punch: false,
        kick: false,
        jump: false,
        crouch: false,
    };

    match AiAction::from_u32(ai.action) {
        AiAction::MoveForward => input.dx = if fighter.facing_right { 1.0 } else { -1.0 },
        AiAction::MoveBack => input.dx = if fighter.facing_right { -1.0 } else { 1.0 },
        AiAction::Punch => input.punch = true,
        AiAction::Kick => input.kick = true,
        AiAction::CrouchKick => {
            input.crouch = true;
            input.kick = true;
        }
        AiAction::JumpInPlace => input.jump = true,
        AiAction::RunJump => {
            input.dx = if fighter.facing_right { 1.0 } else { -1.0 };
            input.jump = true;
        }
        AiAction::Idle => {}
    }

    // Havadayken vurma şansı
    if !is_grounded
        && matches!(
            AiAction::from_u32(ai.action),
            AiAction::JumpInPlace | AiAction::RunJump
        )
    {
        *rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        if (*rng_seed % 100) < 30 {
            input.kick = true;
        }
    }

    input
}

fn state_transition_system(mut state: ResMut<State<GamePhase>>) {
    state.apply_transitions();
}

fn sync_health_system(mut round_state: ResMut<RoundState>, q: Query<&Fighter>, dt: f32) {
    let mut p1_hp = 100.0f32;
    let mut p2_hp = 100.0f32;

    for (_, fighter) in q.iter() {
        if fighter.player_id == 1 {
            p1_hp = fighter.health;
        }
        if fighter.player_id == 2 {
            p2_hp = fighter.health;
        }
    }

    round_state.p1_display_health += (p1_hp - round_state.p1_display_health) * 5.0 * dt;
    round_state.p2_display_health += (p2_hp - round_state.p2_display_health) * 5.0 * dt;
}

fn global_input_system(input: Res<Input>, mut phase_state: ResMut<State<GamePhase>>) {
    let current_phase = *phase_state.get();
    if input.is_key_just_pressed(KeyCode::Escape as u32) {
        match current_phase {
            GamePhase::Playing => phase_state.set(GamePhase::Paused),
            GamePhase::Paused => phase_state.set(GamePhase::Playing),
            _ => {}
        }
    }
}

fn menu_input_system(
    input: Res<Input>,
    mut phase_state: ResMut<State<GamePhase>>,
    mut round_state: ResMut<RoundState>,
) {
    if input.is_key_just_pressed(KeyCode::Enter as u32)
        || input.is_key_just_pressed(KeyCode::Space as u32)
    {
        phase_state.set(GamePhase::Playing);
        round_state.needs_reset = true;
    }
}

fn round_over_system(
    mut status: ResMut<RoundState>,
    mut phase_state: ResMut<State<GamePhase>>,
    mut q_fighters: Query<(Mut<Fighter>, Mut<Transform>)>,
    dt: f32,
) {
    status.round_over_timer -= dt;
    if status.round_over_timer <= 0.0 {
        if status.p1_wins >= 2 || status.p2_wins >= 2 {
            phase_state.set(GamePhase::GameOver);
        } else {
            status.round += 1;
            status.round_timer = 99.0;
            phase_state.set(GamePhase::Playing);

            for (_, (mut f, mut trans)) in q_fighters.iter_mut() {
                f.health = 100.0;
                f.state = FighterState::Idle;
                f.velocity_y = 0.0;
                f.velocity_x = 0.0;
                trans.position.y = 0.0;
                trans.set_scale(Vec3::new(1.0, 1.0, 1.0));
                trans.position.x = if f.player_id == 1 { -5.0 } else { 5.0 };
            }
        }
    }
}

fn round_timer_system(
    mut round_state: ResMut<RoundState>,
    mut phase_state: ResMut<State<GamePhase>>,
    q_fighters: Query<&Fighter>,
    dt: f32,
) {
    round_state.round_timer = (round_state.round_timer - dt).max(0.0);
    if round_state.round_timer == 0.0 {
        phase_state.set(GamePhase::RoundOver);
        round_state.round_over_timer = 3.0;

        let mut p1_hp = 0.0f32;
        let mut p2_hp = 0.0f32;
        for (_, f) in q_fighters.iter() {
            if f.player_id == 1 {
                p1_hp = f.health;
            } else {
                p2_hp = f.health;
            }
        }

        if p1_hp > p2_hp {
            round_state.p1_wins += 1;
        } else if p2_hp > p1_hp {
            round_state.p2_wins += 1;
        } else {
            round_state.p1_wins += 1;
        } // Berabere → P1 kazanır
    }
}

fn ai_input_system(
    mut rng_state: ResMut<RngState>,
    mut q_ai: Query<(Mut<AiController>, Mut<FighterInput>, &Fighter, &Transform)>,
    q_fighters: Query<(&Fighter, &Transform)>,
    feedback: Res<CombatFeedback>,
    dt: f32,
) {
    let mut p1_pos = Vec3::ZERO;
    let mut p2_pos = Vec3::ZERO;

    for (_, (fighter, trans)) in q_fighters.iter() {
        if fighter.player_id == 1 {
            p1_pos = trans.position;
        }
        if fighter.player_id == 2 {
            p2_pos = trans.position;
        }
    }

    let distance_3d = (p1_pos - p2_pos).length();
    let fighter_dt = if feedback.hit_stop > 0.0 { 0.0 } else { dt };

    for (_, (mut ai, mut input, fighter, trans)) in q_ai.iter_mut() {
        let is_grounded = trans.position.y <= 0.0;
        let ai_input = compute_ai_input(
            &mut ai,
            fighter,
            distance_3d,
            is_grounded,
            &mut rng_state.seed,
            fighter_dt,
        );
        input.dx = ai_input.dx;
        input.punch = ai_input.punch;
        input.kick = ai_input.kick;
        input.jump = ai_input.jump;
        input.crouch = ai_input.crouch;
    }
}

fn player_input_system(input: Res<Input>, mut q_fighters: Query<(&Fighter, Mut<FighterInput>)>) {
    for (_, (fighter, mut f_in)) in q_fighters.iter_mut() {
        // Sadece AI bileşeni olmayan fighter'lar player girişi alır
        if fighter.player_id == 2 {
            continue;
        }

        let mut dx = 0.0f32;
        let (crouch, jump, punch, kick) = if fighter.player_id == 1 {
            if input.is_key_pressed(KeyCode::KeyA as u32) {
                dx -= 1.0;
            }
            if input.is_key_pressed(KeyCode::KeyD as u32) {
                dx += 1.0;
            }
            (
                input.is_key_pressed(KeyCode::KeyS as u32),
                input.is_key_just_pressed(KeyCode::KeyW as u32),
                input.is_key_just_pressed(KeyCode::KeyJ as u32),
                input.is_key_just_pressed(KeyCode::KeyK as u32),
            )
        } else {
            if input.is_key_pressed(KeyCode::ArrowLeft as u32) {
                dx -= 1.0;
            }
            if input.is_key_pressed(KeyCode::ArrowRight as u32) {
                dx += 1.0;
            }
            (
                input.is_key_pressed(KeyCode::ArrowDown as u32),
                input.is_key_just_pressed(KeyCode::ArrowUp as u32),
                input.is_key_just_pressed(KeyCode::KeyM as u32),
                input.is_key_just_pressed(KeyCode::Comma as u32),
            )
        };

        f_in.dx = dx;
        f_in.crouch = crouch;
        f_in.jump = jump;
        f_in.punch = punch;
        f_in.kick = kick;
    }
}

fn fighter_state_system(
    mut q_fighters: Query<(&FighterInput, Mut<Fighter>)>,
    mut feedback: ResMut<CombatFeedback>,
    dt: f32,
) {
    if feedback.hit_stop > 0.0 {
        feedback.hit_stop = (feedback.hit_stop - dt).max(0.0);
    }
    let fighter_dt = if feedback.hit_stop > 0.0 { 0.0 } else { dt };

    for (_, (input, mut fighter)) in q_fighters.iter_mut() {
        // Combo timer güncelleme
        if fighter.combo_timer > 0.0 {
            fighter.combo_timer = (fighter.combo_timer - fighter_dt).max(0.0);
            if fighter.combo_timer == 0.0 {
                fighter.combo_count = 0;
            }
        }

        // State timer güncelleme
        if fighter.state_timer > 0.0 {
            fighter.state_timer -= fighter_dt;
            if fighter.state_timer <= 0.0 {
                // Eskisi: if Knockdown || state != HitStun → reset
                // Yenisi: Her zaman Idle'a dön (timer bitti demek hareket bitti)
                // HitStun özel durumu aşağıda zaten ele alınıyor.
                fighter.state = FighterState::Idle;
            }
        }

        // HitStun bitti mi?
        if fighter.state == FighterState::HitStun && fighter.state_timer <= 0.0 {
            fighter.state = FighterState::Idle;
        }

        let is_attacking = matches!(
            fighter.state,
            FighterState::Punching
                | FighterState::LowKicking
                | FighterState::JumpKicking
                | FighterState::StandingKick
                | FighterState::CrouchPunching
        );
        let can_act = !matches!(
            fighter.state,
            FighterState::HitStun | FighterState::Knockdown
        );
        let is_grounded = fighter.velocity_y == 0.0;

        let mut dx = input.dx;
        fighter.is_blocking = is_grounded
            && ((fighter.facing_right && dx < 0.0) || (!fighter.facing_right && dx > 0.0));

        if input.crouch && is_grounded {
            dx = 0.0;
        }

        // Animasyon bitimine yakınken (cancel window) yeni saldırı kabul et
        let can_cancel = is_attacking && fighter.state_timer < 0.15;

        if can_act && (!is_attacking || can_cancel) {
            if input.kick && !is_grounded {
                fighter.state = FighterState::JumpKicking;
                fighter.state_timer = 0.4;
                fighter.has_hit = false;
            } else if input.kick && input.crouch && is_grounded {
                fighter.state = FighterState::LowKicking;
                fighter.state_timer = 0.35;
                fighter.has_hit = false;
            } else if input.punch && input.crouch && is_grounded {
                fighter.state = FighterState::CrouchPunching;
                fighter.state_timer = 0.3;
                fighter.has_hit = false;
            } else if input.kick && is_grounded {
                fighter.state = FighterState::StandingKick;
                fighter.state_timer = 0.35;
                fighter.has_hit = false;
            } else if input.punch {
                fighter.state = FighterState::Punching;
                fighter.state_timer = 0.25;
                fighter.has_hit = false;
                if fighter.combo_timer > 0.0 {
                    fighter.combo_count += 1;
                } else {
                    fighter.combo_count = 1;
                }
                fighter.combo_timer = 0.8;
                if fighter.combo_count > 4 {
                    fighter.combo_count = 1;
                }
            } else if !is_attacking {
                // Sadece normal durumda hareket
                if input.jump && is_grounded {
                    fighter.velocity_y = 16.0;
                    fighter.state = FighterState::Idle;
                } else if input.crouch && is_grounded {
                    fighter.state = FighterState::Crouching;
                } else if dx != 0.0 && is_grounded {
                    fighter.state = FighterState::Walking;
                    fighter.velocity_x = dx * 3.8;
                } else if is_grounded {
                    fighter.state = FighterState::Idle;
                }
            }
        }
    }
}

fn fighter_movement_system(
    mut q_fighters: Query<(Mut<Fighter>, Mut<Transform>)>,
    feedback: Res<CombatFeedback>,
    dt: f32,
) {
    let fighter_dt = if feedback.hit_stop > 0.0 { 0.0 } else { dt };
    const GRAVITY: f32 = -35.0;
    const GROUND_Y: f32 = 0.0;

    let mut p1_pos = Vec3::ZERO;
    let mut p2_pos = Vec3::ZERO;

    // İlk geçiş: pozisyonları oku
    for (_, (fighter, trans)) in q_fighters.iter_mut() {
        if fighter.player_id == 1 {
            p1_pos = trans.position;
        }
        if fighter.player_id == 2 {
            p2_pos = trans.position;
        }
    }

    // İkinci geçiş: güncelle
    for (_, (mut fighter, mut trans)) in q_fighters.iter_mut() {
        fighter.facing_right = if fighter.player_id == 1 {
            p1_pos.x < p2_pos.x
        } else {
            p2_pos.x < p1_pos.x
        };

        let target_yaw = if fighter.facing_right {
            std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2
        };
        // Removed -90 X rotation because main_char is natively Y-Up!
        trans.rotation = gizmo::math::Quat::from_rotation_y(target_yaw);

        fighter.velocity_y += GRAVITY * fighter_dt;
        trans.position.y += fighter.velocity_y * fighter_dt;
        trans.position.x += fighter.velocity_x * fighter_dt;

        let is_grounded = trans.position.y <= GROUND_Y;
        let friction = if is_grounded { 25.0 } else { 8.0 };
        if fighter.state != FighterState::Walking {
            if fighter.velocity_x > 0.0 {
                fighter.velocity_x = (fighter.velocity_x - friction * fighter_dt).max(0.0);
            } else if fighter.velocity_x < 0.0 {
                fighter.velocity_x = (fighter.velocity_x + friction * fighter_dt).min(0.0);
            }
        }

        if is_grounded {
            trans.position.y = GROUND_Y;
            if fighter.velocity_y < -5.0 && fighter.state == FighterState::HitStun {
                fighter.state = FighterState::Knockdown;
                fighter.state_timer = 1.0;
            }
            fighter.velocity_y = 0.0;
        }

        let other_pos = if fighter.player_id == 1 {
            p2_pos
        } else {
            p1_pos
        };
        if (trans.position.y - other_pos.y).abs() < 1.5 {
            let dx = trans.position.x - other_pos.x;
            if dx.abs() < 1.0 {
                if dx < 0.0 || (dx == 0.0 && fighter.player_id == 1) {
                    trans.position.x = other_pos.x - 1.0;
                } else {
                    trans.position.x = other_pos.x + 1.0;
                }
            }
        }
        trans.position.x = trans.position.x.clamp(-14.0, 14.0);

        trans.update_local_matrix();
    }
}

fn combat_hit_system(
    mut q_fighters: Query<(Mut<Fighter>, &Transform)>,
    mut damage_events: gizmo::core::event::EventWriter<DamageEvent>,
) {
    let mut p1_pos = Vec3::ZERO;
    let mut p2_pos = Vec3::ZERO;
    let mut p1_entity: u32 = 0;
    let mut p2_entity: u32 = 0;

    for (id, (fighter, trans)) in q_fighters.iter() {
        if fighter.player_id == 1 {
            p1_pos = trans.position;
            p1_entity = id;
        }
        if fighter.player_id == 2 {
            p2_pos = trans.position;
            p2_entity = id;
        }
    }

    let distance_3d = (p1_pos - p2_pos).length();
    let mut hit_by_1: u32 = 0;
    let mut hit_by_2: u32 = 0;

    for (_, (mut fighter, _)) in q_fighters.iter_mut() {
        let is_attacking = matches!(
            fighter.state,
            FighterState::Punching
                | FighterState::LowKicking
                | FighterState::JumpKicking
                | FighterState::StandingKick
                | FighterState::CrouchPunching
        );
        // Hit window: state_timer 0.10..0.25 arası
        if is_attacking && fighter.state_timer > 0.1 && fighter.state_timer < 0.25 && !fighter.has_hit {
            let attack_type = match fighter.state {
                FighterState::JumpKicking => 10,
                FighterState::LowKicking => 11,
                FighterState::StandingKick => 12,
                FighterState::CrouchPunching => 13,
                FighterState::Punching => fighter.combo_count,
                _ => 0,
            };

            let hit_range = match attack_type {
                10..=12 => 2.4, // Tekmelerin menzili (Uzattım)
                _ => 1.3,       // Yumrukların menzili
            };

            if distance_3d < hit_range {
                if fighter.player_id == 1 {
                    hit_by_1 = attack_type;
                } else {
                    hit_by_2 = attack_type;
                }
                fighter.has_hit = true;
            }
        }
    }

    // İkinci geçiş: Hasar uygulama
    for (id, (fighter, _)) in q_fighters.iter() {
        let hit_type = if fighter.player_id == 2 {
            hit_by_1
        } else {
            hit_by_2
        };
        if hit_type == 0 {
            continue;
        }
        if matches!(
            fighter.state,
            FighterState::HitStun | FighterState::Knockdown
        ) {
            continue;
        }

        let (damage, hit_stun_time, knockback_y) = match hit_type {
            1 => (5.0, 0.3, 0.0),    // Combo 1: Jab
            2 => (7.0, 0.35, 0.0),   // Combo 2: Cross
            3 => (12.0, 0.5, 0.0),   // Combo 3: Heavy hook
            4 => (20.0, 0.8, 12.0),  // Combo 4: Uppercut (launcher)
            10 => (15.0, 0.5, -5.0), // Jump Kick (spike)
            11 => (8.0, 0.4, 0.0),   // Low Kick
            12 => (12.0, 0.4, 0.0),  // Standing Kick
            13 => (8.0, 0.35, 0.0),  // Crouch Punch
            _ => continue,
        };

        let is_low_block = fighter.is_blocking && fighter.state == FighterState::Crouching;
        let is_high_block = fighter.is_blocking && fighter.state != FighterState::Crouching;
        let blocked = match hit_type {
            11 | 13 => is_low_block,            // Low attacks
            10 => is_high_block,                // High/Jump attacks
            _ => is_high_block || is_low_block, // Mid attacks
        };

        damage_events.send(DamageEvent {
            target: id,
            attacker: if fighter.player_id == 1 {
                p2_entity
            } else {
                p1_entity
            },
            damage,
            hit_stun_time,
            knockback_y,
            pushback_x: if fighter.facing_right { -12.0 } else { 12.0 },
            is_blocked: blocked,
            hit_type,
        });
    }
}

fn match_rules_system(
    q_fighters: Query<&Fighter>,
    mut round_state: ResMut<RoundState>,
    mut phase_state: ResMut<State<GamePhase>>,
) {
    let phase = *phase_state.get();
    if phase != GamePhase::Playing {
        return;
    }

    let mut p1_dead = false;
    let mut p2_dead = false;
    for (_, fighter) in q_fighters.iter() {
        if fighter.player_id == 1 && fighter.health <= 0.0 {
            p1_dead = true;
        }
        if fighter.player_id == 2 && fighter.health <= 0.0 {
            p2_dead = true;
        }
    }

    if p1_dead || p2_dead {
        phase_state.set(GamePhase::RoundOver);
        round_state.round_over_timer = 3.0;
        if p1_dead && p2_dead {
            round_state.p1_wins += 1;
            round_state.p2_wins += 1;
        } else if p1_dead {
            round_state.p2_wins += 1;
        } else {
            round_state.p1_wins += 1;
        }
    }
}

/// Tekken-style dynamic camera: tracks both fighters, zooms based on distance,
/// slight downward angle, smooth interpolation, hit-stop camera shake.
fn camera_system(
    mut rng_state: ResMut<RngState>,
    mut feedback: ResMut<CombatFeedback>,
    q_fighters: Query<(&Fighter, &Transform)>,
    mut q_cam: Query<(Mut<gizmo::renderer::components::Camera>, Mut<Transform>)>,
    dt: f32,
) {
    let mut p1_pos = Vec3::ZERO;
    let mut p2_pos = Vec3::ZERO;
    for (_, (f, t)) in q_fighters.iter() {
        if f.player_id == 1 {
            p1_pos = t.position;
        }
        if f.player_id == 2 {
            p2_pos = t.position;
        }
    }

    let distance_x = (p1_pos.x - p2_pos.x).abs();
    let midpoint = (p1_pos + p2_pos) * 0.5;

    // ── Tekken Camera Parameters ──
    // Daha yakın, ayakları ve boydan karakterleri net görecek şekilde.
    let min_z = 2.5;
    let max_z = 6.0;
    let target_cam_z = (min_z + distance_x * 0.4).clamp(min_z, max_z);

    // Yükseklik: Karakterlerin orta noktasına yakın (ayaklardan gövdeye doğru)
    let target_cam_y = 1.0 + (distance_x * 0.04).min(0.5);

    // X: Dövüşçülerin ortası
    let target_cam_x = midpoint.x;

    // Yaw ve Pitch: Hafif yukarı doğru bakarak kahramansı (hero) açısı veriyoruz
    let target_yaw = -std::f32::consts::FRAC_PI_2;
    let target_pitch = 0.08 + (distance_x * 0.01).min(0.12);

    // ── Camera Shake (hit feedback) ──
    feedback.camera_shake = (feedback.camera_shake - dt * 3.0).max(0.0);
    let shake_x = if feedback.camera_shake > 0.0 {
        (rng_state.seed as f32 % 10.0 - 5.0) * feedback.camera_shake * 0.15
    } else {
        0.0
    };
    let shake_y = if feedback.camera_shake > 0.0 {
        ((rng_state.seed >> 1) as f32 % 10.0 - 5.0) * feedback.camera_shake * 0.1
    } else {
        0.0
    };
    rng_state.seed = rng_state.seed.wrapping_add(1);

    // ── Smooth Interpolation ──
    for (_, (mut cam, mut trans)) in q_cam.iter_mut() {
        if !cam.primary {
            continue;
        }
        let s = 4.0 * dt; // smooth factor (lower = more cinematic)
        trans.position.x += (target_cam_x + shake_x - trans.position.x) * s;
        trans.position.y += (target_cam_y + shake_y - trans.position.y) * s;
        trans.position.z += (target_cam_z - trans.position.z) * s;
        cam.yaw += (target_yaw - cam.yaw) * s;
        cam.pitch += (target_pitch - cam.pitch) * s;
        cam.sanitize_angles();
    }
}

fn particle_system(
    queue: Res<gizmo::core::CommandQueue>,
    mut q_particles: Query<(Mut<Particle>, Mut<Transform>)>,
    dt: f32,
) {
    let mut to_despawn = Vec::new();
    for (e, (mut p, mut t)) in q_particles.iter_mut() {
        p.timer -= dt;
        if p.timer <= 0.0 {
            to_despawn.push(e);
        } else {
            t.position += p.velocity * dt;
            p.velocity.y -= 30.0 * dt;
            t.scale *= 1.0 - dt;
        }
    }
    if !to_despawn.is_empty() {
        queue.push(move |w: &mut World| {
            for e in to_despawn {
                w.despawn_by_id(e);
            }
        });
    }
}

fn apply_damage_system(
    events: gizmo::core::event::EventReader<DamageEvent>,
    mut q_fighters: Query<(Mut<Fighter>, Mut<Transform>)>,
    mut feedback: ResMut<CombatFeedback>,
    mut rng_state: ResMut<RngState>,
    queue: Res<gizmo::core::CommandQueue>,
    assets: Res<GameAssets>,
) {
    let mut hit_sparks: Vec<Vec3> = Vec::new();

    for event in events.iter() {
        // Kamera sarsıntısı & hit-stop
        match event.hit_type {
            3 => {
                feedback.camera_shake = 0.5;
                feedback.hit_stop = 0.15;
            }
            5 => {
                feedback.camera_shake = 0.3;
                feedback.hit_stop = 0.1;
            }
            _ => {
                feedback.hit_stop = 0.08;
            }
        }

        for (id, (mut fighter, trans)) in q_fighters.iter_mut() {
            if id == event.target {
                if !event.is_blocked {
                    fighter.health -= event.damage;
                    fighter.state = FighterState::HitStun;
                    fighter.state_timer = event.hit_stun_time;
                    fighter.velocity_x = event.pushback_x;
                    if event.knockback_y != 0.0 {
                        fighter.velocity_y = event.knockback_y;
                    }
                    // Kıvılcım pozisyonu
                    let spark_offset = if fighter.facing_right {
                        Vec3::new(2.0, 1.0, 0.0)
                    } else {
                        Vec3::new(-2.0, 1.0, 0.0)
                    };
                    hit_sparks.push(trans.position + spark_offset);
                } else {
                    fighter.velocity_x = event.pushback_x * 1.5;
                }
            } else if id == event.attacker
                && !matches!(
                    fighter.state,
                    FighterState::HitStun | FighterState::Knockdown
                )
            {
                fighter.velocity_x = if fighter.facing_right { -5.0 } else { 5.0 };
            }
        }
    }

    if hit_sparks.is_empty() {
        return;
    }

    let (spark_mesh, spark_mat) = (assets.spark_mesh.clone(), assets.spark_mat.clone());
    let mut to_spawn: Vec<(Vec3, Vec3)> = Vec::new();

    for pos in hit_sparks {
        for _ in 0..15 {
            let vx = (rng_state.seed as f32 % 20.0) - 10.0;
            rng_state.seed = rng_state.seed.wrapping_add(1);
            let vy = 5.0 + (rng_state.seed as f32 % 15.0);
            rng_state.seed = rng_state.seed.wrapping_add(1);
            let vz = (rng_state.seed as f32 % 10.0) - 5.0;
            rng_state.seed = rng_state.seed.wrapping_add(1);
            to_spawn.push((pos, Vec3::new(vx, vy, vz)));
        }
    }

    queue.push(move |w: &mut World| {
        for (pos, vel) in to_spawn {
            let spark = w.spawn_bundle(
                MeshBundle::new(spark_mesh.clone(), spark_mat.clone())
                    .at(pos)
                    .with_scale(Vec3::new(0.4, 0.4, 0.4)),
            );
            w.add_component(
                spark,
                Particle {
                    velocity: vel,
                    timer: 0.5,
                },
            );
        }
    });
}

fn draw_win_circles(ui: &mut egui::Ui, wins: u8) {
    ui.horizontal(|ui| {
        for i in 0..2u8 {
            let color = if wins > i {
                egui::Color32::YELLOW
            } else {
                egui::Color32::DARK_GRAY
            };
            ui.add(
                egui::Button::new("")
                    .fill(color)
                    .sense(egui::Sense::hover()),
            );
        }
    });
}

fn game_reset_system(
    mut round_state: ResMut<RoundState>,
    mut feedback: ResMut<CombatFeedback>,
    mut q_fighters: Query<(Mut<Fighter>, Mut<Transform>)>,
) {
    if !round_state.needs_reset {
        return;
    }
    round_state.needs_reset = false;
    round_state.p1_wins = 0;
    round_state.p2_wins = 0;
    round_state.round = 1;
    round_state.round_timer = 99.0;
    feedback.hit_stop = 0.0;
    feedback.camera_shake = 0.0;

    for (_, (mut f, mut trans)) in q_fighters.iter_mut() {
        f.health = 100.0;
        f.state = FighterState::Idle;
        f.velocity_y = 0.0;
        f.velocity_x = 0.0;
        f.combo_count = 0;
        trans.position.y = 0.0;
        trans.scale = Vec3::new(1.0, 1.0, 1.0);
        trans.position.x = if f.player_id == 1 { -2.0 } else { 2.0 };
    }
}

fn ui(world: &mut World, _ignored: &mut (), ctx: &egui::Context) {
    let mut round_state = world.get_resource_mut::<RoundState>().unwrap();
    let mut phase_state = world.get_resource_mut::<State<GamePhase>>().unwrap();
    let phase = *phase_state.get();

    if phase == GamePhase::Menu {
        let frame = egui::Frame::window(&ctx.style())
            .fill(egui::Color32::from_black_alpha(200))
            .rounding(10.0)
            .inner_margin(50.0);

        egui::Window::new("Menu")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(frame)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("STREET GIZMO")
                            .size(70.0)
                            .color(egui::Color32::from_rgb(255, 120, 50))
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("THE KING OF IRON FIST")
                            .size(20.0)
                            .color(egui::Color32::LIGHT_GRAY)
                            .italics(),
                    );
                    ui.add_space(50.0);

                    let btn_size = egui::vec2(280.0, 50.0);

                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("VERSUS MODE").size(24.0)),
                        )
                        .clicked()
                    {
                        phase_state.set(GamePhase::Playing);
                        round_state.needs_reset = true;
                    }
                    ui.add_space(15.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("ARCADE MODE").size(24.0)),
                        )
                        .clicked()
                    {
                        phase_state.set(GamePhase::Playing);
                        round_state.needs_reset = true;
                    }
                    ui.add_space(15.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("TRAINING").size(24.0)),
                        )
                        .clicked()
                    {
                        phase_state.set(GamePhase::Playing);
                        round_state.needs_reset = true;
                    }
                    ui.add_space(15.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("OPTIONS").size(24.0)),
                        )
                        .clicked()
                    {
                        // TODO: Options menu
                    }
                    ui.add_space(15.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("EXIT").size(24.0)),
                        )
                        .clicked()
                    {
                        std::process::exit(0);
                    }
                });
            });
        return;
    }

    egui::TopBottomPanel::top("hud_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("GIZMO");
            ui.add_space(20.0);
            draw_win_circles(ui, round_state.p1_wins);
            ui.label(format!(
                "P1: {:.0} HP",
                round_state.p1_display_health.max(0.0)
            ));
            ui.add(
                egui::ProgressBar::new(round_state.p1_display_health.max(0.0) / 100.0)
                    .fill(egui::Color32::BLUE)
                    .desired_width(300.0),
            );
            ui.add_space(30.0);
            ui.heading(format!(
                "RAUND {} | SURE: {:.0}",
                round_state.round, round_state.round_timer
            ));
            ui.add_space(30.0);
            ui.add(
                egui::ProgressBar::new(round_state.p2_display_health.max(0.0) / 100.0)
                    .fill(egui::Color32::RED)
                    .desired_width(300.0),
            );
            ui.label(format!(
                "P2: {:.0} HP",
                round_state.p2_display_health.max(0.0)
            ));
            draw_win_circles(ui, round_state.p2_wins);
        });

        if phase == GamePhase::RoundOver {
            ui.add_space(20.0);
            ui.heading(format!(
                "RAUND BITTI! Siradaki raunt: {:.1}",
                round_state.round_over_timer
            ));
        }

        if phase == GamePhase::GameOver {
            ui.add_space(20.0);
            ui.heading(format!(
                "OYUN BITTI! KAZANAN: {}",
                if round_state.p1_wins >= 2 {
                    "PLAYER 1"
                } else {
                    "PLAYER 2"
                }
            ));
            if ui.button("TEKRAR OYNA").clicked() {
                phase_state.set(GamePhase::Menu);
                round_state.needs_reset = true;
            }
        }

        if phase == GamePhase::Paused {
            ui.add_space(20.0);
            ui.heading("DURAKLATILDI (ESCAPE ile devam et)");
        }
    });
}

fn render(world: &mut World, _ignored: &(), ctx: &mut RenderContext) {
    ctx.disable_gpu_compute();
    {
        let dt = world
            .get_resource::<gizmo::core::time::Time>()
            .map(|t| t.dt())
            .unwrap_or(1.0 / 60.0);
        let queue: *const wgpu::Queue = &ctx.renderer().queue;
        unsafe {
            gizmo::renderer::animation_update_system(world, dt, &*queue);
        }
    }
    ctx.default_render(world);
}

fn main() {
    App::<()>::new("Gizmo Engine - 2D Fighter", 1280, 720)
        .set_setup(setup)
        .add_event::<DamageEvent>()
        .add_plugin(gizmo::asset_server::AssetServerPlugin)
        .add_plugin(gizmo::prelude::TransformPlugin)
        .add_system(state_transition_system)
        .add_system(sync_health_system)
        .add_system(global_input_system)
        .add_system(game_reset_system)
        .add_system(
            menu_input_system
                .run_if(in_state(GamePhase::Menu))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            round_over_system
                .run_if(in_state(GamePhase::RoundOver))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            round_timer_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            ai_input_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            player_input_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            fighter_state_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            fighter_movement_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            combat_hit_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            match_rules_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            camera_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            particle_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            character_animation_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .add_system(
            apply_damage_system
                .run_if(in_state(GamePhase::Playing))
                .reads_res::<State<GamePhase>>(),
        )
        .set_ui(ui)
        .set_simple_render(render)
        .run();
}

fn character_animation_system(
    q_fighters: Query<&Fighter>,
    mut q_anim_players: Query<(
        &gizmo::core::component::Parent,
        Mut<gizmo::renderer::components::AnimationPlayer>,
    )>,
    _feedback: Res<CombatFeedback>,
    _dt: f32,
) {
    // Fighter durumlarını topla
    let mut fighter_states = std::collections::HashMap::new();
    for (entity, fighter) in q_fighters.iter() {
        fighter_states.insert(entity, fighter.clone());
    }

    for (_, (parent, mut player)) in q_anim_players.iter_mut() {
        let Some(fighter) = fighter_states.get(&parent.0) else {
            continue;
        };
        let is_grounded = fighter.velocity_y == 0.0;

        let (target_anim, speed) = if !is_grounded {
            if fighter.state == FighterState::JumpKicking {
                (AnimIndex::KickUp as usize, 1.1)
            } else {
                (AnimIndex::Jump as usize, 1.0)
            }
        } else {
            match fighter.state {
                FighterState::Idle => (AnimIndex::Idle as usize, 1.0),
                FighterState::Walking => {
                    let forward = if fighter.facing_right {
                        fighter.velocity_x > 0.0
                    } else {
                        fighter.velocity_x < 0.0
                    };
                    if forward {
                        (AnimIndex::RunToEnemy as usize, 1.0)
                    } else {
                        (AnimIndex::WalkBack as usize, 1.0)
                    }
                }
                FighterState::StandingKick => (AnimIndex::KickUp as usize, 1.0),
                FighterState::CrouchPunching => (AnimIndex::CrouchPunch as usize, 1.05),
                FighterState::Crouching => (AnimIndex::BodyBlock as usize, 1.0),
                FighterState::LowKicking => (AnimIndex::EvadeKick as usize, 1.0),
                FighterState::JumpKicking => (AnimIndex::KickUp as usize, 1.0),
                FighterState::Punching => {
                    if fighter.combo_count == 3 {
                        (AnimIndex::ElbowPunch as usize, 1.05)
                    } else {
                        (AnimIndex::DirectPunch as usize, 1.05)
                    }
                }
                FighterState::HitStun => (AnimIndex::Damaged as usize, 1.0),
                FighterState::Knockdown => {
                    if fighter.state_timer > 1.0 {
                        (AnimIndex::StandUp as usize, 1.2)
                    } else {
                        (AnimIndex::Dead as usize, 1.0)
                    }
                }
            }
        };

        player.speed = speed;

        if player.active_animation != target_anim && target_anim < player.animations.len() {
            let is_snappy = matches!(
                target_anim,
                3 | 4 | 5 | 7 | 8 | 10 // Punches, Kicks, and Hit Reactions
            );

            player.prev_animation = Some(player.active_animation);
            player.prev_time = player.current_time;
            player.blend_time = 0.0;
            player.blend_duration = if is_snappy { 0.05 } else { 0.18 };
            player.active_animation = target_anim;
            player.current_time = 0.0;
        }
    }
}
