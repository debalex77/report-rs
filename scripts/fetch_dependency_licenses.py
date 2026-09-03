#!/usr/bin/env python3
"""Fetch missing upstream license evidence at Cargo's published VCS revision.

Maintainer tool; review its output before including it in a release.
Uses only GitHub HTTPS endpoints, never executes downloaded content.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
from urllib.request import Request, urlopen
from urllib.parse import urlparse, quote


def download(url):
    with urlopen(Request(url, headers={"User-Agent": "report-rs-license-audit"}), timeout=30) as response:
        return response.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output must be a new directory")
    inventory = json.loads(args.inventory.read_text())
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
        "--filter-platform", inventory["target"]]))
    needed = set(inventory["needs_review"])
    groups = {}
    unresolved = []
    for package in metadata["packages"]:
        key = package["name"] + "-" + package["version"]
        if key not in needed:
            continue
        root = Path(package["manifest_path"]).parent
        try:
            vcs = json.loads((root / ".cargo_vcs_info.json").read_text())
            sha = vcs["git"]["sha1"]
        except (OSError, ValueError, KeyError):
            unresolved.append(key)
            continue
        repo_url = package.get("repository") or ""
        # zune-core omits repository; verify its manifest at the recorded commit
        # in the sibling zune-jpeg repository before accepting this mapping.
        if package["name"] == "zune-core" and package["version"] == "0.4.12":
            repo_url = "https://github.com/etemesi254/zune-image"
        url = urlparse(repo_url)
        parts = url.path.strip("/").split("/")
        if url.hostname != "github.com" or len(parts) < 2:
            unresolved.append(key)
            continue
        repo = "/".join(parts[:2]).removesuffix(".git")
        groups.setdefault((repo, sha), []).append((key, vcs.get("path_in_vcs", ""), package["license"]))
    args.output.mkdir(parents=True)

    def fetch(group):
        (repo, sha), packages = group
        try:
            tree = json.loads(download(f"https://api.github.com/repos/{repo}/git/trees/{sha}?recursive=1"))
            if tree.get("truncated"):
                raise ValueError("truncated tree")
            results = {}
            for key, crate_path, license_expression in packages:
                if key == "zune-core-0.4.12":
                    local = next(p for p in metadata["packages"] if p["name"] == "zune-core" and p["version"] == "0.4.12")
                    local_manifest = (Path(local["manifest_path"]).parent / "Cargo.toml.orig").read_bytes()
                    remote_manifest = download(f"https://raw.githubusercontent.com/{repo}/{sha}/{crate_path}/Cargo.toml")
                    if local_manifest != remote_manifest:
                        raise ValueError("zune-core manifest does not match proposed upstream")
                parents = {str(p) for p in PurePosixPath(crate_path).parents} | {crate_path, ".", ""}
                candidates = []
                for entry in tree["tree"]:
                    path = PurePosixPath(entry["path"])
                    if entry["type"] != "blob":
                        continue
                    license_directory = path.parent.name.lower() in ("license", "licenses", "licence", "licences") and str(path.parent.parent) in parents
                    if (str(path.parent) in parents or license_directory) and path.name.lower().startswith(("license", "licence", "copying", "notice", "copyright")):
                        candidates.append(entry["path"])
                evidence = []
                for path in candidates:
                    url = f"https://raw.githubusercontent.com/{repo}/{sha}/{quote(path)}"
                    data = download(url)
                    destination = args.output / key / path
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(data)
                    evidence.append({"file": str(destination.relative_to(args.output)),
                                     "url": url, "sha256": hashlib.sha256(data).hexdigest()})
                results[key] = {"revision": sha, "repository": repo, "license": license_expression,
                                "files": evidence}
            return results
        except Exception as error:
            print(f"Failed {repo}@{sha}: {error}", flush=True)
            return {key: {"files": []} for key, _, _ in packages}

    results = {}
    with ThreadPoolExecutor(max_workers=4) as pool:
        for result in pool.map(fetch, groups.items()):
            results.update(result)
    unresolved.extend(key for key, value in results.items() if not value["files"])
    (args.output / "index.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"Retrieved evidence for {sum(bool(x['files']) for x in results.values())} packages.")
    print("Unresolved: " + ", ".join(sorted(unresolved)))
    if unresolved:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
