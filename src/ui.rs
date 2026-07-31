use anyhow::Result;

use crate::scene::{CompiledScene, SceneStatistics};

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(CHARS[((triple >> 18) & 63) as usize] as char);
        out.push(CHARS[((triple >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(CHARS[(triple & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Generates a self-contained, interactive HTML 3D viewer for any `CompiledScene`.
pub fn generate_html_viewer(
    scene: &CompiledScene,
    title: &str,
    stats: &SceneStatistics,
) -> Result<String> {
    let mut flat_positions: Vec<f32> = Vec::new();
    let mut flat_normals: Vec<f32> = Vec::new();
    let mut flat_indices: Vec<u32> = Vec::new();

    for instance in &scene.instances {
        let geom = &scene.geometries[instance.geometry];
        let base_index = (flat_positions.len() / 3) as u32;

        for v in &geom.vertices {
            let p = instance
                .transform
                .transform_point3(glam::Vec3::from(v.position));
            let n = (instance.normal_transform * glam::Vec3::from(v.normal)).normalize_or_zero();

            flat_positions.push(p.x);
            flat_positions.push(p.y);
            flat_positions.push(p.z);

            flat_normals.push(n.x);
            flat_normals.push(n.y);
            flat_normals.push(n.z);
        }

        for idx in &geom.indices {
            flat_indices.push(base_index + idx);
        }
    }

    let pos_bytes = bytemuck::cast_slice::<f32, u8>(&flat_positions);
    let norm_bytes = bytemuck::cast_slice::<f32, u8>(&flat_normals);
    let idx_bytes = bytemuck::cast_slice::<u32, u8>(&flat_indices);

    let pos_b64 = base64_encode(pos_bytes);
    let norm_b64 = base64_encode(norm_bytes);
    let idx_b64 = base64_encode(idx_bytes);

    let num_triangles = flat_indices.len() / 3;
    let num_vertices = flat_positions.len() / 3;

    let center = [
        (scene.bounds.min[0] + scene.bounds.max[0]) * 0.5,
        (scene.bounds.min[1] + scene.bounds.max[1]) * 0.5,
        (scene.bounds.min[2] + scene.bounds.max[2]) * 0.5,
    ];
    let fit_radius = scene.fit_radius.max(1.0e-3);

    let format_badge = if title.to_lowercase().ends_with(".step")
        || title.to_lowercase().ends_with(".stp")
    {
        "STEP B-REP"
    } else if title.to_lowercase().ends_with(".glb") || title.to_lowercase().ends_with(".gltf") {
        "glTF / GLB"
    } else if title.to_lowercase().ends_with(".stl") {
        "STL MESH"
    } else {
        "3D MODEL"
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - look 3D Interactive Viewer</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; user-select: none; }}
        body {{
            background-color: #0c0d12;
            color: #e2e8f0;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            overflow: hidden;
            width: 100vw;
            height: 100vh;
        }}
        #canvas-container {{
            width: 100%;
            height: 100%;
            position: absolute;
            top: 0;
            left: 0;
        }}
        canvas {{ width: 100%; height: 100%; display: block; }}
        
        .header-bar {{
            position: absolute;
            top: 16px;
            left: 16px;
            background: rgba(18, 20, 29, 0.75);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            padding: 14px 20px;
            display: flex;
            align-items: center;
            gap: 16px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            pointer-events: auto;
            z-index: 10;
        }}
        .model-title {{
            font-weight: 700;
            font-size: 16px;
            color: #f8fafc;
            letter-spacing: -0.01em;
        }}
        .format-badge {{
            background: linear-gradient(135deg, #3b82f6, #2563eb);
            color: #ffffff;
            font-size: 11px;
            font-weight: 700;
            padding: 4px 8px;
            border-radius: 6px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .stat-item {{
            font-size: 12px;
            color: #94a3b8;
            display: flex;
            gap: 4px;
        }}
        .stat-value {{
            color: #cbd5e1;
            font-weight: 600;
        }}

        .toolbar {{
            position: absolute;
            bottom: 24px;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(18, 20, 29, 0.85);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 30px;
            padding: 6px 12px;
            display: flex;
            gap: 6px;
            box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
            z-index: 10;
        }}
        .btn {{
            background: transparent;
            border: none;
            color: #94a3b8;
            font-weight: 600;
            font-size: 13px;
            padding: 8px 14px;
            border-radius: 20px;
            cursor: pointer;
            transition: all 0.15s ease;
        }}
        .btn:hover {{
            background: rgba(255, 255, 255, 0.1);
            color: #f8fafc;
        }}
        .btn:active {{
            transform: scale(0.96);
        }}
        
        .legend {{
            position: absolute;
            bottom: 24px;
            right: 24px;
            background: rgba(18, 20, 29, 0.65);
            backdrop-filter: blur(8px);
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 8px;
            padding: 10px 14px;
            font-size: 11px;
            color: #64748b;
            line-height: 1.6;
            z-index: 10;
        }}
        .legend kbd {{
            background: rgba(255, 255, 255, 0.1);
            color: #cbd5e1;
            padding: 2px 5px;
            border-radius: 4px;
            font-family: inherit;
        }}
    </style>
</head>
<body>
    <div id="canvas-container">
        <canvas id="gl-canvas"></canvas>
    </div>

    <div class="header-bar">
        <div>
            <div class="model-title">{title}</div>
            <div style="margin-top: 4px; display: flex; gap: 12px; align-items: center;">
                <span class="format-badge">{format_badge}</span>
                <span class="stat-item">Triangles: <span class="stat-value">{num_triangles}</span></span>
                <span class="stat-item">Vertices: <span class="stat-value">{num_vertices}</span></span>
                <span class="stat-item">Geometries: <span class="stat-value">{unique_geometries}</span></span>
            </div>
        </div>
    </div>

    <div class="toolbar">
        <button class="btn" onclick="setPresetView('iso')">Iso</button>
        <button class="btn" onclick="setPresetView('front')">Front</button>
        <button class="btn" onclick="setPresetView('top')">Top</button>
        <button class="btn" onclick="setPresetView('right')">Right</button>
        <button class="btn" onclick="resetView()">Reset</button>
    </div>

    <div class="legend">
        <div><kbd>Left Drag</kbd> Rotate model</div>
        <div><kbd>Scroll Wheel</kbd> Zoom in / out</div>
        <div><kbd>Right / Shift Drag</kbd> Pan view</div>
    </div>

    <script>
        const posB64 = "{pos_b64}";
        const normB64 = "{norm_b64}";
        const idxB64 = "{idx_b64}";

        function b64ToFloat32Array(b64) {{
            const bin = atob(b64);
            const bytes = new Uint8Array(bin.length);
            for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
            return new Float32Array(bytes.buffer);
        }}
        function b64ToUint32Array(b64) {{
            const bin = atob(b64);
            const bytes = new Uint8Array(bin.length);
            for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
            return new Uint32Array(bytes.buffer);
        }}

        const positions = b64ToFloat32Array(posB64);
        const normals = b64ToFloat32Array(normB64);
        const indices = b64ToUint32Array(idxB64);

        const modelCenter = [{center_x}, {center_y}, {center_z}];
        const fitRadius = {fit_radius};

        let yaw = Math.PI / 4;
        let pitch = Math.PI / 6;
        let distance = fitRadius * 2.5;
        let target = [...modelCenter];

        const canvas = document.getElementById('gl-canvas');
        const gl = canvas.getContext('webgl2', {{ antialias: true }});
        if (!gl) alert('WebGL 2.0 is required');

        const vsSource = `#version 300 es
            in vec3 aPosition;
            in vec3 aNormal;
            uniform mat4 uMVP;
            uniform mat4 uModel;
            out vec3 vNormal;
            out vec3 vFragPos;
            void main() {{
                vNormal = mat3(uModel) * aNormal;
                vFragPos = vec3(uModel * vec4(aPosition, 1.0));
                gl_Position = uMVP * vec4(aPosition, 1.0);
            }}
        `;

        const fsSource = `#version 300 es
            precision highp float;
            in vec3 vNormal;
            in vec3 vFragPos;
            uniform vec3 uCameraPos;
            out vec4 fragColor;
            void main() {{
                vec3 N = normalize(vNormal);
                vec3 lightDir1 = normalize(vec3(0.5, 0.8, 1.0));
                vec3 lightDir2 = normalize(vec3(-0.5, -0.4, -0.6));
                
                float diff1 = max(dot(N, lightDir1), 0.0);
                float diff2 = max(dot(N, lightDir2), 0.0) * 0.35;
                float ambient = 0.25;

                vec3 baseColor = vec3(0.72, 0.75, 0.80);
                vec3 col = baseColor * (ambient + diff1 * 0.75 + diff2);

                vec3 viewDir = normalize(uCameraPos - vFragPos);
                vec3 halfDir = normalize(lightDir1 + viewDir);
                float spec = pow(max(dot(N, halfDir), 0.0), 32.0) * 0.25;
                col += vec3(spec);

                fragColor = vec4(col, 1.0);
            }}
        `;

        function createShader(gl, type, source) {{
            const shader = gl.createShader(type);
            gl.shaderSource(shader, source);
            gl.compileShader(shader);
            return shader;
        }}

        const program = gl.createProgram();
        gl.attachShader(program, createShader(gl, gl.VERTEX_SHADER, vsSource));
        gl.attachShader(program, createShader(gl, gl.FRAGMENT_SHADER, fsSource));
        gl.linkProgram(program);

        const vao = gl.createVertexArray();
        gl.bindVertexArray(vao);

        const posBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, posBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
        const aPosLoc = gl.getAttribLocation(program, 'aPosition');
        gl.enableVertexAttribArray(aPosLoc);
        gl.vertexAttribPointer(aPosLoc, 3, gl.FLOAT, false, 0, 0);

        const normBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, normBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, normals, gl.STATIC_DRAW);
        const aNormLoc = gl.getAttribLocation(program, 'aNormal');
        gl.enableVertexAttribArray(aNormLoc);
        gl.vertexAttribPointer(aNormLoc, 3, gl.FLOAT, false, 0, 0);

        const idxBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idxBuffer);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);

        const uMVPLoc = gl.getUniformLocation(program, 'uMVP');
        const uModelLoc = gl.getUniformLocation(program, 'uModel');
        const uCamPosLoc = gl.getUniformLocation(program, 'uCameraPos');

        function mat4Identity() {{
            return new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]);
        }}
        function mat4Perspective(fovy, aspect, near, far) {{
            const f = 1.0 / Math.tan(fovy / 2);
            const nf = 1.0 / (near - far);
            return new Float32Array([
                f / aspect, 0, 0, 0,
                0, f, 0, 0,
                0, 0, (far + near) * nf, -1,
                0, 0, (2 * far * near) * nf, 0
            ]);
        }}
        function mat4LookAt(eye, target, up) {{
            const z0 = eye[0] - target[0], z1 = eye[1] - target[1], z2 = eye[2] - target[2];
            let len = 1 / Math.hypot(z0, z1, z2);
            const zx = z0 * len, zy = z1 * len, zz = z2 * len;

            const xx = up[1] * zz - up[2] * zy, xy = up[2] * zx - up[0] * zz, xz = up[0] * zy - up[1] * zx;
            len = 1 / Math.hypot(xx, xy, xz);
            const rx = xx * len, ry = xy * len, rz = xz * len;

            const yx = zy * rz - zz * ry, yy = zz * rx - zx * rz, yz = zx * ry - zy * rx;

            return new Float32Array([
                rx, yx, zx, 0,
                ry, yy, zy, 0,
                rz, yz, zz, 0,
                -(rx * eye[0] + ry * eye[1] + rz * eye[2]),
                -(yx * eye[0] + yy * eye[1] + yz * eye[2]),
                -(zx * eye[0] + zy * eye[1] + zz * eye[2]), 1
            ]);
        }}
        function mat4Multiply(a, b) {{
            const out = new Float32Array(16);
            for (let i = 0; i < 4; i++) {{
                for (let j = 0; j < 4; j++) {{
                    out[j * 4 + i] =
                        a[i] * b[j * 4] +
                        a[i + 4] * b[j * 4 + 1] +
                        a[i + 8] * b[j * 4 + 2] +
                        a[i + 12] * b[j * 4 + 3];
                }}
            }}
            return out;
        }}

        function render() {{
            const width = canvas.clientWidth;
            const height = canvas.clientHeight;
            if (canvas.width !== width || canvas.height !== height) {{
                canvas.width = width;
                canvas.height = height;
                gl.viewport(0, 0, width, height);
            }}

            gl.enable(gl.DEPTH_TEST);
            gl.clearColor(0.05, 0.055, 0.07, 1.0);
            gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

            const camX = target[0] + distance * Math.cos(pitch) * Math.sin(yaw);
            const camY = target[1] + distance * Math.sin(pitch);
            const camZ = target[2] + distance * Math.cos(pitch) * Math.cos(yaw);
            const camPos = [camX, camY, camZ];

            const proj = mat4Perspective(Math.PI / 4, width / height, fitRadius * 0.01, fitRadius * 20.0);
            const view = mat4LookAt(camPos, target, [0, 1, 0]);
            const model = mat4Identity();

            const mvp = mat4Multiply(proj, mat4Multiply(view, model));

            gl.useProgram(program);
            gl.uniformMatrix4fv(uMVPLoc, false, mvp);
            gl.uniformMatrix4fv(uModelLoc, false, model);
            gl.uniform3fv(uCamPosLoc, camPos);

            gl.bindVertexArray(vao);
            gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);

            requestAnimationFrame(render);
        }}

        let isDragging = false;
        let isPanning = false;
        let lastMouseX = 0;
        let lastMouseY = 0;

        canvas.addEventListener('mousedown', (e) => {{
            isDragging = true;
            isPanning = e.button === 2 || e.shiftKey;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
        }});

        window.addEventListener('mouseup', () => {{ isDragging = false; }});
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());

        canvas.addEventListener('mousemove', (e) => {{
            if (!isDragging) return;
            const dx = e.clientX - lastMouseX;
            const dy = e.clientY - lastMouseY;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;

            if (isPanning) {{
                const panSpeed = distance * 0.0015;
                target[0] -= dx * panSpeed * Math.cos(yaw);
                target[1] += dy * panSpeed;
                target[2] += dx * panSpeed * Math.sin(yaw);
            }} else {{
                yaw -= dx * 0.006;
                pitch += dy * 0.006;
                const limit = Math.PI / 2 - 0.05;
                pitch = Math.max(-limit, Math.min(limit, pitch));
            }}
        }});

        canvas.addEventListener('wheel', (e) => {{
            e.preventDefault();
            const zoomFactor = e.deltaY > 0 ? 1.1 : 0.9;
            distance = Math.max(fitRadius * 0.1, Math.min(fitRadius * 15.0, distance * zoomFactor));
        }}, {{ passive: false }});

        let touchStartDist = 0;
        canvas.addEventListener('touchstart', (e) => {{
            if (e.touches.length === 1) {{
                isDragging = true;
                lastMouseX = e.touches[0].clientX;
                lastMouseY = e.touches[0].clientY;
            }} else if (e.touches.length === 2) {{
                touchStartDist = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
            }}
        }});
        canvas.addEventListener('touchmove', (e) => {{
            if (e.touches.length === 1 && isDragging) {{
                const dx = e.touches[0].clientX - lastMouseX;
                const dy = e.touches[0].clientY - lastMouseY;
                lastMouseX = e.touches[0].clientX;
                lastMouseY = e.touches[0].clientY;
                yaw -= dx * 0.006;
                pitch += dy * 0.006;
            }} else if (e.touches.length === 2) {{
                const dist = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                if (touchStartDist > 0) {{
                    const factor = touchStartDist / dist;
                    distance *= factor;
                    touchStartDist = dist;
                }}
            }}
        }});
        canvas.addEventListener('touchend', () => {{ isDragging = false; }});

        function setPresetView(name) {{
            target = [...modelCenter];
            distance = fitRadius * 2.5;
            if (name === 'iso') {{ yaw = Math.PI / 4; pitch = Math.PI / 6; }}
            else if (name === 'front') {{ yaw = 0; pitch = 0; }}
            else if (name === 'top') {{ yaw = 0; pitch = Math.PI / 2 - 0.01; }}
            else if (name === 'right') {{ yaw = Math.PI / 2; pitch = 0; }}
        }}

        function resetView() {{
            setPresetView('iso');
        }}

        requestAnimationFrame(render);
    </script>
</body>
</html>"#,
        title = title,
        format_badge = format_badge,
        num_triangles = num_triangles,
        num_vertices = num_vertices,
        unique_geometries = stats.unique_geometries,
        pos_b64 = pos_b64,
        norm_b64 = norm_b64,
        idx_b64 = idx_b64,
        center_x = center[0],
        center_y = center[1],
        center_z = center[2],
        fit_radius = fit_radius,
    );

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Bounds, Geometry, Instance, Vertex};

    #[test]
    fn test_generate_html_viewer_embedding() {
        let geom = Geometry {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
            ],
            source_attributes: None,
            indices: vec![0, 1, 2],
            bounds: Bounds {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0],
            },
            bounding_center: [0.5, 0.5, 0.0],
            bounding_radius: 1.0,
        };

        let inst = Instance {
            geometry: 0,
            material: 0,
            transform: glam::Mat4::IDENTITY,
            normal_transform: glam::Mat3::IDENTITY,
            node_name: None,
        };

        let scene = CompiledScene {
            source_hash: "test".to_string(),
            geometries: vec![geom],
            instances: vec![inst],
            materials: vec![],
            textures: vec![],
            bounds: Bounds {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0],
            },
            fit_radius: 1.0,
            statistics: SceneStatistics {
                nodes: 1,
                mesh_primitives: 1,
                unique_geometries: 1,
                instances: 1,
                vertices: 3,
                triangles: 1,
                duplicate_geometries_removed: 0,
                materials: 0,
                textures: 0,
                bounds: Bounds {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 0.0],
                },
            },
        };

        let stats = scene.statistics.clone();
        let html =
            generate_html_viewer(&scene, "test_part.step", &stats).expect("should generate html");

        assert!(html.contains("test_part.step"));
        assert!(html.contains("STEP B-REP"));
        assert!(html.contains("<canvas id=\"gl-canvas\">"));
        assert!(html.contains("Left Drag"));
        assert!(html.contains("Scroll Wheel"));
    }
}
