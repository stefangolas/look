#!/usr/bin/env python3
"""Merge external-buffer glTF scenes into one self-contained GLB.

This deliberately small benchmark utility supports the glTF features used by
Intel's Sponza base, Ivy, and Trees packages. It does not attempt to replace a
general glTF conversion tool.
"""

from __future__ import annotations

import argparse
import copy
import json
import mimetypes
import struct
from pathlib import Path


def align(blob: bytearray) -> None:
    blob.extend(b"\0" * (-len(blob) % 4))


def load_uri(base: Path, uri: str) -> bytes:
    if uri.startswith("data:"):
        raise ValueError("data URIs are not supported")
    return (base / uri).resolve().read_bytes()


def image_bytes(document: dict, buffers: list[bytes], base: Path, image: dict) -> tuple[bytes, str]:
    if "uri" in image:
        uri = image["uri"]
        mime = image.get("mimeType") or mimetypes.guess_type(uri)[0]
        if mime not in {"image/png", "image/jpeg"}:
            raise ValueError(f"unsupported image type for {uri!r}: {mime!r}")
        return load_uri(base, uri), mime
    view = document["bufferViews"][image["bufferView"]]
    start = view.get("byteOffset", 0)
    end = start + view["byteLength"]
    return buffers[view["buffer"]][start:end], image["mimeType"]


def offset_texture_references(material: dict, texture_offset: int) -> None:
    pbr = material.get("pbrMetallicRoughness", {})
    references = [
        pbr.get("baseColorTexture"),
        pbr.get("metallicRoughnessTexture"),
        material.get("normalTexture"),
        material.get("occlusionTexture"),
        material.get("emissiveTexture"),
    ]
    for reference in references:
        if reference is not None:
            reference["index"] += texture_offset


def merge(inputs: list[Path]) -> tuple[dict, bytes]:
    output: dict = {
        "asset": {
            "version": "2.0",
            "generator": "look benchmarks/merge-gltf-scenes.py",
        },
        "scene": 0,
        "scenes": [{"name": "Merged benchmark scene", "nodes": []}],
        "nodes": [],
        "meshes": [],
        "accessors": [],
        "bufferViews": [],
        "materials": [],
        "textures": [],
        "images": [],
        "samplers": [],
    }
    binary = bytearray()

    for source in inputs:
        document = json.loads(source.read_text(encoding="utf-8-sig"))
        base = source.parent
        source_buffers = [load_uri(base, buffer["uri"]) for buffer in document.get("buffers", [])]
        buffer_bases = []
        for source_buffer in source_buffers:
            align(binary)
            buffer_bases.append(len(binary))
            binary.extend(source_buffer)

        offsets = {
            key: len(output[key])
            for key in ("nodes", "meshes", "accessors", "bufferViews", "materials", "textures", "images", "samplers")
        }

        for view in document.get("bufferViews", []):
            merged = copy.deepcopy(view)
            merged["byteOffset"] = buffer_bases[view["buffer"]] + view.get("byteOffset", 0)
            merged["buffer"] = 0
            output["bufferViews"].append(merged)

        for sampler in document.get("samplers", []):
            output["samplers"].append(copy.deepcopy(sampler))

        for image in document.get("images", []):
            encoded, mime = image_bytes(document, source_buffers, base, image)
            align(binary)
            view_index = len(output["bufferViews"])
            output["bufferViews"].append(
                {"buffer": 0, "byteOffset": len(binary), "byteLength": len(encoded)}
            )
            binary.extend(encoded)
            merged = {"bufferView": view_index, "mimeType": mime}
            if "name" in image:
                merged["name"] = image["name"]
            output["images"].append(merged)

        for texture in document.get("textures", []):
            merged = copy.deepcopy(texture)
            if "sampler" in merged:
                merged["sampler"] += offsets["samplers"]
            if "source" in merged:
                merged["source"] += offsets["images"]
            output["textures"].append(merged)

        for material in document.get("materials", []):
            merged = copy.deepcopy(material)
            offset_texture_references(merged, offsets["textures"])
            output["materials"].append(merged)

        for accessor in document.get("accessors", []):
            merged = copy.deepcopy(accessor)
            if "bufferView" in merged:
                merged["bufferView"] += offsets["bufferViews"]
            output["accessors"].append(merged)

        for mesh in document.get("meshes", []):
            merged = copy.deepcopy(mesh)
            for primitive in merged.get("primitives", []):
                primitive["attributes"] = {
                    semantic: accessor + offsets["accessors"]
                    for semantic, accessor in primitive.get("attributes", {}).items()
                }
                if "indices" in primitive:
                    primitive["indices"] += offsets["accessors"]
                if "material" in primitive:
                    primitive["material"] += offsets["materials"]
                for target in primitive.get("targets", []):
                    for semantic in target:
                        target[semantic] += offsets["accessors"]
            output["meshes"].append(merged)

        for node in document.get("nodes", []):
            merged = copy.deepcopy(node)
            if "mesh" in merged:
                merged["mesh"] += offsets["meshes"]
            if "children" in merged:
                merged["children"] = [child + offsets["nodes"] for child in merged["children"]]
            # Cameras and package lights are irrelevant to the renderer
            # comparison and can reference top-level objects we intentionally
            # do not merge.
            merged.pop("camera", None)
            merged.pop("extensions", None)
            output["nodes"].append(merged)

        scene = document.get("scenes", [{}])[document.get("scene", 0)]
        output["scenes"][0]["nodes"].extend(
            node + offsets["nodes"] for node in scene.get("nodes", [])
        )

    align(binary)
    output["buffers"] = [{"byteLength": len(binary)}]
    for key in ("nodes", "meshes", "accessors", "bufferViews", "materials", "textures", "images", "samplers"):
        if not output[key]:
            output.pop(key)
    return output, bytes(binary)


def write_glb(path: Path, document: dict, binary: bytes) -> None:
    encoded_json = json.dumps(document, separators=(",", ":")).encode()
    encoded_json += b" " * (-len(encoded_json) % 4)
    binary += b"\0" * (-len(binary) % 4)
    total = 12 + 8 + len(encoded_json) + 8 + len(binary)
    if total > 0xFFFF_FFFF:
        raise ValueError("merged GLB exceeds the 4 GiB format limit")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as output:
        output.write(struct.pack("<III", 0x46546C67, 2, total))
        output.write(struct.pack("<II", len(encoded_json), 0x4E4F534A))
        output.write(encoded_json)
        output.write(struct.pack("<II", len(binary), 0x004E4942))
        output.write(binary)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("inputs", nargs="+", type=Path)
    args = parser.parse_args()
    document, binary = merge([path.resolve() for path in args.inputs])
    write_glb(args.output, document, binary)
    print(
        json.dumps(
            {
                "output": str(args.output),
                "bytes": args.output.stat().st_size,
                "inputs": [str(path) for path in args.inputs],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
