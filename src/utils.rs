use bevy::math::Vec3;

pub fn cube_particle_positions(ni: usize, nj: usize, nk: usize, particle_rad: f32) -> Vec<Vec3> {
    let mut points = Vec::new();
    let half_extents = Vec3::new(ni as f32, nj as f32, nk as f32) * particle_rad;

    for i in 0..ni {
        for j in 0..nj {
            for k in 0..nk {
                let x = (i as f32) * particle_rad * 2.0;
                let y = (j as f32) * particle_rad * 2.0;
                let z = (k as f32) * particle_rad * 2.0;
                points.push(Vec3::new(x, y, z) + Vec3::splat(particle_rad) - half_extents);
            }
        }
    }

    points
}
