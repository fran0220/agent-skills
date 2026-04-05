#!/usr/bin/env python3
"""Headless Blender sprite renderer for animated GLB/GLTF models."""

import json
import math
import os
import sys

import bpy
import mathutils


ANGLE_PRESETS = {
    "front": (10.0, 0.0),
    "side": (10.0, 90.0),
    "top": (90.0, 0.0),
    "iso": (35.264, 45.0),
    "3/4": (25.0, 35.0),
}


def parse_args():
    if "--" not in sys.argv:
        raise SystemExit("expected Blender arguments after --")

    argv = sys.argv[sys.argv.index("--") + 1 :]
    if len(argv) < 4:
        raise SystemExit(
            "usage: blender --background --python render_sprites.py -- <input.glb> <output_dir> <frame_count> <resolution> [camera_angle] [directions]"
        )

    return {
        "input_path": argv[0],
        "output_dir": argv[1],
        "frame_count": int(argv[2]),
        "resolution": int(argv[3]),
        "camera_angle": argv[4] if len(argv) > 4 else "front",
        "directions": int(argv[5]) if len(argv) > 5 else 1,
    }


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_model(input_path):
    lower = input_path.lower()
    if lower.endswith((".glb", ".gltf")):
        bpy.ops.import_scene.gltf(filepath=input_path)
        return
    raise RuntimeError(f"unsupported model format for sprite rendering: {input_path}")


def set_render_engine(scene):
    for engine in ("BLENDER_EEVEE_NEXT", "BLENDER_EEVEE"):
        try:
            scene.render.engine = engine
            return engine
        except Exception:
            continue
    raise RuntimeError("Blender EEVEE is not available in this Blender build")


def configure_render(scene, resolution):
    engine = set_render_engine(scene)
    scene.render.resolution_x = resolution
    scene.render.resolution_y = resolution
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.compression = 15

    if hasattr(scene, "eevee"):
        scene.eevee.taa_render_samples = 32
        if hasattr(scene.eevee, "use_gtao"):
            scene.eevee.use_gtao = True
        if hasattr(scene.eevee, "use_bloom"):
            scene.eevee.use_bloom = False

    world = scene.world or bpy.data.worlds.new("SpriteWorld")
    scene.world = world
    world.use_nodes = True
    background = world.node_tree.nodes.get("Background")
    if background is not None:
        background.inputs[0].default_value = (1.0, 1.0, 1.0, 1.0)
        background.inputs[1].default_value = 0.3

    return engine


def add_lights(scene, center, radius):
    lights = [
        ((radius * 2.0, -radius * 2.5, radius * 3.0), 3.0),
        ((-radius * 1.5, radius * 2.0, radius * 2.0), 1.6),
    ]
    for index, (location, energy) in enumerate(lights):
        light_data = bpy.data.lights.new(f"SpriteSun{index}", type="SUN")
        light_data.energy = energy
        light_obj = bpy.data.objects.new(f"SpriteSun{index}", light_data)
        light_obj.location = location
        scene.collection.objects.link(light_obj)
        direction = mathutils.Vector(center) - light_obj.location
        light_obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def mesh_objects():
    return [obj for obj in bpy.data.objects if obj.type == "MESH"]


def compute_bounds(objects):
    min_co = [float("inf"), float("inf"), float("inf")]
    max_co = [float("-inf"), float("-inf"), float("-inf")]

    for obj in objects:
        for corner in obj.bound_box:
            world_corner = obj.matrix_world @ mathutils.Vector(corner)
            min_co[0] = min(min_co[0], world_corner.x)
            min_co[1] = min(min_co[1], world_corner.y)
            min_co[2] = min(min_co[2], world_corner.z)
            max_co[0] = max(max_co[0], world_corner.x)
            max_co[1] = max(max_co[1], world_corner.y)
            max_co[2] = max(max_co[2], world_corner.z)

    min_vec = mathutils.Vector(min_co)
    max_vec = mathutils.Vector(max_co)
    center = (min_vec + max_vec) / 2.0
    size = max_vec - min_vec
    radius = max(size.x, size.y, size.z, 0.001)
    return min_vec, max_vec, center, size, radius


