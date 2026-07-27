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
            let half_height = if aspect >= 1.0 {
                radius * view.padding
            } else {
                radius * view.padding / aspect
            };
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
}
