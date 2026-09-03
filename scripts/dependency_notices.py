#!/usr/bin/env python3
"""Collect Cargo license evidence for a release; not a legal compliance verdict."""
import argparse
import json
import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile


def verify_declared_license(package, review, standard_root):
    """Validate a version-specific, reviewed declaration; never infer a license."""
    if not review:
        return None
    manifest = Path(package["manifest_path"])
    if package["license"] != review["declared"] or hashlib.sha256(manifest.read_bytes()).hexdigest() != review["manifest_sha256"]:
        raise ValueError(f"reviewed license declaration changed: {package['name']}")
    selected = review["selected"]
    alternatives = [part.strip() for part in review["declared"].replace("/", " OR ").split(" OR ")]
    if selected not in alternatives:
        raise ValueError("selected license is not a declared alternative")
    index = json.loads((standard_root / "index.json").read_text())
    if selected not in index:
        raise ValueError("unknown standard license")
    source = standard_root / (selected + ".txt")
    if hashlib.sha256(source.read_bytes()).hexdigest() != index[selected]["sha256"]:
        raise ValueError("standard license checksum mismatch")
    return {"selected": selected, "source_url": index[selected]["url"],
            "manifest_sha256": review["manifest_sha256"], "method": "published-declaration-with-standard-text"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--target", default="x86_64-unknown-linux-gnu")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    command = ["cargo", "metadata", "--locked", "--format-version", "1",
               "--filter-platform", args.target]
    if args.offline:
        command.append("--offline")
    metadata = json.loads(subprocess.check_output(command))
    if args.output.exists():
        parser.error("output must be a new directory to avoid stale license files")
    args.output.mkdir(parents=True)
    members = set(metadata["workspace_members"])
    packages = sorted((p for p in metadata["packages"] if p["id"] not in members),
                      key=lambda p: (p["name"], p["version"]))
    records = []
    missing = []
    supplemental_root = Path(__file__).resolve().parent.parent / "LICENSES" / "dependencies"
    supplemental_index = supplemental_root / "index.json"
    supplemental = json.loads(supplemental_index.read_text()) if supplemental_index.is_file() else {}
    standard_root = supplemental_root.parent / "standard"
    reviews_path = standard_root / "reviewed-declarations.json"
    reviews = json.loads(reviews_path.read_text()) if reviews_path.exists() else {}
    # Workspace subcrates sometimes omit the license shipped by their root
    # crate. Reuse it only when repository AND published VCS commit match.
    def vcs_identity(package):
        root = Path(package["manifest_path"]).parent
        try:
            sha = json.loads((root / ".cargo_vcs_info.json").read_text())["git"]["sha1"]
        except (OSError, KeyError, ValueError):
            return None
        repo = package.get("repository")
        return (repo.rstrip("/").removesuffix(".git"), sha) if repo else None

    siblings = {}
    for package in packages:
        identity = vcs_identity(package)
        if identity:
            siblings.setdefault(identity, []).append(package)
    for package in packages:
        key = package["name"] + "-" + package["version"]
        root = Path(package["manifest_path"]).parent
        files = set()
        for path in root.rglob("*"):
            if path.is_file() and not path.is_symlink() and path.name.lower().startswith(
                    ("license", "licence", "copying", "notice", "copyright")):
                files.add(path)
        if package.get("license_file"):
            path = (root / package["license_file"]).resolve()
            if path.is_relative_to(root.resolve()) and path.is_file():
                files.add(path)
        inherited_files = []
        if not files:
            for sibling in siblings.get(vcs_identity(package), []):
                if sibling["license"] != package["license"]:
                    continue
                sibling_root = Path(sibling["manifest_path"]).parent
                for path in sibling_root.iterdir():
                    if path.is_file() and not path.is_symlink() and path.name.lower().startswith(
                            ("license", "licence", "copying", "notice", "copyright")):
                        destination = args.output / "texts" / key / "same-revision" / sibling["name"] / path.name
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(path, destination)
                        inherited_files.append({"package": sibling["name"], "version": sibling["version"],
                                                "file": path.name, "revision": vcs_identity(package)[1]})
        for path in sorted(files):
            destination = args.output / "texts" / key / path.relative_to(root)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
        license_expression = package.get("license") or "UNDECLARED"
        upstream_files = []
        evidence = supplemental.get(key, {})
        identity = vcs_identity(package)
        revision_matches = identity and evidence.get("revision") == identity[1]
        # zune-core's omitted repository was independently checked by comparing
        # Cargo.toml.orig with the upstream manifest at the recorded SHA.
        if key == "zune-core-0.4.12":
            revision_matches = evidence.get("revision") == "f8fbb123d5ed04441e8324a555bfcda0cb1bd28f"
        if revision_matches and evidence.get("license") == license_expression:
            for entry in evidence.get("files", []):
                source = (supplemental_root / entry["file"]).resolve()
                if not source.is_relative_to(supplemental_root.resolve()):
                    raise ValueError("unsafe supplemental license path")
                data = source.read_bytes()
                if hashlib.sha256(data).hexdigest() != entry["sha256"]:
                    raise ValueError(f"supplemental license checksum mismatch: {key}")
                destination = args.output / "texts" / key / "upstream" / source.name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
                upstream_files.append(entry)
        # These retrieved COPYING files are pointers, not complete licenses.
        upstream_complete = bool(upstream_files) and package["name"] not in {
            "gpu-alloc", "gpu-alloc-types", "gpu-descriptor", "gpu-descriptor-types"}
        declaration = verify_declared_license(package, reviews.get(key), standard_root)
        if declaration:
            destination = args.output / "texts" / key / "declared-license"
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(standard_root / (declaration["selected"] + ".txt"), destination / "LICENSE-STANDARD.txt")
            shutil.copyfile(standard_root / "README.md", destination / "METHOD.md")
            shutil.copyfile(package["manifest_path"], destination / "Cargo.toml")
            (destination / "declaration.json").write_text(json.dumps({**declaration, "authors": package["authors"]}, indent=2) + "\n")
        source_archive = None
        # Include the exact cached crate source for file-level copyleft packages.
        if "MPL-2.0" in license_expression or declaration:
            source_archive = "sources/" + key + ".tar.gz"
            destination = args.output / source_archive
            destination.parent.mkdir(parents=True, exist_ok=True)
            with tarfile.open(destination, "w:gz") as archive:
                archive.add(root, arcname=key)
        if not (files or inherited_files or upstream_complete or declaration) or license_expression == "UNDECLARED":
            missing.append(key)
        records.append({"name": package["name"], "version": package["version"],
                        "license": license_expression, "source": package.get("source"),
                        "repository": package.get("repository"),
                        "license_files": [str(p.relative_to(root)) for p in sorted(files)],
                        "same_revision_license_files": inherited_files,
                        "upstream_license_files": upstream_files,
                        "reviewed_declaration": declaration,
                        "source_archive": source_archive})
    (args.output / "inventory.json").write_text(
        json.dumps({"target": args.target, "packages": records, "needs_review": missing},
                   indent=2) + "\n")
    lines = ["# Dependency notices", "",
             "Generated from locked Cargo metadata for `" + args.target + "`.", "",
             "This conservative inventory includes resolved build/test dependencies, not just",
             "code linked into the binaries. License texts and upstream notices are in `texts/`.",
             "Exact cached MPL-2.0 crate sources are in `sources/`.",
             "Thirteen version-pinned exceptions use published declarations plus SPDX standard",
             "texts, retaining full crate sources and supplied author metadata. These are marked",
             "`reviewed_declaration` in the inventory; see each `declared-license/METHOD.md`.",
             "Declared expressions are preserved, including alternative licenses.",
             "This inventory does not replace a review of native/system dependencies.", "",
             "| Package | Version | Declared license |", "| --- | --- | --- |"]
    lines += [f"| {p['name']} | {p['version']} | {p['license']} |" for p in records]
    (args.output / "README.md").write_text("\n".join(lines) + "\n")
    print(f"Collected {len(records)} packages; {len(missing)} missing collection evidence (not a legal compliance verdict).")
    if missing:
        print("Needs review: " + ", ".join(missing))
        raise SystemExit(1)


if __name__ == "__main__":
    main()
