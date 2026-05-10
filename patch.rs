fn character_animation_system(
    mut q_rigs: gizmo::core::query::Query<(gizmo::core::system::Mut<CharacterRig>, &Fighter)>,
    q_transforms: gizmo::core::query::Query<gizmo::core::system::Mut<Transform>>,
    state: gizmo::core::system::Res<GameStatus>,
    dt: f32,
) {
    let fighter_dt = if state.hit_stop > 0.0 { 0.0 } else { dt };

    for (_, (mut rig, fighter)) in q_rigs.iter_mut() {
        rig.base_timer += fighter_dt * 10.0;
        let walk_cycle = f32::sin(rig.base_timer);

        let mut arm_l_rot = Quat::IDENTITY;
        let mut arm_r_rot = Quat::IDENTITY;
        let mut leg_l_rot = Quat::IDENTITY;
        let mut leg_r_rot = Quat::IDENTITY;
        let mut torso_rot = Quat::IDENTITY;
        let mut head_rot = Quat::IDENTITY;

        let mut torso_pos = Vec3::new(0.0, 1.2, 0.0);
        let mut head_pos = Vec3::new(0.0, 2.2, 0.0);

        if fighter.velocity_x.abs() > 0.1 && fighter.state == FighterState::Walking {
            arm_l_rot = Quat::from_rotation_z(walk_cycle * 0.5);
            arm_r_rot = Quat::from_rotation_z(-walk_cycle * 0.5);
            leg_l_rot = Quat::from_rotation_z(-walk_cycle * 0.5);
            leg_r_rot = Quat::from_rotation_z(walk_cycle * 0.5);
            torso_pos.y += f32::abs(walk_cycle) * 0.1;
            head_pos.y += f32::abs(walk_cycle) * 0.1;
        }

        if fighter.state == FighterState::Punching {
            if fighter.facing_right {
                arm_r_rot = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
            } else {
                arm_l_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            }
        } else if fighter.state == FighterState::StandingKick {
             if fighter.facing_right {
                leg_r_rot = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
            } else {
                leg_l_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            }
        } else if fighter.state == FighterState::LowKicking || fighter.state == FighterState::Crouching {
             torso_pos.y = 0.5;
             head_pos.y = 1.5;
             if fighter.state == FighterState::LowKicking {
                 if fighter.facing_right {
                    leg_r_rot = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
                 } else {
                    leg_l_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
                 }
             }
        } else if fighter.state == FighterState::JumpKicking {
             if fighter.facing_right {
                leg_r_rot = Quat::from_rotation_z(-std::f32::consts::PI / 4.0);
             } else {
                leg_l_rot = Quat::from_rotation_z(std::f32::consts::PI / 4.0);
             }
        }

        if fighter.state == FighterState::HitStun {
            torso_rot = Quat::from_rotation_z(if fighter.facing_right { -0.3 } else { 0.3 });
            head_rot = Quat::from_rotation_z(if fighter.facing_right { -0.5 } else { 0.5 });
        } else if fighter.state == FighterState::Knockdown {
            torso_rot = Quat::from_rotation_z(if fighter.facing_right { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 });
            torso_pos.y = 0.2;
            head_pos.y = 0.2;
            head_pos.x = if fighter.facing_right { -1.0 } else { 1.0 };
        }

        if let Some(mut t) = q_transforms.get(rig.arm_l) { t.set_rotation(arm_l_rot); }
        if let Some(mut t) = q_transforms.get(rig.arm_r) { t.set_rotation(arm_r_rot); }
        if let Some(mut t) = q_transforms.get(rig.leg_l) { t.set_rotation(leg_l_rot); }
        if let Some(mut t) = q_transforms.get(rig.leg_r) { t.set_rotation(leg_r_rot); }
        
        if let Some(mut t) = q_transforms.get(rig.torso) { 
            t.set_rotation(torso_rot); 
            t.position = torso_pos;
        }
        if let Some(mut t) = q_transforms.get(rig.head) { 
            t.set_rotation(head_rot); 
            t.position = head_pos;
        }
    }
}