def build_camera(scene, center, radius):
    cam_data = bpy.data.cameras.new("SpriteCamera")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = radius * 2.2
    cam_data.clip_start = 0.01
    cam_data.clip_end = radius * 20.0
    cam_obj = bpy.data.objects.new("SpriteCamera", cam_data)
    scene.collection.objects.link(cam_obj)
    scene.camera = cam_obj
    add_lights(scene, center, radius)
    return cam_obj, cam_data


def animation_range(scene):
    starts = []
    ends = []
    for action in bpy.data.actions:
        if action.fcurves:
            start, end = action.frame_range
            starts.append(int(math.floor(start)))
            ends.append(int(math.ceil(end)))

    if starts and ends:
        return min(starts), max(ends)

    return int(scene.frame_start), int(scene.frame_end)


def sampled_frames(frame_start, frame_end, frame_count):
    span = max(frame_end - frame_start, 0)
    if frame_count <= 1:
        return [frame_start]
    if span == 0:
        return [frame_start for _ in range(frame_count)]

    samples = []
    for index in range(frame_count):
        t = index / float(frame_count - 1)
        frame = int(round(frame_start + span * t))
        samples.append(frame)
    return samples


def place_camera(camera_obj, center, radius, elevation_deg, azimuth_deg):
    elev_rad = math.radians(elevation_deg)
    azim_rad = math.radians(azimuth_deg)
    distance = radius * 3.2

    cam_x = center.x + distance * math.cos(elev_rad) * math.sin(azim_rad)
    cam_y = center.y - distance * math.cos(elev_rad) * math.cos(azim_rad)
    cam_z = center.z + distance * math.sin(elev_rad)
    camera_obj.location = (cam_x, cam_y, cam_z)

    direction = center - camera_obj.location
    camera_obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def render_frames(scene, camera_obj, output_dir, frame_numbers, camera_angle, directions, center, radius):
    base_elev, base_azim = ANGLE_PRESETS.get(camera_angle, ANGLE_PRESETS["front"])
    direction_step = 360.0 / float(directions)
    files = []

    for direction_index in range(directions):
        azimuth = base_azim + direction_index * direction_step
        place_camera(camera_obj, center, radius, base_elev, azimuth)

        for frame_index, source_frame in enumerate(frame_numbers):
            scene.frame_set(source_frame)
            filename = f"dir{direction_index:02d}_frame{frame_index:04d}.png"
            scene.render.filepath = os.path.join(output_dir, filename)
            bpy.ops.render.render(write_still=True)
            files.append(
                {
                    "filename": filename,
                    "direction": direction_index,
                    "frame_index": frame_index,
                    "source_frame": source_frame,
                }
            )

    return files


def main():
    args = parse_args()
    os.makedirs(args["output_dir"], exist_ok=True)

    reset_scene()
    import_model(args["input_path"])

    scene = bpy.context.scene
    engine = configure_render(scene, args["resolution"])

    objects = mesh_objects()
    if not objects:
        raise RuntimeError("imported scene does not contain any mesh objects")

    min_vec, max_vec, center, size, radius = compute_bounds(objects)
    camera_obj, camera_data = build_camera(scene, center, radius)

    frame_start, frame_end = animation_range(scene)
    frame_numbers = sampled_frames(frame_start, frame_end, args["frame_count"])
    files = render_frames(
        scene,
        camera_obj,
        args["output_dir"],
        frame_numbers,
        args["camera_angle"],
        args["directions"],
        center,
        radius,
    )

    metadata = {
        "frame_count": len(frame_numbers),
        "directions": args["directions"],
        "resolution": args["resolution"],
        "camera_angle": args["camera_angle"],
        "render_engine": engine,
        "ortho_scale": camera_data.ortho_scale,
        "animation": {
            "frame_start": frame_start,
            "frame_end": frame_end,
            "sampled_frames": frame_numbers,
        },
        "bounds": {
            "min": [min_vec.x, min_vec.y, min_vec.z],
            "max": [max_vec.x, max_vec.y, max_vec.z],
            "size": [size.x, size.y, size.z],
            "center": [center.x, center.y, center.z],
        },
        "files": files,
        "total_frames": len(files),
    }

    metadata_path = os.path.join(args["output_dir"], "metadata.json")
    with open(metadata_path, "w", encoding="utf-8") as handle:
        json.dump(metadata, handle, indent=2)

    print(f"RENDER_COMPLETE: {len(files)} frames rendered to {args['output_dir']}")


if __name__ == "__main__":
    main()
