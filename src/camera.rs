use glam::{Mat4, Vec3};

use crate::{
    config::{CameraKind, ViewConfig},
    scene::Bounds,
};

#[derive(Debug, Clone)]
pub struct PreparedCamera {
    pub id: String,
    pub view_projection: Mat4,
    pub position: [f32; 3],
    pub target: [f32; 3],
}

pub fn prepare_camera(view: &ViewConfig, bounds: &Bounds, resolution: [u32; 2]) -> PreparedCamera {
    let center = bounds.center();
    let radius = (bounds.size().length() * 0.5).max(1.0e-3);
    let direction = Vec3::from_array(view.direction)
        .try_normalize()
        .unwrap_or(Vec3::new(1.0, 1.0, 1.0).normalize());
    let configured_up = view.up.map(Vec3::from_array);
    let up = configured_up.unwrap_or_else(|| {
        if direction.dot(Vec3::Y).abs() > 0.98 {
            if direction.y > 0.0 {
                Vec3::NEG_Z
            } else {
                Vec3::Z
            }
        } else {
            Vec3::Y
        }
    });
    let aspect = resolution[0] as f32 / resolution[1] as f32;

    let (position, projection) = match view.kind {
        CameraKind::Perspective => {
            let vertical_half = (view.fov_degrees.to_radians() * 0.5).max(0.01);
            let horizontal_half = (vertical_half.tan() * aspect).atan();
            let limiting_half = vertical_half.min(horizontal_half);
            let distance = radius * view.padding / limiting_half.sin().max(0.01);
            let position = center + direction * distance;
            let near = (distance - radius * view.padding * 1.5).max(radius * 1.0e-4);
            let far = distance + radius * view.padding * 2.0;
            (
                position,
                Mat4::perspective_rh(view.fov_degrees.to_radians(), aspect, near, far),
            )
        }
        CameraKind::Orthographic => {
            let forward = -direction;
            let right = forward.cross(up).try_normalize().unwrap_or(Vec3::X);
            let camera_up = right.cross(forward).try_normalize().unwrap_or(Vec3::Y);
            let min = Vec3::from_array(bounds.min);
            let max = Vec3::from_array(bounds.max);
            let mut projected_half_width = 0.0_f32;
            let mut projected_half_height = 0.0_f32;
            for x in [min.x, max.x] {
                for y in [min.y, max.y] {
                    for z in [min.z, max.z] {
                        let offset = Vec3::new(x, y, z) - center;
                        projected_half_width = projected_half_width.max(offset.dot(right).abs());
                        projected_half_height =
                            projected_half_height.max(offset.dot(camera_up).abs());
                    }
                }
            }
            let half_height = projected_half_height
                .max(projected_half_width / aspect)
                .max(1.0e-4)
                * view.padding;
            let half_width = half_height * aspect;
            let distance = radius * 3.0;
            let position = center + direction * distance;
            (
                position,
                Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    radius * 0.01,
                    radius * 8.0,
                ),
            )
        }
    };
    let view_matrix = Mat4::look_at_rh(position, center, up);
    PreparedCamera {
        id: view.id.clone(),
        view_projection: projection * view_matrix,
        position: position.to_array(),
        target: center.to_array(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CameraKind, NamedView, ViewConfig};

    #[test]
    fn camera_matrices_are_finite() {
        let bounds = Bounds {
            min: [-1.0; 3],
            max: [1.0; 3],
        };
        let camera = prepare_camera(
            &ViewConfig::named(NamedView::Iso, CameraKind::Perspective),
            &bounds,
            [1024, 768],
        );
        assert!(camera.view_projection.is_finite());
    }

    #[test]
    fn orthographic_fit_uses_projected_bounds_not_diagonal_radius() {
        let bounds = Bounds {
            min: [-0.5; 3],
            max: [0.5; 3],
        };
        let camera = prepare_camera(
            &ViewConfig::named(NamedView::Front, CameraKind::Orthographic),
            &bounds,
            [512, 512],
        );
        let corner = camera.view_projection * Vec3::new(0.5, 0.5, 0.5).extend(1.0);
        let ndc = corner.truncate() / corner.w;
        assert!((ndc.x - 1.0 / 1.1).abs() < 1.0e-4);
        assert!((ndc.y - 1.0 / 1.1).abs() < 1.0e-4);
    }
}
