use super::{truck_stepio::out, *};

impl out::ConstStepLength for Processor<Sphere, Matrix4> {
    const LENGTH: usize = Processor::<truck_geometry::prelude::Sphere, Matrix4>::LENGTH;
}
impl out::StepLength for Processor<Sphere, Matrix4> {
    fn step_length(&self) -> usize {
        <Self as out::ConstStepLength>::LENGTH
    }
}
impl out::DisplayByStep for Processor<Sphere, Matrix4> {
    fn fmt(&self, idx: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Processor::new(self.entity().0)
            .transformed(*self.transform())
            .fmt(idx, f)
    }
}

impl out::ConstStepLength for Processor<DegenerateTorus, Matrix4> {
    const LENGTH: usize = 5;
}
impl out::StepLength for Processor<DegenerateTorus, Matrix4> {
    fn step_length(&self) -> usize {
        <Self as out::ConstStepLength>::LENGTH
    }
}

impl out::StepSurface for Processor<DegenerateTorus, Matrix4> {
    #[inline(always)]
    fn same_sense(&self) -> bool {
        self.orientation()
    }
}

impl out::DisplayByStep for Processor<DegenerateTorus, Matrix4> {
    fn fmt(&self, idx: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ctx = ToleranceCtx::unscaled_legacy();
        let carrier = self.entity();
        let torus = carrier.inner();
        let transform = self.transform();
        let position_idx = idx + 1;
        let location_idx = idx + 2;
        let axis_idx = idx + 3;
        let ref_direction_idx = idx + 4;
        let location = transform[3].to_point() + torus.center().to_vec();
        let axis = out::VectorAsDirection(transform[2].truncate().normalize());
        let r0 = transform[0].magnitude();
        let r1 = transform[1].magnitude();
        if !ctx.is_small_ratio(r0 - r1) {
            // BG-TOL-001: param
            f.write_str("The transform of degenerate torus includes non-uniform scale.")?;
            return Err(std::fmt::Error);
        }
        let ref_direction = out::VectorAsDirection(transform[0].truncate() / r0);
        let major = out::FloatDisplay(r0 * torus.large_radius());
        let minor = out::FloatDisplay(r0 * torus.small_radius());
        let select_outer = out::BooleanDisplay(carrier.select_outer());
        f.write_fmt(format_args!(
            "#{idx} = DEGENERATE_TOROIDAL_SURFACE('', #{position_idx}, {major}, {minor}, {select_outer});\n\
#{position_idx} = AXIS2_PLACEMENT_3D('', #{location_idx}, #{axis_idx}, #{ref_direction_idx});\n"
        ))?;
        out::DisplayByStep::fmt(&location, location_idx, f)?;
        out::DisplayByStep::fmt(&axis, axis_idx, f)?;
        out::DisplayByStep::fmt(&ref_direction, ref_direction_idx, f)
    }
}

impl out::DisplayByStep for ElementarySurface {
    fn fmt(&self, idx: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plane(x) => x.fmt(idx, f),
            Self::Sphere(x) => x.fmt(idx, f),
            Self::ToroidalSurface(x) => x.fmt(idx, f),
            Self::DegenerateToroidalSurface(x) => x.fmt(idx, f),
            Self::CylindricalSurface(processor) => {
                let position_idx = idx + 1;
                let location_idx = idx + 2;
                let axis_idx = idx + 3;
                let ref_direction_idx = idx + 4;

                let revo = processor.entity();
                let trans = processor.transform();
                let o = trans.transform_point(revo.origin());
                let p = trans.transform_point(revo.entity_curve().0);
                let axis = trans.transform_vector(revo.axis());

                let location = out::StepDataDisplay::new(o, location_idx);
                let direction_axis = out::VectorAsDirection(axis);
                let axis = out::StepDataDisplay::new(direction_axis, axis_idx);
                let raw_ref_direction = out::VectorAsDirection((p - o).normalize());
                let ref_direction = out::StepDataDisplay::new(raw_ref_direction, ref_direction_idx);
                let radius = (p - o).magnitude();

                f.write_fmt(format_args!(
                    "#{idx} = CYLINDRICAL_SURFACE('', #{position_idx}, {radius});
#{position_idx} = AXIS2_PLACEMENT_3D('', #{location_idx}, #{axis_idx}, #{ref_direction_idx});
{location}{axis}{ref_direction}"
                ))
            }
            Self::ConicalSurface(processor) => {
                let revo = processor.entity();
                let transform = processor.transform();
                let line = revo.entity_curve();
                let p = line.0;
                let v = line.1 - p;

                let radius = out::FloatDisplay(p.x);
                let semi_angle = out::FloatDisplay(f64::atan(v.x));

                let position_idx = idx + 1;
                let location_idx = idx + 2;
                let axis_idx = idx + 3;
                let ref_direction_idx = idx + 4;

                let location = out::StepDataDisplay::new(transform[3].to_point(), location_idx);
                let raw_axis = out::VectorAsDirection(transform[2].truncate());
                let axis = out::StepDataDisplay::new(raw_axis, axis_idx);
                let raw_ref_direction = out::VectorAsDirection(transform[0].truncate());
                let ref_direction = out::StepDataDisplay::new(raw_ref_direction, ref_direction_idx);

                f.write_fmt(format_args!(
                    "#{idx} = CONICAL_SURFACE('', #{position_idx}, {radius}, {semi_angle});
#{position_idx} = AXIS2_PLACEMENT_3D('', #{location_idx}, #{axis_idx}, #{ref_direction_idx});
{location}{axis}{ref_direction}"
                ))
            }
        }
    }
}
